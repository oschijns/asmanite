use crate::token::{define::*, lexer::Token};
use chumsky::{
    Parser, error::Rich, extra, input::ValueInput, recursive::recursive, select, span::SimpleSpan,
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

// TODO:
// https://docs.rs/chumsky/latest/chumsky/guide/_07_tutorial/index.html

/// Read tokens to produce an expression
fn parser<'tok, 'src: 'tok, I>() -> impl Parser<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|expr| {
        let literal = select! {
            Token::Lit(lit) => Expr::Lit(lit),
        };

        literal
    })
}
