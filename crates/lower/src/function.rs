//! Pass 3: lowering of a function and of its signature

use super::{
    error::{LowError, LowErrorKind, Result},
    scope::{Local, LocalKind},
    startup::StaticInit,
    statement::{LoopFrame, collect_labels},
};
use crate::symbol::{NodeId, SymbolTree, no_locals};
use asmanite_ast::parser::{
    composite_type::TypeUsed,
    function::FuncDecl,
    memory::{ParamDecl, ParamStorage, ParamType, ReturnType},
};
use asmanite_common::{ident::Ident, path::Path, span::Span};
use asmanite_ir::{
    LabelId,
    intermediate::{IrData, IrFunc, IrStorage, Label, OpCode, Operand},
};
use std::collections::{HashMap, HashSet};

/// A function lowered without error
pub struct Lowered<'a> {
    pub(crate) func: IrFunc,

    /// Initializers of the statics declared in the function
    pub(crate) static_inits: Vec<StaticInit<'a>>,

    /// Memory cells of the inline functions expanded in the function
    pub(crate) inline_data: Vec<IrData>,
}

/// Translate the body of one function
pub struct FuncLowerer<'g, 'a> {
    pub globals: &'g SymbolTree<'a>,

    /// Module in which the function is declared
    pub(crate) module: NodeId,

    /// Absolute path of the function (or of the inline function being expanded)
    pub(crate) path: Path,

    /// Prefix of the user labels. Unique for each inline expansion,
    /// so that expanding a function twice does not define its labels twice.
    pub(crate) label_path: Path,

    /// Stack of lexical scopes
    pub(crate) scopes: Vec<HashMap<Ident, Local<'a>>>,

    /// User labels defined in the function
    pub(crate) labels: HashSet<Ident>,

    /// Loops enclosing the current statement
    pub(crate) loops: Vec<LoopFrame>,

    /// Where the return value goes
    pub(crate) ret: Option<Operand>,

    /// Label placed right before the final `Return`
    pub(crate) exit: LabelId,

    /// Lowering the startup routine ? (only constants and statics are available)
    pub(crate) startup: bool,

    /// Initializers of the statics declared in the function
    pub(crate) static_inits: Vec<StaticInit<'a>>,

    /// Statics already initialized by the startup routine
    pub(crate) initialized: HashSet<Path>,

    /// Inline functions being expanded (to detect recursion)
    pub(crate) inline_stack: Vec<Path>,

    /// Number of inline expansions so far
    pub(crate) inline_count: u32,

    /// Memory cells of the expanded inline functions
    pub(crate) inline_data: Vec<IrData>,

    pub(crate) locals: Vec<IrData>,
    pub(crate) body: Vec<OpCode>,
    pub(crate) next_label: LabelId,
    pub(crate) errors: Vec<LowError>,
}

/// Path of an element owned by another one (`parent::ident`)
pub fn child(parent: &Path, ident: Ident) -> Path {
    let mut path = parent.as_slice().to_vec();
    path.push(ident);
    Path::new(path)
}

/// Is the parameter a compile-time (`let`) parameter ?
pub fn is_comptime(param: &ParamDecl) -> bool {
    matches!(
        &param.param_type,
        ParamType::Memory(mem) if mem.storage == ParamStorage::CompTime
    )
}

/// Location where the caller writes an argument
pub fn param_operand(func: &Path, param: &ParamDecl) -> Operand {
    match &param.param_type {
        ParamType::Register(reg) => Operand::Reg(reg.atom),
        ParamType::Memory(mem) => Operand::mem(child(func, mem.ident.clone())),
    }
}

/// Location where the callee writes its result.
/// `return` is a keyword, so it cannot clash with a user-defined name.
pub fn return_operand(func: &Path, ret: &ReturnType) -> Operand {
    match ret {
        ReturnType::Register(reg) => Operand::Reg(reg.atom),
        ReturnType::Memory(_) => Operand::mem(child(func, Ident::new("return"))),
    }
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Lowerer for a function declared in `module`
    pub fn new(globals: &'g SymbolTree<'a>, module: NodeId, path: Path) -> Self {
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

    /// Lower a whole function, `bank` being the one of the enclosing bank block.
    /// Return every error found in the function if it could not be lowered.
    pub fn lower(
        mut self,
        func: &'a FuncDecl,
        bank: Option<&Ident>,
    ) -> core::result::Result<Lowered<'a>, Vec<LowError>> {
        if func.generics.is_some() {
            self.errors.push(LowError::new(
                func.span,
                LowErrorKind::Unsupported {
                    what: "generic functions",
                },
            ));
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
                self.errors.push(LowError::new(
                    func.span,
                    LowErrorKind::MissingStorage {
                        ident: func.ident.clone(),
                    },
                ));
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
    pub fn declare_signature(
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
                            .push(LowError::new(param.span, LowErrorKind::NotConstant)),
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

    /// Memory cell of a parameter or of the return value
    pub fn param_data(
        &self,
        name: Path,
        ty: &TypeUsed,
        storage: &ParamStorage,
        span: Span,
    ) -> Result<IrData> {
        Ok(IrData::new(
            name,
            self.globals.size_of(ty, self.module, &no_locals)?,
            self.globals.param_storage(storage, self.module, span)?,
        ))
    }

    pub fn push_local_data(&mut self, data: Result<IrData>) {
        match data {
            Ok(data) => self.locals.push(data),
            Err(error) => self.errors.push(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{LowError, LowErrorKind, test_utils::*, translate};
    use asmanite_common::{ident::Ident, path::Path};
    use asmanite_ir::intermediate::IrStorage;

    #[test]
    fn places_functions_explicitly_or_through_bank_blocks() {
        let program = run_translation(
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
        assert_eq!(storage("fixed"), IrStorage::Addr(0x8000));
        // inline functions produce no code of their own
        assert!(
            program
                .functions
                .iter()
                .all(|func| func.name != Path::from("expanded"))
        );
        assert_eq!(storage("banked"), IrStorage::Bank(Ident::new("prog")));
        assert_eq!(program.data[0].storage, IrStorage::Bank(Ident::new("prog")));

        let errors = translate(&parse("fn f() {}")).expect_err("translation should fail");
        assert!(matches!(
            &errors[..],
            [LowError {
                kind: LowErrorKind::MissingStorage { .. },
                ..
            }]
        ));

        let errors =
            translate(&parse("inline fn f in code () {}")).expect_err("translation should fail");
        assert!(matches!(
            &errors[..],
            [LowError {
                kind: LowErrorKind::InlinePlacement,
                ..
            }]
        ));
    }
}
