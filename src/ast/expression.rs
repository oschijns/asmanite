//! AST for expressions

use crate::ast::base::*;
use chumsky::prelude::*;
use std::fmt;

/// Expression AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    /// Literal value
    Lit(PrimLit),

    /// Variable
    Var(IdentPath<'src>),

    /// Function call
    Call(CallExpr<'src>),

    /// Cast operation
    Cast(CastExpr<'src>),

    /// Unary operation
    Unary(UnaryExpr<'src>),

    /// Binary expression
    Binary(BinaryExpr<'src>),
}

/// Function call expression
#[derive(Debug, Clone, PartialEq)]
pub struct CallExpr<'src> {
    /// Function name
    pub(crate) name: IdentPath<'src>,

    /// Parameters
    pub(crate) parameters: Vec<Expr<'src>>,
}

/// Cast expression
#[derive(Debug, Clone, PartialEq)]
pub struct CastExpr<'src> {
    /// a as ?
    pub(crate) element: Box<Expr<'src>>,

    /// primitive type
    pub(crate) prim_type: PrimType,
}

/// Unary operation expression
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpr<'src> {
    /// ~, -, ++, --
    pub(crate) operator: UnaryOp,

    /// ? a
    pub(crate) element: Box<Expr<'src>>,
}

/// Binary expression
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpr<'src> {
    /// ==, !=, <, <=, >, >=
    /// &&, ||
    /// &, |, ^,
    /// +, +', -, -', *, /, %,
    /// <<, >>, >>>, <+, <+', +>, +>'
    pub(crate) operator: BinaryOp,

    /// a ? _
    pub(crate) left: Box<Expr<'src>>,

    /// _ ? b
    pub(crate) right: Box<Expr<'src>>,
}

impl fmt::Display for Expr<'_> {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Lit   (lit)    => write!(f, "{lit}"   ),
            Expr::Var   (var)    => write!(f, "{var}"   ),
            Expr::Call  (call)   => write!(f, "{call}"  ),
            Expr::Cast  (cast)   => write!(f, "{cast}"  ),
            Expr::Unary (unary)  => write!(f, "{unary}" ),
            Expr::Binary(binary) => write!(f, "{binary}"),
        }
    }
}

impl fmt::Display for CallExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let _ = write!(f, "{}(", self.name)?;
        for p in self.parameters.iter() {
            let _ = write!(f, "{},", p)?;
        }
        write!(f, ")")
    }
}

impl fmt::Display for CastExpr<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} as {}", self.element, self.prim_type)
    }
}

impl fmt::Display for UnaryExpr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = self.operator;
        match op {
            UnaryOp::PostIncr | UnaryOp::PostDecr => write!(f, "({}){}", self.element, op),
            _ => write!(f, "{}({})", op, self.element),
        }
    }
}

impl fmt::Display for BinaryExpr<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}){}({})", self.left, self.operator, self.right)
    }
}
