//! Lowering of function calls

use super::{
    constant::no_locals,
    error::{Result, TrsError, TrsErrorKind},
    function::{FuncLowerer, param_operand, return_operand},
    symbols::Decl,
};
use asmanite_ast::parser::{expression::FunctionCall, function::FuncMarker};
use asmanite_ir::inter::opcode::{OpCode, Operand};

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Lower a function call.
    /// Return the location of the result, or `None` if the function returns nothing.
    pub fn lower_call(&mut self, call: &'a FunctionCall) -> Result<Option<Operand>> {
        let symbol = self.globals.resolve(self.module, &call.callee, call.span)?;
        let Decl::Func(func) = symbol.decl else {
            return Err(TrsError::new(
                call.span,
                TrsErrorKind::WrongKind {
                    path: call.callee.clone(),
                    expected: "function",
                },
            ));
        };

        let params = &func.parameters;
        let arg_count_error = || {
            TrsError::new(
                call.span,
                TrsErrorKind::ArgCount {
                    expected: params.len(),
                    found: call.args.len(),
                },
            )
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

#[cfg(test)]
mod tests {

    use asmanite_common::path::Path;

    use crate::transform::test_utils::*;
    use asmanite_ir::inter::opcode::{OpCode, Operand};

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
}
