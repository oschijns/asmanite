use crate::lexer::define::*;

/// Expression AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal
    Lit(Literal),

    /// Path to variable identifier
    Var(Path),

    /// Unary operation
    Unary(UnaryOp),

    /// Binary operation
    Binary(BinaryOp),

    /// Assign operation
    Assign(AssignOp),

    /// Function call
    Call(FunctionCall),

    /// Parenthesized expression
    Group(Box<Self>),
}

/// Unary operation
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOp {
    /// Operator applied to the expression
    pub op: Operator,

    /// Operand
    pub expr: Box<Expr>,
}

impl UnaryOp {
    #[inline]
    pub fn new(op: Operator, expr: Expr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

/// Binary operation
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    /// Operator between the operands
    pub op: Operator,

    /// Left operand
    pub lhs: Box<Expr>,

    /// Right operand
    pub rhs: Box<Expr>,
}

impl BinaryOp {
    #[inline]
    pub fn new(op: Operator, lhs: Expr, rhs: Expr) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

/// Assign operation
#[derive(Debug, Clone, PartialEq)]
pub struct AssignOp {
    /// Operator between the operands
    pub assign: Assign,

    /// Left operand
    pub lhs: Box<Expr>,

    /// Right operand
    pub rhs: Box<Expr>,
}

impl AssignOp {
    #[inline]
    pub fn new(assign: Assign, lhs: Expr, rhs: Expr) -> Self {
        Self {
            assign,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

/// Function call
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Function being called
    pub callee: Path,

    /// Call arguments
    pub args: Box<[Expr]>,
}

impl FunctionCall {
    #[inline]
    pub fn new(callee: Path, args: Vec<Expr>) -> Self {
        Self {
            callee,
            args: args.into_boxed_slice(),
        }
    }
}
