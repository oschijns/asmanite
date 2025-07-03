//!

use crate::ast::{expression::*, operator::*};
use chumsky::prelude::*;

/// Assignement instruction with an optional binary operation
pub struct Assign {
    /// Assignment combined with a binary operation
    pub operation: Option<BinaryOp>,

    /// Variable being assigned to
    pub left: String,

    /// Operand to evaluate
    pub right: Operand,
}
