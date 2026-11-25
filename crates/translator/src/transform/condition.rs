//! Lowering of conditions into conditional branches (with short-circuit)

use super::{error::Result, function::FuncLowerer};
use asmanite_ast::parser::expression::Expr;
use asmanite_common::operator::{OpType, Operator, op};
use asmanite_ir::{
    LabelId,
    intermediate::{Condition, Operand},
};

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Branch to `target` when `expr` evaluates to `when`, fall through otherwise.
    /// `&&` and `||` are short-circuited.
    pub fn lower_cond(&mut self, expr: &'a Expr, when: bool, target: LabelId) -> Result<()> {
        if let Some(value) = self.try_const(expr) {
            if value.is_true() == when {
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
                self.compare(op, value, Operand::Imm(0u64.into()), target);
                Ok(())
            }
        }
    }
}

/// Comparison that holds exactly when the given one does not
pub fn negate(op: Operator) -> Operator {
    let op_type = op.op_type.inverse_comparison().unwrap_or(op.op_type);
    Operator { op_type, ..op }
}

#[cfg(test)]
mod tests {

    use asmanite_common::{operator::op, path::Path};

    use crate::transform::test_utils::*;
    use asmanite_ir::intermediate::{Label, OpCode, Operand};

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
                branch_cmp(op::G_EQ, reg("a"), Operand::Imm(10u64.into()), 4),
                branch_cmp(op::EQ, reg("x"), Operand::Imm(0u64.into()), 4),
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
}
