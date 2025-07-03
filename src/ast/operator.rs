//!

use chumsky::prelude::*;

use crate::ast::Ident;

/*
    Operator/Expression	           | Associativity
    -------------------------------|--------------
    Paths                          |
    Method calls                   |
    Field expressions              | left to right
    Function calls, array indexing |
    Unary ~ - * ! &                |
    as                             | left to right
    * / %                          | left to right
    + +' - -'                      | left to right
    << >> >>> <+ <+' +> +>'        | left to right
    & \| ^                         | left to right
    == != < > <= >=                | Require parentheses
    && \|\|                        | left to right
    .. ..=                         | Require parentheses
    all assignment variants        | right to left
    return break continue goto     |
*/

/// Unary operator over a numerical value
#[derive(Debug, Clone)]
pub enum UnaryOp {
    /// !a
    Not,

    /// ~a
    Inv,

    /// -a
    Neg,

    /// ++a
    PreIncr,

    /// --a
    PreDecr,

    /// a++
    PostIncr,

    /// a--
    PostDecr,

    /// a as b
    Cast(Ident),
}

/// Binary operator over two numerical values
#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    /// a == b
    Equal,

    /// a != b
    Diff,

    /// a < b
    Lesser,

    /// a <= b
    LesserEq,

    /// a > b
    Greater,

    /// a >= b
    GreaterEq,

    /// a && b
    And,

    /// a || b
    Or,

    /// a & b
    BitAnd,

    /// a | b
    BitOr,

    /// a ^ b
    BitXor,

    /// a + b
    Add,

    /// a +' b
    AddCarry,

    /// a - b
    Sub,

    /// a -' b
    SubCarry,

    /// a * b
    Mul,

    /// a / b
    Div,

    /// a % b
    Rem,

    /// a << b
    ShLeft,

    /// a >> b
    ShRightArith,

    /// a >>> b
    ShRightLogic,

    /// a <+ b
    RotLeft,

    /// a <+' b
    RotLeftCarry,

    /// a +> b
    RotRight,

    /// a +>' b
    RotRightCarry,
}
