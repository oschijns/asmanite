//! Lowering of the statements and of the structured control flow
//! (blocks, `if`, `switch`, loops, `break`/`continue`, `goto`, `return`)

use super::{
    error::{Result, TrsError, TrsErrorKind, unsupported},
    function::{FuncLowerer, child},
    scope::LocalKind,
};
use asmanite_ast::{
    lexer::atom::Range,
    parser::{
        expression::Expr,
        memory::ParamType,
        statement::{IfElse, Loop, LoopCtrlType, LoopType, Statement, StmtInner, Switch},
    },
};
use asmanite_common::{
    ToBox,
    ident::Ident,
    operator::{OpType, op},
    span::Spanned,
};
use asmanite_ir::inter::opcode::{JumpTarget, Label, LabelId, OpBinary, OpCode, Operand};
use std::collections::{HashMap, HashSet};

/// Loop currently being lowered, targeted by `break` and `continue`
pub struct LoopFrame {
    pub(super) label: Option<Ident>,
    pub(super) brk: LabelId,
    pub(super) cont: LabelId,
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    pub fn lower_block(&mut self, block: &'a [Statement]) {
        self.scopes.push(HashMap::new());
        for statement in block {
            if let Err(error) = self.lower_statement(statement) {
                self.errors.push(error);
            }
        }
        self.scopes.pop();
    }

    pub fn lower_statement(&mut self, statement: &'a Statement) -> Result<()> {
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
                        .ok_or_else(|| {
                            TrsError::new(
                                ctrl.span,
                                TrsErrorKind::UnknownLabel {
                                    label: label.clone(),
                                },
                            )
                        })?,
                    None => self
                        .loops
                        .last()
                        .ok_or(TrsError::new(ctrl.span, TrsErrorKind::OutsideLoop))?,
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
                    return Err(TrsError::new(
                        goto.span,
                        TrsErrorKind::UnknownLabel {
                            label: goto.label.clone(),
                        },
                    ));
                }
                let path = child(&self.label_path, goto.label.clone());
                self.emit(OpCode::Jump(JumpTarget::Label(Label::Named(path))));
                Ok(())
            }
            StmtInner::Return(ret) => {
                if let Some(value) = &ret.value {
                    let Some(dst) = self.ret.clone() else {
                        return Err(TrsError::new(
                            ret.span,
                            TrsErrorKind::NoReturnValue {
                                path: self.path.clone(),
                            },
                        ));
                    };
                    self.lower_into(dst, value)?;
                }
                self.jump(self.exit);
                Ok(())
            }
        }
    }

    /// Lower an expression used as a statement: it must do something
    pub fn lower_instr(&mut self, expr: &'a Expr) -> Result<()> {
        match expr {
            Expr::Group(group) => self.lower_instr(&group.atom),
            Expr::Assign(_) => self.lower_operand(expr).map(drop),
            // The result of the call, if any, is ignored
            Expr::Call(call) => self.lower_call(call).map(drop),
            Expr::Unary(unary) if matches!(unary.op.op_type, OpType::Inc | OpType::Dec) => {
                self.lower_operand(expr).map(drop)
            }
            _ => Err(TrsError::new(expr.span(), TrsErrorKind::NoEffect)),
        }
    }

    pub fn lower_if(&mut self, if_else: &'a IfElse) -> Result<()> {
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
    pub fn lower_switch(&mut self, switch: &'a Switch) -> Result<()> {
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

    pub fn lower_loop(&mut self, loop_stmt: &'a Loop, label: Option<Ident>) -> Result<()> {
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

    pub fn lower_loop_body(
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
}

/// Collect every user label of a function, including those in nested blocks
pub fn collect_labels(block: &[Statement], labels: &mut HashSet<Ident>) {
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

#[cfg(test)]
mod tests {

    use asmanite_common::{ToBox, ident::Ident, operator::op, path::Path};

    use crate::transform::test_utils::*;
    use asmanite_ir::inter::{
        IrData, IrStorage,
        opcode::{OpBinary, OpCode, Operand},
    };

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
}
