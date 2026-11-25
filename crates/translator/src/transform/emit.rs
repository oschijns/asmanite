//! Emission of opcodes and labels in the body of the function

use super::function::FuncLowerer;
use asmanite_common::{ToBox, operator::Operator};
use asmanite_ir::{
    LabelId,
    intermediate::{
        Condition, JumpTarget, Label, OpBranch, OpCode, OpCompare, OpTransfer, Operand,
    },
};

impl<'g, 'a> FuncLowerer<'g, 'a> {
    #[inline]
    pub fn emit(&mut self, opcode: OpCode) {
        self.body.push(opcode);
    }

    #[inline]
    pub fn new_label(&mut self) -> LabelId {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    #[inline]
    pub fn place_label(&mut self, label: LabelId) {
        self.emit(OpCode::Label(Label::Local(label)));
    }

    #[inline]
    pub fn jump(&mut self, label: LabelId) {
        self.emit(OpCode::Jump(JumpTarget::Label(Label::Local(label))));
    }

    /// Move a value, unless it already is at its destination
    pub fn transfer(&mut self, dst: Operand, src: Operand) {
        if dst != src {
            self.emit(OpCode::Transfer(OpTransfer { dst, src }.to_box()));
        }
    }

    pub fn branch(&mut self, cond: Condition, label: LabelId) {
        let target = JumpTarget::Label(Label::Local(label));
        self.emit(OpCode::Branch(OpBranch { cond, target }.to_box()));
    }

    pub fn compare(&mut self, op: Operator, lhs: Operand, rhs: Operand, label: LabelId) {
        self.branch(
            Condition::Compare(OpCompare { op, lhs, rhs }.to_box()),
            label,
        );
    }
}
