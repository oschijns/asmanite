//! Evaluation of compile-time constants

use super::{
    error::{Result, TrsError, TrsErrorKind},
    function::FuncLowerer,
    scope::LocalKind,
    symbols::{Decl, Globals},
};
use asmanite_ast::{lexer::atom::VarType, parser::expression::Expr};
use asmanite_common::num_traits::ToPrimitive;
use asmanite_common::{operator::OpType, path::Path, span::Spanned, symbol_tree::NodeId};
use asmanite_ir::inter::{Address, Int, opcode::Operand};

/// Lookup of local constants while evaluating a constant expression.
/// - `None`: the name is not a local
/// - `Some(None)`: the name is a local, but not a constant
/// - `Some(Some(value))`: the name is a local constant
pub type LocalConsts<'l> = &'l dyn Fn(&Path) -> Option<Option<Int>>;

/// Used when there is no local scope (global declarations)
pub fn no_locals(_: &Path) -> Option<Option<Int>> {
    None
}

impl<'a> Globals<'a> {
    /// Evaluate an expression at compile time
    pub fn eval_const(
        &self,
        expr: &Expr,
        module: NodeId,
        locals: LocalConsts,
        visiting: &mut Vec<NodeId>,
    ) -> Result<Int> {
        let not_const = || TrsError::new(expr.span(), TrsErrorKind::NotConstant);

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
                    .ok_or_else(|| {
                        TrsError::new(
                            var.span,
                            TrsErrorKind::Unresolved {
                                path: var.atom.clone(),
                            },
                        )
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
                    return Err(TrsError::new(
                        var.span,
                        TrsErrorKind::CyclicConstant {
                            path: symbol.path.clone(),
                        },
                    ));
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

    /// Evaluate a fixed address
    pub fn address(&self, expr: &Expr, module: NodeId, locals: LocalConsts) -> Result<Address> {
        let value = self.eval_const(expr, module, locals, &mut Vec::new())?;
        Address::try_from(value).map_err(|_| TrsError::new(expr.span(), TrsErrorKind::NotConstant))
    }
}

/// Evaluate a binary operation on constants
pub fn eval_binary(op: OpType, lhs: Int, rhs: Int) -> Option<Int> {
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
pub fn eval_unary(op: OpType, value: Int) -> Option<Int> {
    match op {
        OpType::Not => Some(Int::from(value == 0)),
        // Increments modify a place, they are not constant expressions
        OpType::Inc | OpType::Dec => None,
        _ => op.eval_unnary(value),
    }
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Value of a local constant (see [`LocalConsts`])
    pub fn local_const(&self, path: &Path) -> Option<Option<Int>> {
        self.local(path).map(|local| match &local.operand {
            Operand::Imm(value) if local.kind == LocalKind::Const => Some(*value),
            _ => None,
        })
    }

    /// Try to evaluate an expression at compile time, using local constants
    pub fn try_const(&self, expr: &Expr) -> Option<Int> {
        let locals = |path: &Path| self.local_const(path);
        self.globals
            .eval_const(expr, self.module, &locals, &mut Vec::new())
            .ok()
    }

    /// Evaluate an expression which must be constant
    pub fn require_const(&self, expr: &Expr) -> Result<Int> {
        self.try_const(expr)
            .ok_or(TrsError::new(expr.span(), TrsErrorKind::NotConstant))
    }
}
