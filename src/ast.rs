//!
//! Parser for building an AST
//!

///
pub mod expression;

///
pub mod instruction;

///
pub mod operator;

pub trait Precedence {
    fn precedence(&self) -> usize;
}
