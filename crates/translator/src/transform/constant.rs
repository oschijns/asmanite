//! Evaluation of compile-time constants

use super::{
    error::{Result, TrsError, TrsErrorKind},
    function::FuncLowerer,
    scope::LocalKind,
    symbols::{Decl, Globals},
};
use asmanite_ast::{lexer::atom::VarType, parser::expression::Expr};
use asmanite_common::{
    literal::Literal, number::Number, operator::OpType, path::Path, span::Spanned,
    symbol_tree::NodeId,
};
use asmanite_ir::{Addr, intermediate::Operand};

/// Evaluation of constant expression
#[derive(Debug, Default, Clone, Copy)]
pub enum SymbolValue {
    /// Not a local
    #[default]
    None,

    /// Local symbol but not a constant
    Local,

    /// Local constant
    LocalConst(Number),
}

/// Lookup of local constants while evaluating a constant expression
/// (see [`SymbolValue`])
pub type LocalConsts<'l> = &'l dyn Fn(&Path) -> SymbolValue;

/// Used when there is no local scope (global declarations)
pub fn no_locals(_: &Path) -> SymbolValue {
    SymbolValue::None
}

impl<'a> Globals<'a> {
    /// Evaluate an expression at compile time
    pub fn eval_const(
        &self,
        expr: &Expr,
        module: NodeId,
        locals: LocalConsts,
        visiting: &mut Vec<NodeId>,
    ) -> Result<Number> {
        let not_const = || TrsError::new(expr.span(), TrsErrorKind::NotConstant);

        match expr {
            Expr::Lit(lit) => eval_literal(&lit.atom).ok_or_else(not_const),
            Expr::Group(group) => self.eval_const(&group.atom, module, locals, visiting),
            // TODO: truncate the value to the size of the primitive
            Expr::Cast(cast) => self.eval_const(&cast.expr, module, locals, visiting),
            Expr::Var(var) => {
                match locals(&var.atom) {
                    SymbolValue::LocalConst(value) => return Ok(value),
                    SymbolValue::Local => return Err(not_const()),
                    SymbolValue::None => {}
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
    pub fn address(&self, expr: &Expr, module: NodeId, locals: LocalConsts) -> Result<Addr> {
        let value = self.eval_const(expr, module, locals, &mut Vec::new())?;
        Addr::try_from(value).map_err(|_| TrsError::new(expr.span(), TrsErrorKind::NotConstant))
    }
}

/// Evaluate a literal
#[rustfmt::skip]
fn eval_literal(lit: &Literal) -> Option<Number> {
    let value = match lit {
        Literal::Bool (lit) => Number::from(u8::from(lit.0)),
        Literal::Char (lit) => Number::from(u32::from(lit.0)),
        Literal::Int  (lit) => Number::from(u64::try_from(lit.0).ok()?),
        Literal::Float(lit) => Number::from(lit.0),
        Literal::Str  ( _ ) => { return None; }
    };
    Some(value)
}

/// Truth value of a constant (non-zero)
pub fn is_true(value: Number) -> bool {
    match value {
        Number::UInt(uint) => uint != 0,
        Number::Int(int) => int != 0,
        Number::Float(float) => float != 0.0,
    }
}

/// Operands of a binary operation, promoted to a common representation:
/// floats win over signed integers, which win over unsigned integers.
enum Promoted {
    UInt(u64, u64),
    Int(i64, i64),
    Float(f64, f64),
}

impl Promoted {
    fn new(lhs: Number, rhs: Number) -> Self {
        match (lhs, rhs) {
            (Number::UInt(lhs), Number::UInt(rhs)) => Self::UInt(lhs, rhs),
            (Number::Float(_), _) | (_, Number::Float(_)) => {
                Self::Float(as_float(lhs), as_float(rhs))
            }
            _ => Self::Int(as_int(lhs), as_int(rhs)),
        }
    }
}

/// Convert a number into a signed integer (wrapping)
fn as_int(value: Number) -> i64 {
    match value {
        Number::UInt(uint) => uint as i64,
        Number::Int(int) => int,
        Number::Float(float) => float as i64,
    }
}

/// Convert a number into a floating point number
fn as_float(value: Number) -> f64 {
    match value {
        Number::UInt(uint) => uint as f64,
        Number::Int(int) => int as f64,
        Number::Float(float) => float,
    }
}

/// Evaluate a binary operation on constants
pub fn eval_binary(op: OpType, lhs: Number, rhs: Number) -> Option<Number> {
    let compare = match Promoted::new(lhs, rhs) {
        Promoted::UInt(lhs, rhs) => op.eval_compare(lhs, rhs),
        Promoted::Int(lhs, rhs) => op.eval_compare(lhs, rhs),
        Promoted::Float(lhs, rhs) => op.eval_compare(lhs, rhs),
    };
    if let Some(result) = compare.or_else(|| op.eval_boolean(is_true(lhs), is_true(rhs))) {
        return Some(Number::from(u8::from(result)));
    }
    match op {
        // TODO: rotations depend on the width of the value,
        // which requires tracking the primitive type of the constants
        OpType::RotL | OpType::RotR => None,
        OpType::ShL | OpType::ShRA | OpType::ShRL => {
            let amount = match rhs {
                Number::Float(_) => return None,
                _ => u32::try_from(rhs).ok()?,
            };
            match lhs {
                Number::UInt(lhs) => op.eval_bitshift(lhs, amount).map(Number::from),
                Number::Int(lhs) => op.eval_bitshift(lhs, amount).map(Number::from),
                Number::Float(_) => None,
            }
        }
        _ => match Promoted::new(lhs, rhs) {
            // `1 - 2` is negative: switch to signed integers
            Promoted::UInt(lhs, rhs) if op == OpType::Sub && lhs < rhs => {
                Some(Number::from((lhs as i64).wrapping_sub(rhs as i64)))
            }
            Promoted::UInt(lhs, rhs) => op.eval_binary(lhs, rhs).map(Number::from),
            Promoted::Int(lhs, rhs) => op.eval_binary(lhs, rhs).map(Number::from),
            Promoted::Float(lhs, rhs) => eval_float_binary(op, lhs, rhs).map(Number::from),
        },
    }
}

/// Evaluate an arithmetic operation on floating point numbers.
/// Returns `None` for bitwise operators and divisions by zero.
#[rustfmt::skip]
fn eval_float_binary(op: OpType, lhs: f64, rhs: f64) -> Option<f64> {
    let result = match op {
        OpType::Add => lhs + rhs,
        OpType::Sub => lhs - rhs,
        OpType::Mul => lhs * rhs,
        OpType::Div if rhs != 0.0 => lhs / rhs,
        OpType::Rem if rhs != 0.0 => lhs % rhs,
        _ => { return None; }
    };
    Some(result)
}

/// Evaluate a unary operation on a constant
pub fn eval_unary(op: OpType, value: Number) -> Option<Number> {
    match (op, value) {
        (OpType::Not, _) => Some(Number::from(u8::from(!is_true(value)))),
        // Increments modify a place, they are not constant expressions
        (OpType::Inc | OpType::Dec, _) => None,
        // Negating an unsigned integer gives a signed one
        (OpType::Sub, Number::UInt(uint)) => Some(Number::from((uint as i64).wrapping_neg())),
        (_, Number::UInt(uint)) => op.eval_unnary(uint).map(Number::from),
        (_, Number::Int(int)) => op.eval_unnary(int).map(Number::from),
        (OpType::Add, Number::Float(_)) => Some(value),
        (OpType::Sub, Number::Float(float)) => Some(Number::from(-float)),
        (_, Number::Float(_)) => None,
    }
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Value of a local constant (see [`LocalConsts`])
    pub fn local_const(&self, path: &Path) -> SymbolValue {
        match self.local(path) {
            None => SymbolValue::None,
            Some(local) => match &local.operand {
                Operand::Imm(value) if local.kind == LocalKind::Const => {
                    SymbolValue::LocalConst(*value)
                }
                _ => SymbolValue::Local,
            },
        }
    }

    /// Try to evaluate an expression at compile time, using local constants
    pub fn try_const(&self, expr: &Expr) -> Option<Number> {
        let locals = |path: &Path| self.local_const(path);
        self.globals
            .eval_const(expr, self.module, &locals, &mut Vec::new())
            .ok()
    }

    /// Evaluate an expression which must be constant
    pub fn require_const(&self, expr: &Expr) -> Result<Number> {
        self.try_const(expr)
            .ok_or(TrsError::new(expr.span(), TrsErrorKind::NotConstant))
    }
}
