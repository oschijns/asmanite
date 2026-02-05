//!
//! Parser for building an AST
//!

///
pub mod expression;

///
pub mod statement;

///
pub mod base;

/// Identifier
#[derive(Debug, Clone)]
pub struct Ident(String);

pub trait Precedence {
    fn precedence(&self) -> usize;
}
