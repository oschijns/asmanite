//!

use crate::ast::operator::*;
use chumsky::prelude::*;

/// Boolean operand
#[derive(Clone, Debug)]
pub enum BoolOperand {
    /// Literal value
    Literal(bool),

    /// Identifier
    Ident(String),

    /// Negation: !a
    Neg(Box<BoolOperand>),

    /// Sub expression
    Logic {
        /// ==, !=, &&, ||
        operator: BoolOp,

        /// a ? _
        left: Box<BoolOperand>,

        /// _ ? b
        right: Box<BoolOperand>,
    },

    /// Sub expression
    Compare {
        /// ==, !=, <, <=, >, >=
        operator: CompOp,

        /// a ? _
        left: Box<NumOperand>,

        /// _ ? b
        right: Box<NumOperand>,
    },
}

/// Integer operand
#[derive(Clone, Debug)]
pub enum NumOperand {
    /// Literal value
    Literal(i64),

    /// Identifier
    Ident(String),

    /// Sub expression
    Unary {
        /// ~, -, ++, --
        operator: UnaryOp,

        /// ? a
        element: Box<NumOperand>,
    },

    /// Sub expression
    Binary {
        /// &, |, ^,
        /// +, +', -, -', *, /, %,
        /// <<, >>, >>>, <+, <+', +>, +>'
        operator: BinaryOp,

        /// a ? _
        left: Box<NumOperand>,

        /// _ ? b
        right: Box<NumOperand>,
    },
}
