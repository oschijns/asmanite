//! Expansion of the inline functions at their call sites

use super::{
    error::{LowError, LowErrorKind, LowResult},
    function::{FuncLowerer, child, is_comptime, param_operand},
    statement::collect_labels,
};
use crate::symbol::SymbolData;
use asmanite_ast::parser::function::FuncDecl;
use asmanite_common::{ident::Ident, span::Span};
use asmanite_ir::intermediate::Operand;
use std::collections::HashMap;

impl<'g, 'a, 's> FuncLowerer<'g, 'a, 's> {
    /// Replace a call to an inline function by the body of the function.
    /// The runtime arguments are written into the parameters like for a call.
    pub fn expand_inline(
        &mut self,
        symbol: &'g SymbolData<'a>,
        func: &'a FuncDecl,
        args: Vec<Operand>,
        span: Span,
    ) -> LowResult<Option<Operand>> {
        if self.shared.inline_stack.contains(symbol.path()) {
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

            // Prefix of the user labels of this expansion
            let name = symbol
                .path()
                .as_slice()
                .last()
                .expect("a path is never empty");
            let label_path = child(
                &self.label_path,
                Ident::new(&format!("{name}.{}", self.shared.inline_count)),
            );
            self.shared.inline_count += 1;

            // The inline function is lowered on its own, like a function,
            // but its code and labels go into the output of the caller
            let mut callee = FuncLowerer::new(
                self.globals,
                &mut *self.shared,
                self.globals.module_of(symbol),
                symbol.path().clone(),
            );
            callee.label_path = label_path;
            let ret = callee.lower_inline_body(func, &args);

            // Its memory cells are reserved once for every expansion
            let cells = callee.locals;
            self.shared.inline_data.extend(cells);
            Ok(ret)
        }
    }

    /// Lower the body of an inline function, `args` being the arguments of the call.
    /// `return` jumps right after the expansion.
    /// Return where the result of the call is.
    fn lower_inline_body(&mut self, func: &'a FuncDecl, args: &[Operand]) -> Option<Operand> {
        self.shared.inline_stack.push(self.path.clone());
        self.scopes.push(HashMap::new());
        self.declare_signature(func, Some(args));
        collect_labels(&func.block, &mut self.labels);
        self.exit = self.new_label();
        self.lower_block(&func.block);
        self.place_label(self.exit);
        self.shared.inline_stack.pop();
        self.ret.take()
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
