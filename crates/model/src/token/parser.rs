/*
 * TODO:
 * https://codeberg.org/zesterer/chumsky/src/branch/main/examples/logos.rs
 *
 * identify scopes/groups using pair of delimiters (), [], {}
 *
 */

use crate::token::{define::*, lexer::Token};

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal
    Lit(Literal),

    /// Operator
    Op(Operator),

    /// Identifier
    Ident(Ident),

    /// Sub expression
    Group {
        /// Delimitiers of the group
        delim_type: DelimType,

        /// Content of the group
        inner: Box<[Self]>,
    },
}
