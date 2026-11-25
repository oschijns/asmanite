//! Lowering of expressions onto operands, without temporaries

use super::{
    error::{Result, TrsError, TrsErrorKind, unsupported},
    function::FuncLowerer,
};
use crate::symbol::{SymbolType, no_locals};
use asmanite_ast::parser::{composite_type::TypeUsed, expression::Expr};
use asmanite_common::{ToBox, operator::OpType, span::Spanned};
use asmanite_ir::intermediate::{OpBinary, OpCode, OpUnary, Operand};

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Lower an expression which must map onto a single operand.
    ///
    /// Assignments and calls are operands: their value is their destination
    /// (`x = a = 1` stores 1 in `a`, then `a` in `x`).
    pub fn lower_operand(&mut self, expr: &'a Expr) -> Result<Operand> {
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
                _ => Err(TrsError::new(expr.span(), TrsErrorKind::NotAnOperand)),
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
            Expr::Call(call) => self.lower_call(call)?.ok_or_else(|| {
                TrsError::new(
                    call.span,
                    TrsErrorKind::NoReturnValue {
                        path: call.callee.clone(),
                    },
                )
            }),
            // Status flags, comparisons and other operations need a destination
            Expr::Status(_) | Expr::Binary(_) => {
                Err(TrsError::new(expr.span(), TrsErrorKind::NotAnOperand))
            }
        }
    }

    /// Write the value of an expression into `dst`.
    /// Besides operands, the expression can be a single unary or binary
    /// operation on operands (`dst = -x`, `dst = x + 1`).
    pub fn lower_into(&mut self, dst: Operand, expr: &'a Expr) -> Result<()> {
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
                OpType::Not => Err(TrsError::new(expr.span(), TrsErrorKind::NotAnOperand)),
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
                    return Err(TrsError::new(expr.span(), TrsErrorKind::NotAnOperand));
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
    pub fn lower_place(&mut self, expr: &'a Expr) -> Result<Operand> {
        let not_assignable = || TrsError::new(expr.span(), TrsErrorKind::NotAssignable);
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
                let elem_size = self.element_size(&index.expr)?;

                if let Some(value) = self.try_const(&index.index) {
                    let offset = i64::try_from(value).map_err(|_| {
                        TrsError::new(index.index.span(), TrsErrorKind::NotConstant)
                    })?;
                    cell.offset += offset * i64::from(elem_size);
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
    pub fn element_size(&self, expr: &Expr) -> Result<u32> {
        let (ty, module) = match expr {
            Expr::Var(var) => match self.local(&var.atom) {
                Some(local) => (local.ty, self.module),
                None => {
                    let symbol = self.globals.resolve(self.module, &var.atom, var.span)?;
                    match symbol.symbol_type {
                        SymbolType::Var(decl) => {
                            (Some(&decl.type_used), self.globals.module_of(symbol))
                        }
                        _ => (None, self.globals.module_of(symbol)),
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
}

#[cfg(test)]
mod tests {
    use crate::transform::test_utils::*;
    use crate::transform::{TrsError, TrsErrorKind, transform};
    use asmanite_common::{ToBox, operator::op};
    use asmanite_ir::intermediate::{OpCode, OpUnary, Operand};

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
                binary(op::ADD, reg("a"), mem("f::v"), Operand::Imm(1u64.into())),
                binary(op::ADD, reg("a"), reg("a"), Operand::Imm(2u64.into())),
                transfer(reg("y"), Operand::Imm(3u64.into())),
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
                    TrsError {
                        kind: TrsErrorKind::NotAnOperand,
                        ..
                    },
                    TrsError {
                        kind: TrsErrorKind::NotAnOperand,
                        ..
                    },
                    TrsError {
                        kind: TrsErrorKind::NotAnOperand,
                        ..
                    },
                    TrsError {
                        kind: TrsErrorKind::NoEffect,
                        ..
                    },
                ]
            ),
            "unexpected errors: {errors:?}"
        );
    }
}
