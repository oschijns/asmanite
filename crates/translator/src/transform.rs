//! First draft of the AST → IR translation.
//!
//! The translation runs in three passes:
//!
//! 1. **Collection**: walk the module tree and register every declaration
//!    (module, variable, function, type) in a [`SymbolTree`].
//! 2. **Global data**: evaluate constants, reserve memory for static variables
//!    and lower their initializers into the startup routine ([`IrProgram::init`]).
//! 3. **Function lowering**: translate each function body into a flat list of
//!    [`OpCode`]s. Structured control flow (`if`, loops, `switch`) is turned into
//!    labels, jumps and conditional branches. Inline functions are expanded at
//!    each call site and produce no code of their own. The initializers of the statics
//!    declared in functions are appended to the startup routine, after the global
//!    ones, as they may depend on them.
//!
//! The language is a high-level assembly: the user decides where every variable
//! lives and which registers perform each operation. The translator follows that:
//! - Placements are never chosen: every `static`, `var` and non-inline function
//!   must be declared `in bank` or `at address`, or be part of an `in bank { ... }`
//!   block.
//! - There are no temporaries: every expression maps onto operands the user wrote.
//!   An expression is either an operand (constant, register, variable, array cell,
//!   assignment or call, whose value is their destination) or a single operation
//!   written into a destination (`a = x + 1`). Anything more complex must be split
//!   into several instructions by the user.
//! - Operations are emitted as written. Multi-byte operations are written byte by
//!   byte by the user (low bytes, then high bytes with the carry), the translator
//!   never splits them.
//! - Only `const` values are computed by the compiler. Everything else, including
//!   the initial value of a `static`, is computed at runtime by the target.
//!   Checking that each operation is encodable on the target CPU (e.g. storing an
//!   immediate value in memory requires a register on the 6502) is the job of the
//!   backend.
//!
//! Limitations of this draft (reported as [`TransformError::Unsupported`] or ignored):
//! - `use` statements are ignored: names resolve by walking up enclosing modules.
//! - Only primitive and array types have a size (no struct/union/enum layouts yet).
//! - Generics, `defer`, `inline for`, field accesses and pointers are not handled.

use asmanite_ast::{
    lexer::atom::{Range, VarType},
    parser::{
        composite_type::{TypeDecl, TypeUsed},
        expression::{Expr, FunctionCall},
        function::FuncDecl,
        function::FuncMarker,
        memory::{
            BankElement, MemStorage, ParamDecl, ParamStorage, ParamType, ReturnType, VarDecl,
        },
        module::{ModElement, Module},
        statement::{IfElse, Loop, LoopCtrlType, LoopType, Statement, StmtInner, Switch},
    },
};
use asmanite_common::num_traits::ToPrimitive;
use asmanite_common::{
    ToBox,
    ident::Ident,
    operator::{OpType, Operator, op},
    path::Path,
    span::{Span, Spanned},
    symbol_tree::{NodeId, ROOT, SymbolData, SymbolTree, SymbolType},
};
use asmanite_ir::inter::{
    Address, Int, IrData, IrFunc, IrProgram, IrStorage,
    opcode::{
        Condition, JumpTarget, Label, LabelId, OpBinary, OpBranch, OpCode, OpCompare, OpTransfer,
        OpUnary, Operand,
    },
};
use std::collections::{HashMap, HashSet};

// MARK: Errors

/// Error raised while translating the AST into the IR
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TransformError {
    #[error("`{ident}` is defined multiple times in the same module")]
    Duplicate { ident: Ident, span: Span },

    #[error("cannot find `{path}`")]
    Unresolved { path: Path, span: Span },

    #[error("`{path}` is not a {expected}")]
    WrongKind {
        path: Path,
        expected: &'static str,
        span: Span,
    },

    #[error("expression is not a compile-time constant")]
    NotConstant { span: Span },

    #[error("constant `{path}` depends on itself")]
    CyclicConstant { path: Path, span: Span },

    #[error("unknown label `{label}`")]
    UnknownLabel { label: Ident, span: Span },

    #[error("`break` or `continue` outside of a loop")]
    OutsideLoop { span: Span },

    #[error("this expression cannot be assigned to")]
    NotAssignable { span: Span },

    #[error("expected {expected} arguments, found {found}")]
    ArgCount {
        expected: usize,
        found: usize,
        span: Span,
    },

    #[error("function `{path}` does not return a value")]
    NoReturnValue { path: Path, span: Span },

    #[error("`{ident}` must be placed in a bank (`in bank`) or at an address (`at address`)")]
    MissingStorage { ident: Ident, span: Span },

    #[error("`{var_type}` variables can only be declared inside a function")]
    NotInModule { var_type: &'static str, span: Span },

    #[error("this expression does not map onto an operand, split it into several instructions")]
    NotAnOperand { span: Span },

    #[error("this expression has no effect")]
    NoEffect { span: Span },

    #[error(
        "`{ident}` does not exist at startup: a static can only be initialized from constants, statics and registers"
    )]
    NotAtStartup { ident: Ident, span: Span },

    #[error("an inline function is expanded at each call site, it cannot be placed in memory")]
    InlinePlacement { span: Span },

    #[error("inline function `{path}` cannot be expanded inside itself")]
    RecursiveInline { path: Path, span: Span },

    #[error("{what} is not supported yet")]
    Unsupported { what: &'static str, span: Span },
}

type Result<T> = core::result::Result<T, TransformError>;

/// Shorthand to report an unsupported feature
#[inline]
fn unsupported<T>(what: &'static str, span: Span) -> Result<T> {
    Err(TransformError::Unsupported { what, span })
}

// MARK: Entry point

/// Translate the root module of a project into the IR.
///
/// All `mod name;` declarations must already have been replaced by the module
/// they refer to. Every error encountered is reported, not only the first one.
pub fn transform(root: &Module) -> core::result::Result<IrProgram, Vec<TransformError>> {
    let mut errors = Vec::new();

    // Pass 1: register every declaration
    let mut globals = Globals::new();
    let root_node = globals.tree.root();
    globals.collect(root, root_node, &[], &mut errors);

    // Pass 2: constants and static variables.
    // The initializers of the statics are computed at runtime, by the startup routine.
    let mut data = Vec::new();
    let mut init = FuncLowerer::startup(&globals, root_node);
    for &node in &globals.order {
        let symbol = &globals.symbols[&node];
        if let Decl::Var(var) = symbol.decl {
            match globals.global_data(symbol, var) {
                Ok(Some(item)) => {
                    data.push(item);
                    if let Some(value) = &var.assign_expr {
                        init.lower_static_init(StaticInit {
                            target: symbol.path.clone(),
                            value,
                            module: symbol.module,
                            scope: HashMap::new(),
                        });
                    }
                }
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
    }

    // Pass 3: function bodies
    let mut functions = Vec::new();
    let mut local_inits = Vec::new();
    for &node in &globals.order {
        let symbol = &globals.symbols[&node];
        let Decl::Func(func) = symbol.decl else {
            continue;
        };
        // Inline functions only exist where they are expanded
        if func.marker == FuncMarker::Inline {
            if func.storage != MemStorage::Undefined {
                errors.push(TransformError::InlinePlacement { span: func.span });
            }
            continue;
        }
        let lowerer = FuncLowerer::new(&globals, symbol.module, symbol.path.clone());
        match lowerer.lower(func, symbol.bank.as_ref()) {
            Ok(lowered) => {
                local_inits.extend(lowered.static_inits);
                push_new_data(&mut data, lowered.inline_data);
                functions.push(lowered.func);
            }
            Err(func_errors) => errors.extend(func_errors),
        }
    }

    // The statics declared in functions are initialized after the global ones
    for static_init in local_inits {
        init.lower_static_init(static_init);
    }
    let init = match init.finish_init() {
        Ok(lowered) => {
            push_new_data(&mut data, lowered.inline_data);
            Some(lowered.func)
        }
        Err(init_errors) => {
            errors.extend(init_errors);
            None
        }
    };

    // An error in an inline function is found again at each expansion
    let mut unique_errors: Vec<TransformError> = Vec::with_capacity(errors.len());
    for error in errors {
        if !unique_errors.contains(&error) {
            unique_errors.push(error);
        }
    }
    let errors = unique_errors;

    match init {
        Some(init) if errors.is_empty() => Ok(IrProgram {
            data,
            init,
            functions,
        }),
        _ => Err(errors),
    }
}

/// Reserve the memory cells of inline functions.
/// Each expansion declares the same cells, they are reserved only once.
fn push_new_data(data: &mut Vec<IrData>, cells: Vec<IrData>) {
    for cell in cells {
        if !data.iter().any(|known| known.name == cell.name) {
            data.push(cell);
        }
    }
}

// MARK: Symbols

/// Declaration a symbol refers to
#[derive(Debug, Clone, Copy)]
enum Decl<'a> {
    Module,
    Type,
    Var(&'a VarDecl),
    Func(&'a FuncDecl),
}

/// A symbol declared in a module
#[derive(Debug)]
struct Symbol<'a> {
    /// Absolute path of the symbol, used as its name in the IR
    path: Path,

    /// Module in which the symbol is declared (names in it resolve from there)
    module: NodeId,

    /// Bank of the enclosing `in bank { ... }` block, if any
    bank: Option<Ident>,

    /// The declaration itself
    decl: Decl<'a>,
}

/// Every symbol declared in the program
struct Globals<'a> {
    tree: SymbolTree,
    symbols: HashMap<NodeId, Symbol<'a>>,

    /// Declaration order, to keep the output deterministic
    order: Vec<NodeId>,
}

/// Lookup of local constants while evaluating a constant expression.
/// - `None`: the name is not a local
/// - `Some(None)`: the name is a local, but not a constant
/// - `Some(Some(value))`: the name is a local constant
type LocalConsts<'l> = &'l dyn Fn(&Path) -> Option<Option<Int>>;

/// Used when there is no local scope (global declarations)
fn no_locals(_: &Path) -> Option<Option<Int>> {
    None
}

impl<'a> Globals<'a> {
    fn new() -> Self {
        Self {
            tree: SymbolTree::new(),
            symbols: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Pass 1: register the elements of a module and of its sub-modules
    fn collect(
        &mut self,
        module: &'a Module,
        node: NodeId,
        prefix: &[Ident],
        errors: &mut Vec<TransformError>,
    ) {
        for element in module.elements.iter() {
            let declared = match element {
                ModElement::Mod(sub) => self
                    .declare(
                        node,
                        prefix,
                        &sub.ident,
                        SymbolType::Module,
                        Decl::Module,
                        sub.span,
                    )
                    .map(|child| {
                        let mut sub_prefix = prefix.to_vec();
                        sub_prefix.push(sub.ident.clone());
                        self.collect(sub, child, &sub_prefix, errors);
                    }),
                ModElement::ModDecl(decl) => unsupported(
                    "loading a module from another file (`mod name;`)",
                    decl.span,
                ),
                // TODO: resolve imports into per-module alias tables
                ModElement::Use(_) => Ok(()),
                ModElement::Type(ty) => {
                    let name = match ty {
                        TypeDecl::Composite(c) => c.name.as_ref(),
                        TypeDecl::Enum(e) => e.name.as_ref(),
                        TypeDecl::Alias(a) => Some(&a.name),
                    };
                    match name {
                        Some(name) => self
                            .declare(node, prefix, name, SymbolType::Type, Decl::Type, ty.span())
                            .map(drop),
                        None => Ok(()),
                    }
                }
                ModElement::Var(var) => self
                    .declare(
                        node,
                        prefix,
                        &var.ident,
                        SymbolType::Variable,
                        Decl::Var(var),
                        var.span,
                    )
                    .map(drop),
                ModElement::Func(func) => self
                    .declare(
                        node,
                        prefix,
                        &func.ident,
                        SymbolType::Function,
                        Decl::Func(func),
                        func.span,
                    )
                    .map(drop),
                // The elements of a bank block belong to the enclosing module
                ModElement::Bank(bank) => {
                    for element in bank.elements.iter() {
                        let declared = match element {
                            BankElement::Var(var) => self.declare(
                                node,
                                prefix,
                                &var.ident,
                                SymbolType::Variable,
                                Decl::Var(var),
                                var.span,
                            ),
                            BankElement::Func(func) => self.declare(
                                node,
                                prefix,
                                &func.ident,
                                SymbolType::Function,
                                Decl::Func(func),
                                func.span,
                            ),
                        };
                        match declared {
                            Ok(child) => {
                                if let Some(symbol) = self.symbols.get_mut(&child) {
                                    symbol.bank = Some(bank.ident.clone());
                                }
                            }
                            Err(error) => errors.push(error),
                        }
                    }
                    Ok(())
                }
            };
            if let Err(error) = declared {
                errors.push(error);
            }
        }
    }

    /// Insert a symbol in the tree, rejecting duplicates in the same module
    fn declare(
        &mut self,
        parent: NodeId,
        prefix: &[Ident],
        ident: &Ident,
        symbol_type: SymbolType,
        decl: Decl<'a>,
        span: Span,
    ) -> Result<NodeId> {
        if self
            .tree
            .find_from(parent, &Path::new(vec![ident.clone()]))
            .is_some()
        {
            return Err(TransformError::Duplicate {
                ident: ident.clone(),
                span,
            });
        }

        let node = self
            .tree
            .insert(parent, SymbolData::new(ident.clone(), symbol_type));
        let mut path = prefix.to_vec();
        path.push(ident.clone());
        self.symbols.insert(
            node,
            Symbol {
                path: Path::new(path),
                module: parent,
                bank: None,
                decl,
            },
        );
        self.order.push(node);
        Ok(node)
    }

    /// Find the symbol a path refers to, starting from a module
    fn resolve(&self, module: NodeId, path: &Path, span: Span) -> Result<&Symbol<'a>> {
        self.tree
            .resolve_path_from(module, path)
            .and_then(|node| self.symbols.get(&node))
            .ok_or_else(|| TransformError::Unresolved {
                path: path.clone(),
                span,
            })
    }

    /// Pass 2: memory to reserve for a variable declared in a module.
    /// Only `const` and `static` can be declared there.
    /// Constants do not need memory: they are replaced by their value.
    fn global_data(&self, symbol: &Symbol<'a>, var: &'a VarDecl) -> Result<Option<IrData>> {
        let module = symbol.module;
        match var.var_type {
            VarType::Const => {
                // Evaluate it now to report errors even if the constant is never used
                let value = var
                    .assign_expr
                    .as_ref()
                    .ok_or(TransformError::NotConstant { span: var.span })?;
                self.eval_const(value, module, &no_locals, &mut Vec::new())?;
                Ok(None)
            }
            VarType::Static => Ok(Some(IrData {
                name: symbol.path.clone(),
                size: self.size_of(&var.type_used, module, &no_locals)?,
                storage: self.var_storage(var, module, &no_locals, symbol.bank.as_ref())?,
            })),
            VarType::Var | VarType::Let | VarType::Stack => Err(TransformError::NotInModule {
                var_type: var.var_type.into(),
                span: var.span,
            }),
        }
    }

    // MARK: Constants

    /// Evaluate an expression at compile time
    fn eval_const(
        &self,
        expr: &Expr,
        module: NodeId,
        locals: LocalConsts,
        visiting: &mut Vec<NodeId>,
    ) -> Result<Int> {
        let not_const = || TransformError::NotConstant { span: expr.span() };

        match expr {
            Expr::Lit(lit) => lit.atom.to_i64().ok_or_else(not_const),
            Expr::Group(group) => self.eval_const(&group.atom, module, locals, visiting),
            // TODO: truncate the value to the size of the primitive
            Expr::Cast(cast) => self.eval_const(&cast.expr, module, locals, visiting),
            Expr::Var(var) => {
                if let Some(local) = locals(&var.atom) {
                    return local.ok_or_else(not_const);
                }
                let node = self
                    .tree
                    .resolve_path_from(module, &var.atom)
                    .ok_or_else(|| TransformError::Unresolved {
                        path: var.atom.clone(),
                        span: var.span,
                    })?;
                let symbol = &self.symbols[&node];
                let Decl::Var(decl) = symbol.decl else {
                    return Err(not_const());
                };
                if !matches!(decl.var_type, VarType::Const | VarType::Let) {
                    return Err(not_const());
                }
                let Some(value) = &decl.assign_expr else {
                    return Err(not_const());
                };
                if visiting.contains(&node) {
                    return Err(TransformError::CyclicConstant {
                        path: symbol.path.clone(),
                        span: var.span,
                    });
                }
                visiting.push(node);
                let result = self.eval_const(value, symbol.module, &no_locals, visiting);
                visiting.pop();
                result
            }
            Expr::Unary(unary) => {
                let value = self.eval_const(&unary.expr, module, locals, visiting)?;
                eval_unary(unary.op.op_type, value).ok_or_else(not_const)
            }
            Expr::Binary(binary) if !binary.op.carry => {
                let lhs = self.eval_const(&binary.lhs, module, locals, visiting)?;
                let rhs = self.eval_const(&binary.rhs, module, locals, visiting)?;
                eval_binary(binary.op.op_type, lhs, rhs).ok_or_else(not_const)
            }
            _ => Err(not_const()),
        }
    }

    /// Size in bytes of a type
    fn size_of(&self, ty: &TypeUsed, module: NodeId, locals: LocalConsts) -> Result<u32> {
        match ty {
            TypeUsed::Prim(prim) => Ok(u32::from(prim.atom.size)),
            TypeUsed::Array(array) => {
                let cell = self.size_of(&array.cell_type, module, locals)?;
                let count = self.eval_const(&array.size, module, locals, &mut Vec::new())?;
                u32::try_from(count)
                    .ok()
                    .and_then(|count| count.checked_mul(cell))
                    .ok_or(TransformError::NotConstant {
                        span: array.size.span(),
                    })
            }
            // TODO: struct/union/enum layouts, named types, tuples, pointers
            _ => unsupported("this kind of type", ty.span()),
        }
    }

    /// Placement of a variable or a function, `None` if not specified
    fn mem_storage(
        &self,
        storage: &MemStorage,
        module: NodeId,
        locals: LocalConsts,
    ) -> Result<Option<IrStorage>> {
        match storage {
            MemStorage::Undefined => Ok(None),
            MemStorage::In(bank) => Ok(Some(IrStorage::Bank(bank.clone()))),
            MemStorage::At(addr) => self
                .address(addr, module, locals)
                .map(|addr| Some(IrStorage::Address(addr))),
        }
    }

    /// Placement of a variable, which must be explicit:
    /// given in its declaration, or by the enclosing bank block.
    fn var_storage(
        &self,
        var: &VarDecl,
        module: NodeId,
        locals: LocalConsts,
        bank: Option<&Ident>,
    ) -> Result<IrStorage> {
        self.mem_storage(&var.storage, module, locals)?
            .or_else(|| bank.map(|bank| IrStorage::Bank(bank.clone())))
            .ok_or_else(|| TransformError::MissingStorage {
                ident: var.ident.clone(),
                span: var.span,
            })
    }

    /// Placement of a parameter or a return value
    fn param_storage(
        &self,
        storage: &ParamStorage,
        module: NodeId,
        span: Span,
    ) -> Result<IrStorage> {
        match storage {
            ParamStorage::In(bank) => Ok(IrStorage::Bank(bank.clone())),
            ParamStorage::At(addr) => self
                .address(addr, module, &no_locals)
                .map(IrStorage::Address),
            ParamStorage::Stack => Ok(IrStorage::Stack),
            ParamStorage::CompTime => unsupported("compile-time parameters", span),
        }
    }

    /// Evaluate a fixed address
    fn address(&self, expr: &Expr, module: NodeId, locals: LocalConsts) -> Result<Address> {
        let value = self.eval_const(expr, module, locals, &mut Vec::new())?;
        Address::try_from(value).map_err(|_| TransformError::NotConstant { span: expr.span() })
    }
}

/// Evaluate a binary operation on constants
fn eval_binary(op: OpType, lhs: Int, rhs: Int) -> Option<Int> {
    if let Some(result) = op.eval_compare(lhs, rhs) {
        return Some(Int::from(result));
    }
    if let Some(result) = op.eval_boolean(lhs != 0, rhs != 0) {
        return Some(Int::from(result));
    }
    match op {
        // TODO: rotations depend on the width of the value,
        // which requires tracking the primitive type of the constants
        OpType::RotL | OpType::RotR => None,
        OpType::ShL | OpType::ShRA | OpType::ShRL => {
            op.eval_bitshift(lhs, u32::try_from(rhs).ok()?)
        }
        _ => op.eval_binary(lhs, rhs),
    }
}

/// Evaluate a unary operation on a constant
fn eval_unary(op: OpType, value: Int) -> Option<Int> {
    match op {
        OpType::Not => Some(Int::from(value == 0)),
        // Increments modify a place, they are not constant expressions
        OpType::Inc | OpType::Dec => None,
        _ => op.eval_unnary(value),
    }
}

/// Comparison that holds exactly when the given one does not
fn negate(op: Operator) -> Operator {
    let op_type = op.op_type.inverse_comparison().unwrap_or(op.op_type);
    Operator { op_type, ..op }
}

/// Path of an element owned by another one (`parent::ident`)
fn child(parent: &Path, ident: Ident) -> Path {
    let mut path = parent.as_slice().to_vec();
    path.push(ident);
    Path::new(path)
}

/// Is the parameter a compile-time (`let`) parameter ?
fn is_comptime(param: &ParamDecl) -> bool {
    matches!(
        &param.param_type,
        ParamType::Memory(mem) if mem.storage == ParamStorage::CompTime
    )
}

/// Location where the caller writes an argument
fn param_operand(func: &Path, param: &ParamDecl) -> Operand {
    match &param.param_type {
        ParamType::Register(reg) => Operand::Reg(reg.atom),
        ParamType::Memory(mem) => Operand::mem(child(func, mem.ident.clone())),
    }
}

/// Location where the callee writes its result.
/// `return` is a keyword, so it cannot clash with a user-defined name.
fn return_operand(func: &Path, ret: &ReturnType) -> Operand {
    match ret {
        ReturnType::Register(reg) => Operand::Reg(reg.atom),
        ReturnType::Memory(_) => Operand::mem(child(func, Ident::new("return"))),
    }
}

// MARK: Functions

/// Variable visible in a function body
#[derive(Debug, Clone)]
struct Local<'a> {
    /// Where the variable lives (an immediate value for constants)
    operand: Operand,

    /// Declared type, if any (used to compute element sizes)
    ty: Option<&'a TypeUsed>,

    /// What kind of variable it is
    kind: LocalKind,
}

/// Kind of local variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalKind {
    /// Compile-time constant (`const`, `let`)
    Const,

    /// Static variable, exists from the startup of the program
    Static,

    /// Variable or parameter, only exists while the function runs
    Runtime,
}

/// Initializer of a static variable, lowered into the startup routine
struct StaticInit<'a> {
    /// Static variable to initialize
    target: Path,

    /// Initial value
    value: &'a Expr,

    /// Module from which names are resolved
    module: NodeId,

    /// Local variables visible from the initializer (for statics declared in functions)
    scope: HashMap<Ident, Local<'a>>,
}

/// A function lowered without error
struct Lowered<'a> {
    func: IrFunc,

    /// Initializers of the statics declared in the function
    static_inits: Vec<StaticInit<'a>>,

    /// Memory cells of the inline functions expanded in the function
    inline_data: Vec<IrData>,
}

/// State of the function being lowered, saved while an inline function is expanded
struct Context<'a> {
    module: NodeId,
    path: Path,
    label_path: Path,
    scopes: Vec<HashMap<Ident, Local<'a>>>,
    labels: HashSet<Ident>,
    loops: Vec<LoopFrame>,
    ret: Option<Operand>,
    exit: LabelId,
    locals: Vec<IrData>,
    startup: bool,
}

/// Loop currently being lowered, targeted by `break` and `continue`
struct LoopFrame {
    label: Option<Ident>,
    brk: LabelId,
    cont: LabelId,
}

/// Translate the body of one function
struct FuncLowerer<'g, 'a> {
    globals: &'g Globals<'a>,

    /// Module in which the function is declared
    module: NodeId,

    /// Absolute path of the function (or of the inline function being expanded)
    path: Path,

    /// Prefix of the user labels. Unique for each inline expansion,
    /// so that expanding a function twice does not define its labels twice.
    label_path: Path,

    /// Stack of lexical scopes
    scopes: Vec<HashMap<Ident, Local<'a>>>,

    /// User labels defined in the function
    labels: HashSet<Ident>,

    /// Loops enclosing the current statement
    loops: Vec<LoopFrame>,

    /// Where the return value goes
    ret: Option<Operand>,

    /// Label placed right before the final `Return`
    exit: LabelId,

    /// Lowering the startup routine ? (only constants and statics are available)
    startup: bool,

    /// Initializers of the statics declared in the function
    static_inits: Vec<StaticInit<'a>>,

    /// Statics already initialized by the startup routine
    initialized: HashSet<Path>,

    /// Inline functions being expanded (to detect recursion)
    inline_stack: Vec<Path>,

    /// Number of inline expansions so far
    inline_count: u32,

    /// Memory cells of the expanded inline functions
    inline_data: Vec<IrData>,

    locals: Vec<IrData>,
    body: Vec<OpCode>,
    next_label: LabelId,
    errors: Vec<TransformError>,
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Lowerer for a function declared in `module`
    fn new(globals: &'g Globals<'a>, module: NodeId, path: Path) -> Self {
        Self {
            globals,
            module,
            label_path: path.clone(),
            path,
            scopes: Vec::new(),
            labels: HashSet::new(),
            loops: Vec::new(),
            ret: None,
            exit: 0,
            startup: false,
            static_inits: Vec::new(),
            initialized: HashSet::new(),
            inline_stack: Vec::new(),
            inline_count: 0,
            inline_data: Vec::new(),
            locals: Vec::new(),
            body: Vec::new(),
            next_label: 0,
            errors: Vec::new(),
        }
    }

    /// Lowerer for the startup routine
    fn startup(globals: &'g Globals<'a>, root: NodeId) -> Self {
        let mut lowerer = Self::new(globals, root, Path::from(ROOT));
        lowerer.startup = true;
        lowerer
    }

    /// Lower a whole function, `bank` being the one of the enclosing bank block.
    /// Return every error found in the function if it could not be lowered.
    fn lower(
        mut self,
        func: &'a FuncDecl,
        bank: Option<&Ident>,
    ) -> core::result::Result<Lowered<'a>, Vec<TransformError>> {
        if func.generics.is_some() {
            self.errors.push(TransformError::Unsupported {
                what: "generic functions",
                span: func.span,
            });
        }

        // The code of a function must be placed explicitly
        let header = self
            .globals
            .mem_storage(&func.storage, self.module, &no_locals)
            .unwrap_or_else(|error| {
                self.errors.push(error);
                None
            });
        // Without placement the function cannot be emitted, but its body is
        // still lowered to report the other errors it may contain.
        let storage = match (header, bank) {
            (Some(storage), _) => Some(storage),
            (None, Some(bank)) => Some(IrStorage::Bank(bank.clone())),
            (None, None) => {
                self.errors.push(TransformError::MissingStorage {
                    ident: func.ident.clone(),
                    span: func.span,
                });
                None
            }
        };

        self.scopes.push(HashMap::new());
        let params = self.declare_signature(func, None);

        // Labels can be targeted before their definition, collect them first
        collect_labels(&func.block, &mut self.labels);

        self.exit = self.new_label();
        self.lower_block(&func.block);
        self.emit(OpCode::Label(Label::Local(self.exit)));
        self.emit(OpCode::Return);
        self.scopes.pop();

        match storage {
            Some(storage) if self.errors.is_empty() => Ok(Lowered {
                func: IrFunc {
                    name: self.path,
                    storage,
                    params,
                    ret: self.ret,
                    locals: self.locals,
                    body: self.body,
                },
                static_inits: self.static_inits,
                inline_data: self.inline_data,
            }),
            _ => Err(self.errors),
        }
    }

    /// Declare the parameters and the return value of a function in the current scope.
    /// Memory parameters own a memory cell named `func::param`.
    ///
    /// `comptime` holds the arguments of an inline expansion: they give the value
    /// of the compile-time (`let`) parameters.
    fn declare_signature(
        &mut self,
        func: &'a FuncDecl,
        comptime: Option<&[Operand]>,
    ) -> Vec<Operand> {
        let mut params = Vec::with_capacity(func.parameters.len());
        for (index, param) in func.parameters.iter().enumerate() {
            let operand = param_operand(&self.path, param);
            if let ParamType::Memory(mem) = &param.param_type {
                match (&mem.storage, comptime) {
                    // Compile-time parameter: replaced by the value of the argument
                    (ParamStorage::CompTime, Some(args)) => match args.get(index) {
                        Some(Operand::Imm(value)) => self.bind(
                            mem.ident.clone(),
                            Operand::Imm(*value),
                            Some(&mem.type_used),
                            LocalKind::Const,
                        ),
                        _ => self
                            .errors
                            .push(TransformError::NotConstant { span: param.span }),
                    },
                    _ => {
                        let data = self.param_data(
                            child(&self.path, mem.ident.clone()),
                            &mem.type_used,
                            &mem.storage,
                            mem.span,
                        );
                        self.push_local_data(data);
                        self.bind(
                            mem.ident.clone(),
                            operand.clone(),
                            Some(&mem.type_used),
                            LocalKind::Runtime,
                        );
                    }
                }
            }
            params.push(operand);
        }

        // Return value
        if let Some(ret) = &func.return_type {
            let operand = return_operand(&self.path, ret);
            if let ReturnType::Memory(mem) = ret.as_ref() {
                let data = self.param_data(
                    child(&self.path, Ident::new("return")),
                    &mem.type_used,
                    &mem.storage,
                    mem.span,
                );
                self.push_local_data(data);
            }
            self.ret = Some(operand);
        }
        params
    }

    /// Replace a call to an inline function by the body of the function.
    /// The runtime arguments are written into the parameters like for a call.
    fn expand_inline(
        &mut self,
        symbol: &'g Symbol<'a>,
        func: &'a FuncDecl,
        args: Vec<Operand>,
        span: Span,
    ) -> Result<Option<Operand>> {
        if self.inline_stack.contains(&symbol.path) {
            return Err(TransformError::RecursiveInline {
                path: symbol.path.clone(),
                span,
            });
        }
        if func.generics.is_some() {
            return unsupported("generic functions", func.span);
        }

        // Compile-time parameters need a constant argument
        for (param, arg) in func.parameters.iter().zip(&args) {
            if is_comptime(param) && !matches!(arg, Operand::Imm(_)) {
                return Err(TransformError::NotConstant { span });
            }
        }
        for (param, arg) in func.parameters.iter().zip(&args) {
            if !is_comptime(param) {
                self.transfer(param_operand(&symbol.path, param), arg.clone());
            }
        }

        // Switch to the context of the inline function
        let name = symbol
            .path
            .as_slice()
            .last()
            .expect("a path is never empty");
        let label_path = child(
            &self.label_path,
            Ident::new(&format!("{name}.{}", self.inline_count)),
        );
        self.inline_count += 1;
        let caller = Context {
            module: core::mem::replace(&mut self.module, symbol.module),
            path: core::mem::replace(&mut self.path, symbol.path.clone()),
            label_path: core::mem::replace(&mut self.label_path, label_path),
            scopes: core::mem::take(&mut self.scopes),
            labels: core::mem::take(&mut self.labels),
            loops: core::mem::take(&mut self.loops),
            ret: self.ret.take(),
            exit: self.exit,
            locals: core::mem::take(&mut self.locals),
            startup: core::mem::replace(&mut self.startup, false),
        };

        // Lower the body: `return` jumps right after the expansion
        self.inline_stack.push(symbol.path.clone());
        self.scopes.push(HashMap::new());
        self.declare_signature(func, Some(&args));
        collect_labels(&func.block, &mut self.labels);
        self.exit = self.new_label();
        self.lower_block(&func.block);
        self.place_label(self.exit);
        self.inline_stack.pop();
        let ret = self.ret.take();

        // Back to the caller
        let cells = core::mem::replace(&mut self.locals, caller.locals);
        self.inline_data.extend(cells);
        self.module = caller.module;
        self.path = caller.path;
        self.label_path = caller.label_path;
        self.scopes = caller.scopes;
        self.labels = caller.labels;
        self.loops = caller.loops;
        self.ret = caller.ret;
        self.exit = caller.exit;
        self.startup = caller.startup;
        Ok(ret)
    }

    /// Startup routine: compute the initial value of a static variable at runtime
    fn lower_static_init(&mut self, init: StaticInit<'a>) {
        // A static declared in an inline function is found at each expansion
        if !self.initialized.insert(init.target.clone()) {
            return;
        }
        // The initializer resolves names from where the static is declared
        self.module = init.module;
        self.scopes = vec![init.scope];
        if let Err(error) = self.lower_into(Operand::mem(init.target), init.value) {
            self.errors.push(error);
        }
    }

    /// Finalize the startup routine.
    /// Return every error found in the initializers if it could not be lowered.
    fn finish_init(mut self) -> core::result::Result<Lowered<'a>, Vec<TransformError>> {
        // Statics declared in inline functions expanded by the startup routine
        while let Some(init) = self.static_inits.pop() {
            self.lower_static_init(init);
        }
        self.emit(OpCode::Return);
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        Ok(Lowered {
            func: IrFunc {
                name: self.path,
                storage: IrStorage::Startup,
                params: Vec::new(),
                ret: None,
                locals: self.locals,
                body: self.body,
            },
            static_inits: Vec::new(),
            inline_data: self.inline_data,
        })
    }

    /// Memory cell of a parameter or of the return value
    fn param_data(
        &self,
        name: Path,
        ty: &TypeUsed,
        storage: &ParamStorage,
        span: Span,
    ) -> Result<IrData> {
        Ok(IrData {
            name,
            size: self.globals.size_of(ty, self.module, &no_locals)?,
            storage: self.globals.param_storage(storage, self.module, span)?,
        })
    }

    fn push_local_data(&mut self, data: Result<IrData>) {
        match data {
            Ok(data) => self.locals.push(data),
            Err(error) => self.errors.push(error),
        }
    }

    // MARK: Helpers

    #[inline]
    fn emit(&mut self, opcode: OpCode) {
        self.body.push(opcode);
    }

    #[inline]
    fn new_label(&mut self) -> LabelId {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    #[inline]
    fn place_label(&mut self, label: LabelId) {
        self.emit(OpCode::Label(Label::Local(label)));
    }

    #[inline]
    fn jump(&mut self, label: LabelId) {
        self.emit(OpCode::Jump(JumpTarget::Label(Label::Local(label))));
    }

    /// Move a value, unless it already is at its destination
    fn transfer(&mut self, dst: Operand, src: Operand) {
        if dst != src {
            self.emit(OpCode::Transfer(OpTransfer { dst, src }.to_box()));
        }
    }

    fn branch(&mut self, cond: Condition, label: LabelId) {
        let target = JumpTarget::Label(Label::Local(label));
        self.emit(OpCode::Branch(OpBranch { cond, target }.to_box()));
    }

    fn compare(&mut self, op: Operator, lhs: Operand, rhs: Operand, label: LabelId) {
        self.branch(
            Condition::Compare(OpCompare { op, lhs, rhs }.to_box()),
            label,
        );
    }

    fn bind(&mut self, ident: Ident, operand: Operand, ty: Option<&'a TypeUsed>, kind: LocalKind) {
        let local = Local { operand, ty, kind };
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(ident, local);
        }
    }

    /// Find a local variable (only single identifiers can be locals)
    fn local(&self, path: &Path) -> Option<&Local<'a>> {
        let [ident] = path.as_slice() else {
            return None;
        };
        self.scopes.iter().rev().find_map(|scope| scope.get(ident))
    }

    /// Every local variable visible from the current scope
    fn visible_locals(&self) -> HashMap<Ident, Local<'a>> {
        let mut visible = HashMap::new();
        for scope in &self.scopes {
            for (ident, local) in scope {
                visible.insert(ident.clone(), local.clone());
            }
        }
        visible
    }

    /// Unique IR name for a local variable (shadowed names get a suffix)
    fn local_path(&self, ident: &Ident) -> Path {
        let base = child(&self.path, ident.clone());
        let taken = |path: &Path| self.locals.iter().any(|data| data.name == *path);
        if !taken(&base) {
            return base;
        }
        // '.' cannot appear in a user identifier
        (1..)
            .map(|n| child(&self.path, Ident::new(&format!("{ident}.{n}"))))
            .find(|path| !taken(path))
            .expect("there is always a free name")
    }

    /// Value of a local constant (see [`LocalConsts`])
    fn local_const(&self, path: &Path) -> Option<Option<Int>> {
        self.local(path).map(|local| match &local.operand {
            Operand::Imm(value) if local.kind == LocalKind::Const => Some(*value),
            _ => None,
        })
    }

    /// Try to evaluate an expression at compile time, using local constants
    fn try_const(&self, expr: &Expr) -> Option<Int> {
        let locals = |path: &Path| self.local_const(path);
        self.globals
            .eval_const(expr, self.module, &locals, &mut Vec::new())
            .ok()
    }

    /// Evaluate an expression which must be constant
    fn require_const(&self, expr: &Expr) -> Result<Int> {
        self.try_const(expr)
            .ok_or(TransformError::NotConstant { span: expr.span() })
    }

    // MARK: Statements

    fn lower_block(&mut self, block: &'a [Statement]) {
        self.scopes.push(HashMap::new());
        for statement in block {
            if let Err(error) = self.lower_statement(statement) {
                self.errors.push(error);
            }
        }
        self.scopes.pop();
    }

    fn lower_statement(&mut self, statement: &'a Statement) -> Result<()> {
        if let Some(label) = &statement.label {
            let path = child(&self.label_path, label.clone());
            self.emit(OpCode::Label(Label::Named(path)));
        }

        match &statement.inner {
            // TODO: imports
            StmtInner::Use(_) => Ok(()),
            StmtInner::Bank(bank) => {
                for var in bank.variables.iter() {
                    if let Err(error) = self.declare_local(var, Some(bank.ident.clone())) {
                        self.errors.push(error);
                    }
                }
                Ok(())
            }
            StmtInner::Var(var) => self.declare_local(var, None),
            StmtInner::Instr(instr) => self.lower_instr(&instr.expr),
            StmtInner::Defer(defer) => unsupported("`defer`", defer.span),
            StmtInner::If(if_else) => self.lower_if(if_else),
            StmtInner::Switch(switch) => self.lower_switch(switch),
            StmtInner::Loop(loop_stmt) => self.lower_loop(loop_stmt, statement.label.clone()),
            StmtInner::LoopCtrl(ctrl) => {
                let frame = match &ctrl.label {
                    Some(label) => self
                        .loops
                        .iter()
                        .rev()
                        .find(|frame| frame.label.as_ref() == Some(label))
                        .ok_or_else(|| TransformError::UnknownLabel {
                            label: label.clone(),
                            span: ctrl.span,
                        })?,
                    None => self
                        .loops
                        .last()
                        .ok_or(TransformError::OutsideLoop { span: ctrl.span })?,
                };
                let target = match ctrl.ctrl_type {
                    LoopCtrlType::Break => frame.brk,
                    LoopCtrlType::Continue => frame.cont,
                };
                self.jump(target);
                Ok(())
            }
            StmtInner::Goto(goto) => {
                if !self.labels.contains(&goto.label) {
                    return Err(TransformError::UnknownLabel {
                        label: goto.label.clone(),
                        span: goto.span,
                    });
                }
                let path = child(&self.label_path, goto.label.clone());
                self.emit(OpCode::Jump(JumpTarget::Label(Label::Named(path))));
                Ok(())
            }
            StmtInner::Return(ret) => {
                if let Some(value) = &ret.value {
                    let Some(dst) = self.ret.clone() else {
                        return Err(TransformError::NoReturnValue {
                            path: self.path.clone(),
                            span: ret.span,
                        });
                    };
                    self.lower_into(dst, value)?;
                }
                self.jump(self.exit);
                Ok(())
            }
        }
    }

    /// Lower an expression used as a statement: it must do something
    fn lower_instr(&mut self, expr: &'a Expr) -> Result<()> {
        match expr {
            Expr::Group(group) => self.lower_instr(&group.atom),
            Expr::Assign(_) => self.lower_operand(expr).map(drop),
            // The result of the call, if any, is ignored
            Expr::Call(call) => self.lower_call(call).map(drop),
            Expr::Unary(unary) if matches!(unary.op.op_type, OpType::Inc | OpType::Dec) => {
                self.lower_operand(expr).map(drop)
            }
            _ => Err(TransformError::NoEffect { span: expr.span() }),
        }
    }

    /// Declare a variable in the current scope
    fn declare_local(&mut self, var: &'a VarDecl, bank: Option<Ident>) -> Result<()> {
        match var.var_type {
            // Constants are replaced by their value
            VarType::Const | VarType::Let => {
                let value = match &var.assign_expr {
                    Some(expr) => self.require_const(expr)?,
                    None => return Err(TransformError::NotConstant { span: var.span }),
                };
                self.bind(
                    var.ident.clone(),
                    Operand::Imm(value),
                    Some(&var.type_used),
                    LocalKind::Const,
                );
                Ok(())
            }
            VarType::Stack => unsupported("stack variables", var.span),
            VarType::Var | VarType::Static => {
                let locals = |path: &Path| self.local_const(path);
                let storage = self
                    .globals
                    .var_storage(var, self.module, &locals, bank.as_ref())?;
                let size = self.globals.size_of(&var.type_used, self.module, &locals)?;

                // The initializer is evaluated before the variable is visible
                let name = self.local_path(&var.ident);
                let operand = Operand::mem(name.clone());
                let kind = match var.var_type {
                    VarType::Static => LocalKind::Static,
                    _ => LocalKind::Runtime,
                };
                match (&var.assign_expr, kind) {
                    // A static is initialized once, by the startup routine
                    (Some(value), LocalKind::Static) => self.static_inits.push(StaticInit {
                        target: name.clone(),
                        value,
                        module: self.module,
                        scope: self.visible_locals(),
                    }),
                    // A var is initialized each time it is declared
                    (Some(value), _) => self.lower_into(operand.clone(), value)?,
                    (None, _) => {}
                }

                self.locals.push(IrData {
                    name,
                    size,
                    storage,
                });
                self.bind(var.ident.clone(), operand, Some(&var.type_used), kind);
                Ok(())
            }
        }
    }

    fn lower_if(&mut self, if_else: &'a IfElse) -> Result<()> {
        let end = self.new_label();
        for branch in if_else.branches.iter() {
            match &branch.condition {
                Some(condition) => {
                    let next = self.new_label();
                    self.lower_cond(condition, false, next)?;
                    self.lower_block(&branch.block);
                    self.jump(end);
                    self.place_label(next);
                }
                None => self.lower_block(&branch.block),
            }
        }
        self.place_label(end);
        Ok(())
    }

    /// A switch is lowered as a chain of comparisons
    fn lower_switch(&mut self, switch: &'a Switch) -> Result<()> {
        let value = self.lower_operand(&switch.variable)?;
        let end = self.new_label();
        for case in switch.cases.iter() {
            match &case.condition {
                Some(case_value) => {
                    let case_value = self.lower_operand(case_value)?;
                    let next = self.new_label();
                    self.compare(op::DIFF, value.clone(), case_value, next);
                    self.lower_block(&case.block);
                    self.jump(end);
                    self.place_label(next);
                }
                None => self.lower_block(&case.block),
            }
        }
        self.place_label(end);
        Ok(())
    }

    fn lower_loop(&mut self, loop_stmt: &'a Loop, label: Option<Ident>) -> Result<()> {
        let head = self.new_label();
        let end = self.new_label();

        match &loop_stmt.loop_type {
            // head: body; jump head; end:
            LoopType::Loop => {
                self.place_label(head);
                self.lower_loop_body(&loop_stmt.block, label, end, head);
                self.jump(head);
            }
            // head: if !cond goto end; body; jump head; end:
            LoopType::While(header) if !header.do_while => {
                self.place_label(head);
                self.lower_cond(&header.condition, false, end)?;
                self.lower_loop_body(&loop_stmt.block, label, end, head);
                self.jump(head);
            }
            // head: body; cont: if cond goto head; end:
            LoopType::While(header) => {
                let cont = self.new_label();
                self.place_label(head);
                self.lower_loop_body(&loop_stmt.block, label, end, cont);
                self.place_label(cont);
                self.lower_cond(&header.condition, true, head)?;
            }
            // iter = start;
            // head: if iter >= end goto end; body;
            // cont: iter += step; jump head; end:
            LoopType::For(header) => {
                if header.inline {
                    return unsupported("`inline for`", header.span);
                }
                self.scopes.push(HashMap::new());
                let iter = match &header.iter {
                    ParamType::Register(reg) => Operand::Reg(reg.atom),
                    ParamType::Memory(mem) => {
                        let name = self.local_path(&mem.ident);
                        let data =
                            self.param_data(name.clone(), &mem.type_used, &mem.storage, mem.span)?;
                        self.locals.push(data);
                        let operand = Operand::mem(name);
                        self.bind(
                            mem.ident.clone(),
                            operand.clone(),
                            Some(&mem.type_used),
                            LocalKind::Runtime,
                        );
                        operand
                    }
                };

                self.lower_into(iter.clone(), &header.start)?;
                let last = self.lower_operand(&header.end)?;
                let step = match &header.step {
                    Some(step) => self.lower_operand(step)?,
                    None => Operand::Imm(1),
                };

                // TODO: an inclusive range ending on the maximum value of the
                // iterator type never terminates, it needs a dedicated lowering.
                let exit_op = if header.range == Range::incl() {
                    op::GT
                } else {
                    op::G_EQ
                };
                let cont = self.new_label();
                self.place_label(head);
                self.compare(exit_op, iter.clone(), last, end);
                self.lower_loop_body(&loop_stmt.block, label, end, cont);
                self.place_label(cont);
                self.emit(OpCode::Binary(
                    OpBinary {
                        op: op::ADD,
                        dst: iter.clone(),
                        lhs: iter,
                        rhs: step,
                    }
                    .to_box(),
                ));
                self.jump(head);
                self.scopes.pop();
            }
        }

        self.place_label(end);
        Ok(())
    }

    fn lower_loop_body(
        &mut self,
        block: &'a [Statement],
        label: Option<Ident>,
        brk: LabelId,
        cont: LabelId,
    ) {
        self.loops.push(LoopFrame { label, brk, cont });
        self.lower_block(block);
        self.loops.pop();
    }

    // MARK: Conditions

    /// Branch to `target` when `expr` evaluates to `when`, fall through otherwise.
    /// `&&` and `||` are short-circuited.
    fn lower_cond(&mut self, expr: &'a Expr, when: bool, target: LabelId) -> Result<()> {
        if let Some(value) = self.try_const(expr) {
            if (value != 0) == when {
                self.jump(target);
            }
            return Ok(());
        }

        match expr {
            Expr::Group(group) => self.lower_cond(&group.atom, when, target),
            Expr::Status(flag) => {
                let cond = Condition::Status {
                    flag: flag.atom,
                    inverse: !when,
                };
                self.branch(cond, target);
                Ok(())
            }
            Expr::Unary(unary) if unary.op.op_type == OpType::Not => {
                self.lower_cond(&unary.expr, !when, target)
            }
            Expr::Binary(binary) if binary.op.op_type == OpType::And => {
                if when {
                    // jump if both are true: skip as soon as one is false
                    let skip = self.new_label();
                    self.lower_cond(&binary.lhs, false, skip)?;
                    self.lower_cond(&binary.rhs, true, target)?;
                    self.place_label(skip);
                } else {
                    self.lower_cond(&binary.lhs, false, target)?;
                    self.lower_cond(&binary.rhs, false, target)?;
                }
                Ok(())
            }
            Expr::Binary(binary) if binary.op.op_type == OpType::Or => {
                if when {
                    self.lower_cond(&binary.lhs, true, target)?;
                    self.lower_cond(&binary.rhs, true, target)?;
                } else {
                    // jump if both are false: skip as soon as one is true
                    let skip = self.new_label();
                    self.lower_cond(&binary.lhs, true, skip)?;
                    self.lower_cond(&binary.rhs, false, target)?;
                    self.place_label(skip);
                }
                Ok(())
            }
            Expr::Binary(binary) if OpType::is_comparison(binary.op.op_type) => {
                let lhs = self.lower_operand(&binary.lhs)?;
                let rhs = self.lower_operand(&binary.rhs)?;
                let op = if when { binary.op } else { negate(binary.op) };
                self.compare(op, lhs, rhs, target);
                Ok(())
            }
            // Any other value: true when non-zero
            _ => {
                let value = self.lower_operand(expr)?;
                let op = if when { op::DIFF } else { op::EQ };
                self.compare(op, value, Operand::Imm(0), target);
                Ok(())
            }
        }
    }

    // MARK: Expressions

    /// Lower an expression which must map onto a single operand.
    ///
    /// Assignments and calls are operands: their value is their destination
    /// (`x = a = 1` stores 1 in `a`, then `a` in `x`).
    fn lower_operand(&mut self, expr: &'a Expr) -> Result<Operand> {
        if let Some(value) = self.try_const(expr) {
            return Ok(Operand::Imm(value));
        }

        match expr {
            Expr::Lit(lit) => unsupported("float and string literals", lit.span),
            Expr::Register(reg) => Ok(Operand::Reg(reg.atom)),
            Expr::Var(var) => self.lookup_var(&var.atom, var.span),
            Expr::Group(group) => self.lower_operand(&group.atom),
            Expr::Cast(cast) => self.lower_operand(&cast.expr),
            Expr::Index(_) | Expr::Access(_) => self.lower_place(expr),
            Expr::Unary(unary) => match unary.op.op_type {
                OpType::Add => self.lower_operand(&unary.expr),
                // `++x` / `--x` modify their operand in place
                OpType::Inc | OpType::Dec => {
                    let place = self.lower_place(&unary.expr)?;
                    self.emit(OpCode::Unary(
                        OpUnary {
                            op: unary.op,
                            dst: place.clone(),
                            src: place.clone(),
                        }
                        .to_box(),
                    ));
                    Ok(place)
                }
                _ => Err(TransformError::NotAnOperand { span: expr.span() }),
            },
            Expr::Assign(assign) => {
                let dst = self.lower_place(&assign.lhs)?;
                match assign.assign.op {
                    // `dst = value` or `dst = lhs op rhs`
                    None => self.lower_into(dst.clone(), &assign.rhs)?,
                    // `dst op= rhs`
                    Some(op) => {
                        let rhs = self.lower_operand(&assign.rhs)?;
                        self.emit(OpCode::Binary(
                            OpBinary {
                                op,
                                dst: dst.clone(),
                                lhs: dst.clone(),
                                rhs,
                            }
                            .to_box(),
                        ));
                    }
                }
                Ok(dst)
            }
            Expr::Call(call) => {
                self.lower_call(call)?
                    .ok_or_else(|| TransformError::NoReturnValue {
                        path: call.callee.clone(),
                        span: call.span,
                    })
            }
            // Status flags, comparisons and other operations need a destination
            Expr::Status(_) | Expr::Binary(_) => {
                Err(TransformError::NotAnOperand { span: expr.span() })
            }
        }
    }

    /// Write the value of an expression into `dst`.
    /// Besides operands, the expression can be a single unary or binary
    /// operation on operands (`dst = -x`, `dst = x + 1`).
    fn lower_into(&mut self, dst: Operand, expr: &'a Expr) -> Result<()> {
        if let Some(value) = self.try_const(expr) {
            self.transfer(dst, Operand::Imm(value));
            return Ok(());
        }

        match expr {
            Expr::Group(group) => self.lower_into(dst, &group.atom),
            Expr::Unary(unary) => match unary.op.op_type {
                OpType::Add | OpType::Inc | OpType::Dec => {
                    let src = self.lower_operand(expr)?;
                    self.transfer(dst, src);
                    Ok(())
                }
                OpType::BitAnd | OpType::Mul => unsupported("pointers", unary.span),
                // `!` is a logical operator, only valid in conditions
                OpType::Not => Err(TransformError::NotAnOperand { span: expr.span() }),
                _ => {
                    let src = self.lower_operand(&unary.expr)?;
                    self.emit(OpCode::Unary(
                        OpUnary {
                            op: unary.op,
                            dst,
                            src,
                        }
                        .to_box(),
                    ));
                    Ok(())
                }
            },
            Expr::Binary(binary) => {
                // Comparisons and logical operators are only valid in conditions
                let op_type = binary.op.op_type;
                if OpType::is_comparison(op_type) || matches!(op_type, OpType::And | OpType::Or) {
                    return Err(TransformError::NotAnOperand { span: expr.span() });
                }
                let lhs = self.lower_operand(&binary.lhs)?;
                let rhs = self.lower_operand(&binary.rhs)?;
                self.emit(OpCode::Binary(
                    OpBinary {
                        op: binary.op,
                        dst,
                        lhs,
                        rhs,
                    }
                    .to_box(),
                ));
                Ok(())
            }
            _ => {
                let src = self.lower_operand(expr)?;
                self.transfer(dst, src);
                Ok(())
            }
        }
    }

    /// Lower an expression designating a location that can be written to
    fn lower_place(&mut self, expr: &'a Expr) -> Result<Operand> {
        let not_assignable = || TransformError::NotAssignable { span: expr.span() };
        match expr {
            Expr::Register(reg) => Ok(Operand::Reg(reg.atom)),
            Expr::Group(group) => self.lower_place(&group.atom),
            Expr::Var(var) => match self.lookup_var(&var.atom, var.span)? {
                Operand::Imm(_) => Err(not_assignable()),
                operand => Ok(operand),
            },
            Expr::Index(index) => {
                let Operand::Mem(mut cell) = self.lower_place(&index.expr)? else {
                    return Err(not_assignable());
                };
                let elem_size = Int::from(self.element_size(&index.expr)?);

                if let Some(value) = self.try_const(&index.index) {
                    cell.offset += value * elem_size;
                    Ok(Operand::Mem(cell))
                } else if let (Expr::Register(reg), 1, None) = (&index.index, elem_size, cell.index)
                {
                    cell.index = Some(reg.atom);
                    Ok(Operand::Mem(cell))
                } else {
                    // TODO: compute the address at runtime
                    unsupported(
                        "indexing with something else than a constant or a register",
                        index.span,
                    )
                }
            }
            // TODO: requires struct layouts
            Expr::Access(access) => unsupported("field access", access.span),
            _ => Err(not_assignable()),
        }
    }

    /// Size of the elements of an array variable
    fn element_size(&self, expr: &Expr) -> Result<u32> {
        let (ty, module) = match expr {
            Expr::Var(var) => match self.local(&var.atom) {
                Some(local) => (local.ty, self.module),
                None => {
                    let symbol = self.globals.resolve(self.module, &var.atom, var.span)?;
                    match symbol.decl {
                        Decl::Var(decl) => (Some(&decl.type_used), symbol.module),
                        _ => (None, symbol.module),
                    }
                }
            },
            _ => (None, self.module),
        };
        match ty {
            Some(TypeUsed::Array(array)) => {
                self.globals.size_of(&array.cell_type, module, &no_locals)
            }
            _ => unsupported(
                "indexing something else than an array variable",
                expr.span(),
            ),
        }
    }

    /// Operand of a variable
    fn lookup_var(&self, path: &Path, span: Span) -> Result<Operand> {
        if let Some(local) = self.local(path) {
            if self.startup && local.kind == LocalKind::Runtime {
                return Err(TransformError::NotAtStartup {
                    ident: path[0].clone(),
                    span,
                });
            }
            return Ok(local.operand.clone());
        }

        let symbol = self.globals.resolve(self.module, path, span)?;
        match symbol.decl {
            Decl::Var(var) if matches!(var.var_type, VarType::Const | VarType::Let) => {
                let value = var
                    .assign_expr
                    .as_ref()
                    .ok_or(TransformError::NotConstant { span })?;
                self.globals
                    .eval_const(value, symbol.module, &no_locals, &mut Vec::new())
                    .map(Operand::Imm)
            }
            Decl::Var(_) => Ok(Operand::mem(symbol.path.clone())),
            _ => Err(TransformError::WrongKind {
                path: path.clone(),
                expected: "variable",
                span,
            }),
        }
    }

    /// Lower a function call.
    /// Return the location of the result, or `None` if the function returns nothing.
    fn lower_call(&mut self, call: &'a FunctionCall) -> Result<Option<Operand>> {
        let symbol = self.globals.resolve(self.module, &call.callee, call.span)?;
        let Decl::Func(func) = symbol.decl else {
            return Err(TransformError::WrongKind {
                path: call.callee.clone(),
                expected: "function",
                span: call.span,
            });
        };

        let params = &func.parameters;
        let arg_count_error = || TransformError::ArgCount {
            expected: params.len(),
            found: call.args.len(),
            span: call.span,
        };
        if call.args.len() > params.len() {
            return Err(arg_count_error());
        }

        // Evaluate every argument before writing any parameter,
        // so that nested calls do not overwrite parameters already written.
        // Arguments are then written in declaration order: reading a register
        // already overwritten by a previous argument is a mistake of the user.
        let mut values = Vec::with_capacity(params.len());
        for (index, param) in params.iter().enumerate() {
            let value = match (call.args.get(index), &param.default_value) {
                (Some(arg), _) => self.lower_operand(arg)?,
                (None, Some(default)) => Operand::Imm(self.globals.eval_const(
                    default,
                    symbol.module,
                    &no_locals,
                    &mut Vec::new(),
                )?),
                (None, None) => return Err(arg_count_error()),
            };
            values.push(value);
        }

        if func.marker == FuncMarker::Inline {
            return self.expand_inline(symbol, func, values, call.span);
        }

        for (param, value) in params.iter().zip(values) {
            self.transfer(param_operand(&symbol.path, param), value);
        }
        self.emit(OpCode::Call(symbol.path.clone()));

        // The result stays where the callee wrote it
        Ok(func
            .return_type
            .as_ref()
            .map(|ret| return_operand(&symbol.path, ret)))
    }
}

/// Collect every user label of a function, including those in nested blocks
fn collect_labels(block: &[Statement], labels: &mut HashSet<Ident>) {
    for statement in block {
        if let Some(label) = &statement.label {
            labels.insert(label.clone());
        }
        match &statement.inner {
            StmtInner::Defer(defer) => collect_labels(&defer.block, labels),
            StmtInner::If(if_else) => {
                for branch in if_else.branches.iter() {
                    collect_labels(&branch.block, labels);
                }
            }
            StmtInner::Switch(switch) => {
                for case in switch.cases.iter() {
                    collect_labels(&case.block, labels);
                }
            }
            StmtInner::Loop(loop_stmt) => collect_labels(&loop_stmt.block, labels),
            _ => {}
        }
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use asmanite_ast::{
        chumsky::{
            Parser as _,
            input::{Input as _, Stream},
        },
        lexer::token::Token,
        logos::Logos as _,
        parser::Parsers,
    };
    use asmanite_common::{
        primitive::def::U8,
        register::{REGISTERS, RegDef, RegIdent, RegSet},
    };

    fn parse(source: &str) -> Module {
        REGISTERS.get_or_init(|| {
            RegSet::new(&[
                ("a", RegDef::new(U8)),
                ("x", RegDef::new(U8)),
                ("y", RegDef::new(U8)),
            ])
        });
        let tokens: Vec<_> = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()))
            .collect();
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));
        Parsers::new()
            .module_file(Ident::new("main"))
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    fn translate(source: &str) -> IrProgram {
        transform(&parse(source)).expect("test source should translate")
    }

    fn function<'p>(program: &'p IrProgram, name: &str) -> &'p IrFunc {
        program
            .functions
            .iter()
            .find(|func| func.name == Path::from(name))
            .expect("function should exist")
    }

    fn reg(name: &str) -> Operand {
        Operand::Reg(RegIdent::new(name))
    }

    fn mem(path: &str) -> Operand {
        Operand::mem(Path::from(path))
    }

    fn label(id: LabelId) -> OpCode {
        OpCode::Label(Label::Local(id))
    }

    fn jump(id: LabelId) -> OpCode {
        OpCode::Jump(JumpTarget::Label(Label::Local(id)))
    }

    fn transfer(dst: Operand, src: Operand) -> OpCode {
        OpCode::Transfer(OpTransfer { dst, src }.to_box())
    }

    fn branch_cmp(op: Operator, lhs: Operand, rhs: Operand, target: LabelId) -> OpCode {
        OpCode::Branch(
            OpBranch {
                cond: Condition::Compare(OpCompare { op, lhs, rhs }.to_box()),
                target: JumpTarget::Label(Label::Local(target)),
            }
            .to_box(),
        )
    }

    #[test]
    fn reserves_statics_and_initializes_them_at_startup() {
        let program = translate(
            "const LIMIT: u8 = 2 + 3;
             static DEVICE: u8 at 0x0010 = a = LIMIT;
             static BUFFER: [u8; LIMIT] in ram;
             mod sub { static FLAG: u8 in ram; }",
        );

        // Constants take no memory, statics are placed where the user said
        assert_eq!(
            program.data,
            vec![
                IrData {
                    name: Path::from("DEVICE"),
                    size: 1,
                    storage: IrStorage::Address(0x10),
                },
                IrData {
                    name: Path::from("BUFFER"),
                    size: 5,
                    storage: IrStorage::Bank(Ident::new("ram")),
                },
                IrData {
                    name: Path::from("sub::FLAG"),
                    size: 1,
                    storage: IrStorage::Bank(Ident::new("ram")),
                },
            ]
        );

        // The initial value of a static is computed at runtime, through a register
        assert_eq!(program.init.name, Path::from(ROOT));
        assert_eq!(program.init.storage, IrStorage::Startup);
        assert_eq!(
            program.init.body,
            vec![
                transfer(reg("a"), Operand::Imm(5)),
                transfer(mem("DEVICE"), reg("a")),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn requires_explicit_placement_of_variables() {
        for source in [
            "static FLAG: u8;",
            "mod sub { static FLAG: u8; }",
            "fn f in code () { var local: u8; }",
        ] {
            let errors = transform(&parse(source)).expect_err("translation should fail");
            assert!(
                matches!(&errors[..], [TransformError::MissingStorage { ident, .. }]
                    if *ident == Ident::new("FLAG") || *ident == Ident::new("local")),
                "unexpected errors for {source:?}: {errors:?}"
            );
        }

        // An in-bank block gives a placement to its variables
        let program = translate("fn f in code () { in ram { var local: u8; } }");
        assert_eq!(
            function(&program, "f").locals[0].storage,
            IrStorage::Bank(Ident::new("ram"))
        );
    }

    #[test]
    fn rejects_runtime_variables_in_modules() {
        for (source, var_type) in [
            ("var counter: u8 in ram;", "var"),
            ("let value: u8 = 1;", "let"),
            ("stack value: u8;", "stack"),
        ] {
            let errors = transform(&parse(source)).expect_err("translation should fail");
            assert!(
                matches!(&errors[..], [TransformError::NotInModule { var_type: found, .. }]
                    if *found == var_type),
                "unexpected errors for {source:?}: {errors:?}"
            );
        }
    }

    #[test]
    fn lowers_if_and_return_into_branches() {
        let program = translate(
            "fn check in code (value: u8 in zp) -> a {
                if value == 0 { return 1; }
                return value;
            }",
        );
        let func = function(&program, "check");

        assert_eq!(func.params, vec![mem("check::value")]);
        assert_eq!(func.ret, Some(reg("a")));
        assert_eq!(
            func.locals,
            vec![IrData {
                name: Path::from("check::value"),
                size: 1,
                storage: IrStorage::Bank(Ident::new("zp")),
            }]
        );
        // L0: exit, L1: end of if, L2: next branch
        assert_eq!(
            func.body,
            vec![
                branch_cmp(op::DIFF, mem("check::value"), Operand::Imm(0), 2),
                transfer(reg("a"), Operand::Imm(1)),
                jump(0),
                jump(1),
                label(2),
                label(1),
                transfer(reg("a"), mem("check::value")),
                jump(0),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn lowers_calls_with_default_arguments() {
        let program = translate(
            "fn add in code (a, x = 1) -> a { a += x; return a; }
             fn main in code () { var total: u8 in ram; total = add(2); }",
        );
        let func = function(&program, "main");

        assert_eq!(
            func.body,
            vec![
                transfer(reg("a"), Operand::Imm(2)),
                transfer(reg("x"), Operand::Imm(1)),
                OpCode::Call(Path::from("add")),
                // the result is read from the return register of `add`
                transfer(mem("main::total"), reg("a")),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn lowers_labeled_loops_and_short_circuit_conditions() {
        let program = translate(
            "fn f in code () {
                'outer: loop {
                    while a < 10 && x != 0 { break 'outer; }
                }
            }",
        );
        let func = function(&program, "f");

        // L0: exit, L1/L2: outer head/end, L3/L4: while head/end
        assert_eq!(
            func.body,
            vec![
                OpCode::Label(Label::Named(Path::from("f::outer"))),
                label(1),
                label(3),
                branch_cmp(op::G_EQ, reg("a"), Operand::Imm(10), 4),
                branch_cmp(op::EQ, reg("x"), Operand::Imm(0), 4),
                jump(2),
                jump(3),
                label(4),
                jump(1),
                label(2),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn lowers_for_loop_with_step() {
        let program = translate("fn f in code () { for i: u8 in zp of 0..4 step 2 { a = i; } }");
        let func = function(&program, "f");

        assert_eq!(func.locals[0].name, Path::from("f::i"));
        // L0: exit, L1: head, L2: end, L3: continue
        assert_eq!(
            func.body,
            vec![
                transfer(mem("f::i"), Operand::Imm(0)),
                label(1),
                branch_cmp(op::G_EQ, mem("f::i"), Operand::Imm(4), 2),
                transfer(reg("a"), mem("f::i")),
                label(3),
                OpCode::Binary(
                    OpBinary {
                        op: op::ADD,
                        dst: mem("f::i"),
                        lhs: mem("f::i"),
                        rhs: Operand::Imm(2),
                    }
                    .to_box()
                ),
                jump(1),
                label(2),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn reports_every_error() {
        let errors = transform(&parse(
            "fn f in code () { break; goto 'nowhere; unknown = 1; }
             fn f in code () {}",
        ))
        .expect_err("translation should fail");

        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TransformError::Duplicate { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TransformError::OutsideLoop { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TransformError::UnknownLabel { .. }))
        );
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, TransformError::Unresolved { .. }))
        );
    }

    fn binary(op: Operator, dst: Operand, lhs: Operand, rhs: Operand) -> OpCode {
        OpCode::Binary(OpBinary { op, dst, lhs, rhs }.to_box())
    }

    #[test]
    fn maps_expressions_onto_operands_without_temporaries() {
        let program = translate(
            "fn f in code (v: u8 in zp) {
                a = v + 1;
                a += 2;
                x = y = 3;
                v = -a;
                v = x;
            }",
        );

        assert_eq!(
            function(&program, "f").body,
            vec![
                binary(op::ADD, reg("a"), mem("f::v"), Operand::Imm(1)),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(2)),
                transfer(reg("y"), Operand::Imm(3)),
                transfer(reg("x"), reg("y")),
                OpCode::Unary(
                    OpUnary {
                        op: op::SUB,
                        dst: mem("f::v"),
                        src: reg("a"),
                    }
                    .to_box()
                ),
                transfer(mem("f::v"), reg("x")),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn rejects_expressions_needing_temporaries() {
        let errors = transform(&parse(
            "fn f in code () {
                a = x + y * 2;
                a = x < y;
                if a + 1 == 3 {}
                a + 1;
            }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(
                &errors[..],
                [
                    TransformError::NotAnOperand { .. },
                    TransformError::NotAnOperand { .. },
                    TransformError::NotAnOperand { .. },
                    TransformError::NoEffect { .. },
                ]
            ),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn places_functions_explicitly_or_through_bank_blocks() {
        let program = translate(
            "fn header in code () {}
             fn fixed at 0x8000 () {}
             inline fn expanded() {}
             in prog {
                 fn banked() {}
                 static FLAG: u8;
             }",
        );

        let storage = |name| function(&program, name).storage.clone();
        assert_eq!(storage("header"), IrStorage::Bank(Ident::new("code")));
        assert_eq!(storage("fixed"), IrStorage::Address(0x8000));
        // inline functions produce no code of their own
        assert!(
            program
                .functions
                .iter()
                .all(|func| func.name != Path::from("expanded"))
        );
        assert_eq!(storage("banked"), IrStorage::Bank(Ident::new("prog")));
        assert_eq!(program.data[0].storage, IrStorage::Bank(Ident::new("prog")));

        let errors = transform(&parse("fn f() {}")).expect_err("translation should fail");
        assert!(matches!(
            &errors[..],
            [TransformError::MissingStorage { .. }]
        ));

        let errors =
            transform(&parse("inline fn f in code () {}")).expect_err("translation should fail");
        assert!(matches!(
            &errors[..],
            [TransformError::InlinePlacement { .. }]
        ));
    }

    #[test]
    fn initializes_function_statics_after_global_ones() {
        let program = translate(
            "fn f in code () {
                const OFFSET: u8 = 2;
                static COUNT: u8 in ram = a = OFFSET;
                static TOTAL: u8 in ram = x = COUNT;
             }
             static BASE: u8 at 0x10 = a = 1;",
        );

        // BASE is declared last, but global statics are initialized first
        assert_eq!(
            program.init.body,
            vec![
                transfer(reg("a"), Operand::Imm(1)),
                transfer(mem("BASE"), reg("a")),
                transfer(reg("a"), Operand::Imm(2)),
                transfer(mem("f::COUNT"), reg("a")),
                transfer(reg("x"), mem("f::COUNT")),
                transfer(mem("f::TOTAL"), reg("x")),
                OpCode::Return,
            ]
        );
        // The memory of the statics belongs to the function, no code is emitted there
        let func = function(&program, "f");
        assert_eq!(func.locals.len(), 2);
        assert_eq!(func.body, vec![label(0), OpCode::Return]);
    }

    #[test]
    fn rejects_runtime_values_in_static_initializers() {
        let errors = transform(&parse(
            "fn f in code (v: u8 in zp) { static S: u8 in ram = a = v; }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [TransformError::NotAtStartup { ident, .. }]
                if *ident == Ident::new("v")),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn expands_inline_functions_at_each_call_site() {
        let program = translate(
            "inline fn add_to(v: u8 in zp, x, let amount: u8 = 1) -> a {
                a = v;
                a += amount;
                'done: return a;
             }
             fn main in code () {
                var total: u8 in ram;
                total = add_to(total, 5);
                total = add_to(total, 6, 2);
             }",
        );

        assert_eq!(program.functions.len(), 1);
        // L0: exit of main, L1/L2: exit of each expansion
        assert_eq!(
            function(&program, "main").body,
            vec![
                transfer(mem("add_to::v"), mem("main::total")),
                transfer(reg("x"), Operand::Imm(5)),
                transfer(reg("a"), mem("add_to::v")),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(1)),
                OpCode::Label(Label::Named(Path::from("main::add_to.0::done"))),
                jump(1),
                label(1),
                transfer(mem("main::total"), reg("a")),
                transfer(mem("add_to::v"), mem("main::total")),
                transfer(reg("x"), Operand::Imm(6)),
                transfer(reg("a"), mem("add_to::v")),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(2)),
                OpCode::Label(Label::Named(Path::from("main::add_to.1::done"))),
                jump(2),
                label(2),
                transfer(mem("main::total"), reg("a")),
                label(0),
                OpCode::Return,
            ]
        );
        // The parameter cell of the inline function is reserved once
        assert_eq!(
            program.data,
            vec![IrData {
                name: Path::from("add_to::v"),
                size: 1,
                storage: IrStorage::Bank(Ident::new("zp")),
            }]
        );
    }

    #[test]
    fn expands_nested_inline_functions() {
        let program = translate(
            "inline fn inner() { a += 1; }
             inline fn outer() { inner(); inner(); }
             fn main in code () { outer(); }",
        );

        let increment = binary(op::ADD, reg("a"), reg("a"), Operand::Imm(1));
        // L0: exit of main, L1: exit of outer, L2/L3: exits of inner
        assert_eq!(
            function(&program, "main").body,
            vec![
                increment.clone(),
                label(2),
                increment,
                label(3),
                label(1),
                label(0),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn rejects_recursive_inline_functions() {
        let errors = transform(&parse(
            "inline fn ping() { pong(); }
             inline fn pong() { ping(); }
             fn main in code () { ping(); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [TransformError::RecursiveInline { path, .. }]
                if *path == Path::from("ping")),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn reports_errors_of_inline_functions_once() {
        let errors = transform(&parse(
            "inline fn bad() { unknown = 1; }
             fn main in code () { bad(); bad(); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [TransformError::Unresolved { .. }]),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn requires_constant_arguments_for_compile_time_parameters() {
        let errors = transform(&parse(
            "inline fn shift(let count: u8) { a <<= count; }
             fn main in code (v: u8 in zp) { shift(v); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [TransformError::NotConstant { .. }]),
            "unexpected errors: {errors:?}"
        );
    }
}
