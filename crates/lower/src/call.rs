//! Lowering of function calls

use crate::{
    error::{LowError, LowResult},
    function::{FuncLowerer, param_operand, return_operand},
    symbol::{SymbolType, no_locals},
};
use asmanite_ast::parser::{expression::FunctionCall, function::FuncMarker};
use asmanite_ir::intermediate::{OpCode, Operand};

impl<'g, 'a, 's> FuncLowerer<'g, 'a, 's> {
    /// Lower a function call.
    /// Return the location of the result, or `None` if the function returns nothing.
    pub fn lower_call(&mut self, call: &'a FunctionCall) -> LowResult<Option<Operand>> {
        let symbol = self.globals.resolve(self.module, &call.callee, call.span)?;
        let SymbolType::Func(func) = symbol.symbol_type else {
            return Err(LowError::wrong_kind(
                call.span,
                call.callee.clone(),
                "function",
            ));
        };

        let params = &func.parameters;
        if call.args.len() > params.len() {
            return Err(LowError::arg_count(
                call.span,
                params.len(),
                call.args.len(),
            ));
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
                    self.globals.module_of(symbol),
                    &no_locals,
                    &mut Vec::new(),
                )?),
                (None, None) => {
                    return Err(LowError::arg_count(
                        call.span,
                        params.len(),
                        call.args.len(),
                    ));
                }
            };
            values.push(value);
        }

        if func.marker == FuncMarker::Inline {
            return self.expand_inline(symbol, func, values, call.span);
        }

        for (param, value) in params.iter().zip(values) {
            self.transfer(param_operand(symbol.path(), param), value);
        }
        self.emit(OpCode::Call(symbol.path().clone()));

        // The result stays where the callee wrote it
        Ok(func
            .return_type
            .as_ref()
            .map(|ret| return_operand(symbol.path(), ret)))
    }
}

#[cfg(test)]
mod tests {
    use crate::test_utils::*;
    use asmanite_common::path::Path;
    use asmanite_ir::intermediate::{OpCode, Operand};

    #[test]
    fn lowers_calls_with_default_arguments() {
        let program = run_translation(
            "fn add in code (a, x = 1) -> a { a += x; return a; }
             fn main in code () { var total: u8 in ram; total = add(2); }",
        );
        let func = function(&program, "main");

        assert_eq!(
            func.body,
            vec![
                transfer(reg("a"), Operand::Imm(2u64.into())),
                transfer(reg("x"), Operand::Imm(1u64.into())),
                OpCode::Call(Path::from("add")),
                // the result is read from the return register of `add`
                transfer(mem("main::total"), reg("a")),
                label(0),
                OpCode::Return,
            ]
        );
    }
}
