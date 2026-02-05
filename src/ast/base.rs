//! Basic elements, neither statements nor expressions.

use std::fmt;

use crate::lexer::Token;

/// Error encountered when converting tokens into a given element
#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum TokenError {
    #[error("Expected a primitive literal")]
    PrimLit,

    #[error("Expected a primitive type")]
    PrimType,

    #[error("Expected an unary operator")]
    Unary,

    #[error("Expected a binary operator")]
    Binary,

    #[error("Expected an assignment operator")]
    Assign,
}

/// Identifier
#[derive(Debug, Clone, PartialEq)]
pub struct Ident<'src>(pub(crate) &'src str);

/// Path to a symbol, sequence of identifiers separated by `::`
#[derive(Debug, Clone, PartialEq)]
pub struct IdentPath<'src> {
    /// Path to the symbol
    pub(crate) path: Vec<Ident<'src>>,

    /// The symbol
    pub(crate) ident: Ident<'src>,
}

/// Basic literals
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum PrimLit {
    /// Boolean literal
    Bool(bool),

    /// Character literal
    Char(char),

    /// Integer literal
    Int(i64),

    /// Float literal
    Float(f64),
}

/// Primitive types available for operations
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimType {
    /// Unsigned 8-bits integer type
    U8,

    /// Unsigned 16-bits integer type
    U16,

    /// Unsigned 24-bits integer type
    U24,

    /// Unsigned 32-bits integer type
    U32,

    /// Unsigned 64-bits integer type
    U64,

    /// Signed 8-bits integer type
    I8,

    /// Signed 16-bits integer type
    I16,

    /// Signed 24-bits integer type
    I24,

    /// Signed 32-bits integer type
    I32,

    /// Signed 64-bits integer type
    I64,

    /// Floating point 16-bits number type
    F16,

    /// Floating point 32-bits number type
    F32,

    /// Floating point 64-bits number type
    F64,
}

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

/// Unary operator over a value
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Binary operator over two values
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Assignement wrapper for binary operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssignOp(pub BinaryOp);

/// Print the identifier
impl fmt::Display for Ident<'_> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Print the identifier path
impl fmt::Display for IdentPath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for i in self.path.iter() {
            let _ = write!(f, "{i}::")?;
        }
        write!(f, "{}", self.ident)
    }
}

/// Try to get a primitive literal from a token
impl TryFrom<Token> for PrimLit {
    type Error = TokenError;

    #[rustfmt::skip]
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::False           => Ok(Self::Bool(false)),
            Token::True            => Ok(Self::Bool(true)),
            Token::CharLit (chr)   => Ok(Self::Char(chr)),
            Token::IntLit  (int)   => Ok(Self::Int(int)),
            Token::FloatLit(float) => Ok(Self::Float(float)),
            _ => Err(TokenError::PrimLit),
        }
    }
}

/// Print the primitive type
impl fmt::Display for PrimLit {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool (b)     => write!(f, "{b}"    ),
            Self::Char (chr)   => write!(f, "'{chr}'"),
            Self::Int  (int)   => write!(f, "{int}"  ),
            Self::Float(float) => write!(f, "{float}"),
        }
    }
}

/// Try to get a primitive type from a token
impl TryFrom<Token> for PrimType {
    type Error = TokenError;

    #[rustfmt::skip]
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::U8  => Ok(Self::U8 ),
            Token::U16 => Ok(Self::U16),
            Token::U24 => Ok(Self::U24),
            Token::U32 => Ok(Self::U32),
            Token::U64 => Ok(Self::U64),
            Token::I8  => Ok(Self::I8 ),
            Token::I16 => Ok(Self::I16),
            Token::I24 => Ok(Self::I24),
            Token::I32 => Ok(Self::I32),
            Token::I64 => Ok(Self::I64),
            Token::F16 => Ok(Self::F16),
            Token::F32 => Ok(Self::F32),
            Token::F64 => Ok(Self::F64),
            _ => Err(TokenError::PrimType),
        }
    }
}

/// Print the primitive type
impl fmt::Display for PrimType {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::U8  => write!(f, "u8" ),
            Self::U16 => write!(f, "u16"),
            Self::U24 => write!(f, "u24"),
            Self::U32 => write!(f, "u32"),
            Self::U64 => write!(f, "u64"),
            Self::I8  => write!(f, "i8" ),
            Self::I16 => write!(f, "i16"),
            Self::I24 => write!(f, "i24"),
            Self::I32 => write!(f, "i32"),
            Self::I64 => write!(f, "i64"),
            Self::F16 => write!(f, "f16"),
            Self::F32 => write!(f, "f32"),
            Self::F64 => write!(f, "f64"),
        }
    }
}

/// Try to get an unary operation from a token
/// note: post increment and post decrement cannot be obtained here
impl TryFrom<Token> for UnaryOp {
    type Error = TokenError;

    #[rustfmt::skip]
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::BoolNot => Ok(Self::Not),
            Token::BitInv  => Ok(Self::Inv),
            Token::Minus   => Ok(Self::Neg),
            Token::Incr    => Ok(Self::PreIncr),
            Token::Decr    => Ok(Self::PreDecr),
            _ => Err(TokenError::Unary),
        }
    }
}

/// Print the unary operation
impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Not => write!(f, "!"),
            Self::Inv => write!(f, "~"),
            Self::Neg => write!(f, "-"),
            Self::PreIncr | Self::PostIncr => write!(f, "++"),
            Self::PreDecr | Self::PostDecr => write!(f, "--"),
        }
    }
}

/// Try to get a binary operation from a token
impl TryFrom<Token> for BinaryOp {
    type Error = TokenError;

    #[rustfmt::skip]
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::Equal            => Ok(Self::Equal),
            Token::Diff             => Ok(Self::Diff),
            Token::Lesser           => Ok(Self::Lesser),
            Token::LesserEq         => Ok(Self::LesserEq),
            Token::Greater          => Ok(Self::Greater),
            Token::GreaterEq        => Ok(Self::GreaterEq),
            Token::BoolAnd          => Ok(Self::And),
            Token::BoolOr           => Ok(Self::Or),
            Token::BitAnd           => Ok(Self::BitAnd),
            Token::BitOr            => Ok(Self::BitOr),
            Token::BitXor           => Ok(Self::BitXor),
            Token::Plus             => Ok(Self::Add),
            Token::PlusCarry        => Ok(Self::AddCarry),
            Token::Minus            => Ok(Self::Sub),
            Token::MinusCarry       => Ok(Self::SubCarry),
            Token::Mul              => Ok(Self::Mul),
            Token::Div              => Ok(Self::Div),
            Token::Rem              => Ok(Self::Rem),
            Token::ShiftLeft        => Ok(Self::ShLeft),
            Token::ShiftRightArith  => Ok(Self::ShRightArith),
            Token::ShiftRightLogic  => Ok(Self::ShRightLogic),
            Token::RotateLeft       => Ok(Self::RotLeft),
            Token::RotateLeftCarry  => Ok(Self::RotLeftCarry),
            Token::RotateRight      => Ok(Self::RotRight),
            Token::RotateRightCarry => Ok(Self::RotRightCarry),
            _ => Err(TokenError::Binary),
        }
    }
}

/// Print the binary operation
impl fmt::Display for BinaryOp {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Equal         => write!(f, "==" ),
            Self::Diff          => write!(f, "!=" ),
            Self::Lesser        => write!(f, "<"  ),
            Self::LesserEq      => write!(f, "<=" ),
            Self::Greater       => write!(f, ">"  ),
            Self::GreaterEq     => write!(f, ">=" ),
            Self::And           => write!(f, "&&" ),
            Self::Or            => write!(f, "||" ),
            Self::BitAnd        => write!(f, "&"  ),
            Self::BitOr         => write!(f, "|"  ),
            Self::BitXor        => write!(f, "^"  ),
            Self::Add           => write!(f, "+"  ),
            Self::AddCarry      => write!(f, "+'" ),
            Self::Sub           => write!(f, "-"  ),
            Self::SubCarry      => write!(f, "-'" ),
            Self::Mul           => write!(f, "*"  ),
            Self::Div           => write!(f, "/"  ),
            Self::Rem           => write!(f, "%"  ),
            Self::ShLeft        => write!(f, "<<" ),
            Self::ShRightArith  => write!(f, ">>" ),
            Self::ShRightLogic  => write!(f, ">>>"),
            Self::RotLeft       => write!(f, "<+" ),
            Self::RotLeftCarry  => write!(f, "<+'"),
            Self::RotRight      => write!(f, "+>" ),
            Self::RotRightCarry => write!(f, "+>'"),
        }
    }
}

/// Try to get a assignement operation from a token
impl TryFrom<Token> for AssignOp {
    type Error = TokenError;

    #[rustfmt::skip]
    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::BitAndAssign           => Ok(Self(BinaryOp::BitAnd)),
            Token::BitOrAssign            => Ok(Self(BinaryOp::BitOr)),
            Token::BitXorAssign           => Ok(Self(BinaryOp::BitXor)),
            Token::PlusAssign             => Ok(Self(BinaryOp::Add)),
            Token::PlusCarryAssign        => Ok(Self(BinaryOp::AddCarry)),
            Token::MinusAssign            => Ok(Self(BinaryOp::Sub)),
            Token::MinusCarryAssign       => Ok(Self(BinaryOp::SubCarry)),
            Token::MulAssign              => Ok(Self(BinaryOp::Mul)),
            Token::DivAssign              => Ok(Self(BinaryOp::Div)),
            Token::RemAssign              => Ok(Self(BinaryOp::Rem)),
            Token::ShiftLeftAssign        => Ok(Self(BinaryOp::ShLeft)),
            Token::ShiftRightArithAssign  => Ok(Self(BinaryOp::ShRightArith)),
            Token::ShiftRightLogicAssign  => Ok(Self(BinaryOp::ShRightLogic)),
            Token::RotateLeftAssign       => Ok(Self(BinaryOp::RotLeft)),
            Token::RotateLeftCarryAssign  => Ok(Self(BinaryOp::RotLeftCarry)),
            Token::RotateRightAssign      => Ok(Self(BinaryOp::RotRight)),
            Token::RotateRightCarryAssign => Ok(Self(BinaryOp::RotRightCarry)),
            _ => Err(TokenError::Assign),
        }
    }
}

/// Print the assignement operation
impl fmt::Display for AssignOp {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}=", self.0)
    }
}
