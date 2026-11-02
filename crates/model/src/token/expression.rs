use crate::token::{define::*, lexer::Token};
use tokora::{
    Emitter, InputRef, ParseContext, ParseInput,
    lexer::LogosLexer,
    parser::{PrattInfix, PrattLHS, PrattRHS, Precedenced, pratt},
    token::PrattToken,
};

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

/// Function call
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Function being called
    pub callee: Path,

    /// Call arguments
    pub args: Box<[Expr]>,
}
