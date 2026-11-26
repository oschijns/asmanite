//! Expansion of the inline functions at their call sites

use super::{
    error::{LowError, LowErrorKind, LowResult},
    function::{FuncLowerer, child, is_comptime, param_operand},
    scope::Local,
    statement::{LoopFrame, collect_labels},
};
use crate::symbol::{NodeId, SymbolData};
use asmanite_ast::parser::function::FuncDecl;
use asmanite_common::{ident::Ident, path::Path, span::Span};
use asmanite_ir::{
    LabelId,
    intermediate::{IrData, Operand},
};
use std::collections::{HashMap, HashSet};

/// State of the function being lowered, saved while an inline function is expanded
pub struct Context<'a> {
    /// Module of the caller, from which its names resolve
    pub(crate) module: NodeId,

    /// Absolute path of the caller (prefix of its parameters and locals)
    pub(crate) path: Path,

    /// Prefix of the user labels of the caller
    pub(crate) label_path: Path,

    /// Lexical scopes of the caller, hidden from the body of the inline function
    pub(crate) scopes: Vec<HashMap<Ident, Local<'a>>>,

    /// User labels defined in the caller
    pub(crate) labels: HashSet<Ident>,

    /// Loops of the caller enclosing the call: `break` and `continue`
    /// in the inline function cannot target them
    pub(crate) loops: Vec<LoopFrame>,

    /// Where the return value of the caller goes
    pub(crate) ret: Option<Operand>,

    /// Label placed right before the final `Return` of the caller
    pub(crate) exit: LabelId,

    /// Memory cells owned by the caller. The cells declared while expanding
    /// the inline function go into the program data instead.
    pub(crate) locals: Vec<IrData>,

    /// Is the caller the startup routine ?
    pub(crate) startup: bool,
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Replace a call to an inline function by the body of the function.
    /// The runtime arguments are written into the parameters like for a call.
    pub fn expand_inline(
        &mut self,
        symbol: &'g SymbolData<'a>,
        func: &'a FuncDecl,
        args: Vec<Operand>,
        span: Span,
    ) -> LowResult<Option<Operand>> {
        if self.inline_stack.contains(symbol.path()) {
            Err(LowError::rec_inline(span, symbol.path().clone()))
        } else if func.generics.is_some() {
            Err(LowError::unsupported(func.span, "generic functions"))
        } else {
            // Compile-time parameters need a constant argument
            for (param, arg) in func.parameters.iter().zip(&args) {
                if is_comptime(param) && !matches!(arg, Operand::Imm(_)) {
                    return Err(LowError::new(span, LowErrorKind::NotConstant));
                }
            }
            for (param, arg) in func.parameters.iter().zip(&args) {
                if !is_comptime(param) {
                    self.transfer(param_operand(symbol.path(), param), arg.clone());
                }
            }

            // Switch to the context of the inline function
            let name = symbol
                .path()
                .as_slice()
                .last()
                .expect("a path is never empty");
            let label_path = child(
                &self.label_path,
                Ident::new(&format!("{name}.{}", self.inline_count)),
            );
            self.inline_count += 1;

            // TODO: REWORK

            let caller = Context {
                module: core::mem::replace(&mut self.module, self.globals.module_of(symbol)),
                path: core::mem::replace(&mut self.path, symbol.path().clone()),
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
            self.inline_stack.push(symbol.path().clone());
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
    }
}

#[cfg(test)]
mod tests {
    use crate::{LowError, LowErrorKind, test_utils::*, translate};
    use asmanite_common::{ident::Ident, operator::op, path::Path};
    use asmanite_ir::intermediate::{IrData, IrStorage, Label, OpCode, Operand};

    #[test]
    fn expands_inline_functions_at_each_call_site() {
        let program = run_translation(
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
                transfer(reg("x"), Operand::Imm(5u64.into())),
                transfer(reg("a"), mem("add_to::v")),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(1u64.into())),
                OpCode::Label(Label::Named(Path::from("main::add_to.0::done"))),
                jump(1),
                label(1),
                transfer(mem("main::total"), reg("a")),
                transfer(mem("add_to::v"), mem("main::total")),
                transfer(reg("x"), Operand::Imm(6u64.into())),
                transfer(reg("a"), mem("add_to::v")),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(2u64.into())),
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
            vec![IrData::new(
                Path::from("add_to::v"),
                1,
                IrStorage::Bank(Ident::new("zp"))
            )]
        );
    }

    #[test]
    fn expands_nested_inline_functions() {
        let program = run_translation(
            "inline fn inner() { a += 1; }
             inline fn outer() { inner(); inner(); }
             fn main in code () { outer(); }",
        );

        let increment = binary(op::ADD, reg("a"), reg("a"), Operand::Imm(1u64.into()));
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
        let errors = translate(&parse(
            "inline fn ping() { pong(); }
             inline fn pong() { ping(); }
             fn main in code () { ping(); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [LowError { kind: LowErrorKind::RecursiveInline { path, .. }, .. }]
                if *path == Path::from("ping")),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn reports_errors_of_inline_functions_once() {
        let errors = translate(&parse(
            "inline fn bad() { unknown = 1; }
             fn main in code () { bad(); bad(); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(
                &errors[..],
                [LowError {
                    kind: LowErrorKind::Unresolved { .. },
                    ..
                }]
            ),
            "unexpected errors: {errors:?}"
        );
    }

    #[test]
    fn requires_constant_arguments_for_compile_time_parameters() {
        let errors = translate(&parse(
            "inline fn shift(let count: u8) { a <<= count; }
             fn main in code (v: u8 in zp) { shift(v); }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(
                &errors[..],
                [LowError {
                    kind: LowErrorKind::NotConstant,
                    ..
                }]
            ),
            "unexpected errors: {errors:?}"
        );
    }
}
