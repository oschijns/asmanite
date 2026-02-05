//!

use crate::ast::{base::*, expression::*};
use chumsky::prelude::*;

/// Assignement instruction with an optional binary operation
pub struct Assign<'src> {
    /// Assignment combined with a binary operation
    pub operation: Option<AssignOp>,

    /// Variable being assigned to
    pub left: String,

    /// Operand to evaluate
    pub right: Expr<'src>,
}
