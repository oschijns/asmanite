//!

use crate::ast::operator::*;
use chumsky::prelude::*;

/// Integer operand
#[derive(Clone, Debug)]
pub enum Operand {
    /// Literal value
    Literal(i64),

    /// Identifier
    Ident(String),

    /// Sub expression
    Unary {
        /// ~, -, ++, --
        operator: UnaryOp,

        /// ? a
        element: Box<Operand>,
    },

    /// Sub expression
    Binary {
        /// ==, !=, <, <=, >, >=
        /// &&, ||
        /// &, |, ^,
        /// +, +', -, -', *, /, %,
        /// <<, >>, >>>, <+, <+', +>, +>'
        operator: BinaryOp,

        /// a ? _
        left: Box<Operand>,

        /// _ ? b
        right: Box<Operand>,
    },
}
