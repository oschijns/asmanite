use asmanite_common::operator::OpType;
use chumsky::pratt;
use core::fmt::{self, Write};

/// Implement Display trait
macro_rules! impl_display {
    ($type:ty, $fmt:literal) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $fmt, self.0)
            }
        }
    };
    ($type:ty, str) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.into())
            }
        }
    };
}

/// Define the precedence of an operator.
/// Higher values mean the operator have a higher priority than others.
pub type Precedence = u16;

/// Define the associativity of the operation
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    /// No associativity (parenthesis are required)
    #[default]
    None,

    /// Left to right associativity
    Left,

    /// Right to left associativity
    Right,
}

impl Associativity {
    /// Convert generic associativity enum into chumsky compatible one
    #[rustfmt::skip]
    pub fn to_chumsky(self, precedence: Precedence) -> pratt::Associativity {
        match self {
            Self::None  => pratt::none (precedence),
            Self::Left  => pratt::left (precedence),
            Self::Right => pratt::right(precedence),
        }
    }
}

/// Associativity of type cast operation
pub const ASSOC_TYPE_CAST: Associativity = Associativity::Left;

/// Precendence of type cast operation
pub const PREC_TYPE_CAST: Precedence = 39;

/// Associativity of the assignement expression is always right to left
pub const ASSOC_ASSIGN: Associativity = Associativity::Right;

/// Precendence of assignement expression is always the same
pub const PREC_ASSIGN: Precedence = 10;

/// Associativity of indexing operation
pub const ASSOC_INDEX: Associativity = Associativity::Left;

/// Precendence of indexing operation
pub const PREC_INDEX: Precedence = 41;

/// Precendence of unary operators
pub const PREC_UNARY: Precedence = 40;

/// Provide the precedence of the binary operators.
#[rustfmt::skip]
pub const fn precedence_binary(op: OpType) -> Precedence {
    match op {
        OpType::Mul    |      // *
        OpType::Div    |      // /
        OpType::Rem    => 29, // %
        OpType::Add    |      // +
        OpType::Sub    => 28, // -
        OpType::ShL    |      // <<
        OpType::ShRA   |      // >>
        OpType::ShRL   |      // >>>
        OpType::RotL   |      // <+
        OpType::RotR   => 27, // +>
        OpType::BitAnd => 26, // &
        OpType::BitXor => 25, // ^
        OpType::BitOr  => 24, // |
        OpType::Eq     |      // ==
        OpType::Diff   |      // !=
        OpType::Lt     |      // <
        OpType::Gt     |      // >
        OpType::LEq    |      // <=
        OpType::GEq    => 23, // >=
        OpType::And    => 22, // &&
        OpType::Or     => 21, // ||
        _ => Precedence::MIN,
    }
}

/// Provide the precedence of the unary operators.
#[rustfmt::skip]
pub const fn precedence_unary(op: OpType) -> Precedence {
    match op {
        OpType::Not    | // !
        OpType::BitInv | // ~
        OpType::Inc    | // ++
        OpType::Dec    | // --
        OpType::Add    | // +
        OpType::Sub    | // -
        OpType::Mul    | // *
        OpType::BitAnd   // &
            => PREC_UNARY,
        _ => Precedence::MIN,
    }
}

/// Provide the associativity of the operators.
#[rustfmt::skip]
pub const fn associativity_binary(op: OpType) -> Associativity {
    if matches!(op,
        OpType::Eq   | // ==
        OpType::Diff | // !=
        OpType::Lt   | // <
        OpType::Gt   | // >
        OpType::LEq  | // <=
        OpType::GEq    // >=
    ) {
        Associativity::None
    } else {
        Associativity::Left
    }
}

/// Get a bitmask for the given operator, this allows for grouping
/// operators with similar properties (associativity and precedence).
#[inline]
pub const fn op_bitmask(op: OpType) -> u64 {
    1u64 << (op as u64)
}

// MARK: DELIMITER

#[rustfmt::skip]
pub mod delimiter {
    use super::{Delimiter, DelimType};

    pub const PAREN_OPEN   : Delimiter = Delimiter::open (DelimType::Paren  );
    pub const PAREN_CLOSE  : Delimiter = Delimiter::close(DelimType::Paren  );
    pub const BRACKET_OPEN : Delimiter = Delimiter::open (DelimType::Bracket);
    pub const BRACKET_CLOSE: Delimiter = Delimiter::close(DelimType::Bracket);
    pub const BRACE_OPEN   : Delimiter = Delimiter::open (DelimType::Brace  );
    pub const BRACE_CLOSE  : Delimiter = Delimiter::close(DelimType::Brace  );
}

/// Precendence of expression enclosed in parenthesis
pub const PREC_PAREN: Precedence = 100;

/// Define a delimiter token '(', ')', '[', ']', '{', '}'
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delimiter {
    /// Type of delimiter
    pub delim_type: DelimType,

    /// Is it an open or close delimiter
    pub is_close: bool,
}

impl Delimiter {
    /// Create a new open delimiter
    #[inline]
    pub const fn open(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: false,
        }
    }

    /// Create a new close delimiter
    #[inline]
    pub const fn close(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: true,
        }
    }
}

impl fmt::Display for Delimiter {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = if self.is_close {
            match self.delim_type {
                DelimType::Paren   => ')',
                DelimType::Brace   => '}',
                DelimType::Bracket => ']',
            }
        } else {
            match self.delim_type {
                DelimType::Paren   => '(',
                DelimType::Brace   => '{',
                DelimType::Bracket => '[',
            }
        };
        f.write_char(d)
    }
}

/// Type of delimiter
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum DelimType {
    /// Parenthesis '(', ')'
    #[strum(serialize = "()")]
    Paren,

    /// Brace '{', '}'
    #[strum(serialize = "{}")]
    Brace,

    /// Bracket '[', ']'
    #[strum(serialize = "[]")]
    Bracket,
}
impl_display!(DelimType, str);

// MARK: RANGE

/// Range '..' or '..='
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    inclusive: bool,
}

impl Range {
    /// Exclusive range
    #[inline]
    pub const fn excl() -> Self {
        Self { inclusive: false }
    }

    /// Inclusive range
    #[inline]
    pub const fn incl() -> Self {
        Self { inclusive: true }
    }
}

impl fmt::Display for Range {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = if self.inclusive { "..=" } else { ".." };
        f.write_str(s)
    }
}

/// Declare a variable/constant
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum VarType {
    /// Variable
    #[strum(serialize = "var")]
    Var,

    /// Let
    #[strum(serialize = "let")]
    Let,

    /// Stack based variable
    #[strum(serialize = "stack")]
    Stack,

    /// Static variable
    #[strum(serialize = "static")]
    Static,

    /// Constant
    #[strum(serialize = "const")]
    Const,
}
impl_display!(VarType, str);

/// Type definition keyword 'struct', 'enum', 'union'
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum TypeDef {
    /// Structure
    #[strum(serialize = "struct")]
    Struct,

    /// Enumeration
    #[strum(serialize = "enum")]
    Enum,

    /// Memory Union
    #[strum(serialize = "union")]
    Union,

    /// Type alias
    #[strum(serialize = "type")]
    Alias,
}
impl_display!(TypeDef, str);
