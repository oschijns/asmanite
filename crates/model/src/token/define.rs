use std::fmt::{self, Write};

use crate::token::lexer::Token;

/// Implement Display trait
macro_rules! impl_display {
    ($type:ty) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
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
pub type Precedence = i8;

/// Define a primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Primitive {
    /// Type of the primitive uint, int or float
    pub prim_type: PrimType,

    /// Size of the primitive in bytes
    pub size: u8,
}

impl Primitive {
    /// Define a primitive
    #[inline]
    pub fn new(prim_type: PrimType, size: u8) -> Self {
        Self { prim_type, size }
    }

    /// Define a primitive with a size in bits
    #[inline]
    pub fn new_bits(prim_type: PrimType, bitsize: u8) -> Self {
        Self {
            prim_type,
            size: bitsize >> 8,
        }
    }
}

impl fmt::Display for Primitive {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.prim_type {
            PrimType::UInt  => 'u',
            PrimType::Int   => 'i',
            PrimType::Float => 'f',
        };
        write!(f, "{}{}", prefix, self.size * 8)
    }
}

/// Type of primitive values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum PrimType {
    /// Unsigned integer
    #[strum(serialize = "uint")]
    UInt,

    /// Signed integer
    #[strum(serialize = "int")]
    Int,

    /// Floating point number
    #[strum(serialize = "float")]
    Float,
}
impl_display!(PrimType, str);

/// Assignment instruction
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Assign {
    /// Optional operator while doing the assignment
    pub op: Option<Operator>,
}

impl Assign {
    /// Assignment
    #[inline]
    pub fn new() -> Self {
        Self { op: None }
    }

    /// New operator
    #[inline]
    pub fn op(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::new(op_type)),
        }
    }

    /// New operator with carry
    #[inline]
    pub fn op_carry(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::carry(op_type)),
        }
    }
}

impl fmt::Display for Assign {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(op) = self.op {
            op.fmt(f)?;
        }
        f.write_char('=')
    }
}

/// Operator with optional carry flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operator {
    /// The type of operator
    pub op_type: OpType,

    /// Should we use the carry flag or clear it ?
    pub carry: bool,
}

impl Operator {
    /// New operator
    #[inline]
    pub fn new(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: false,
        }
    }

    /// New operator with carry
    #[inline]
    pub fn carry(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: true,
        }
    }
}

impl fmt::Display for Operator {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.op_type.into())?;
        if self.carry {
            f.write_char('\'')?;
        }
        Ok(())
    }
}

/// Type of operator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum OpType {
    /// Equality '=='
    #[strum(serialize = "==")]
    Eq,

    /// Difference '!='
    #[strum(serialize = "!=")]
    Diff,

    /// Lesser than '<'
    #[strum(serialize = "<")]
    Lt,

    /// Greater than '>'
    #[strum(serialize = ">")]
    Gt,

    /// Lesser or equal than '<='
    #[strum(serialize = "<=")]
    LEq,

    /// Greater or equal than '>='
    #[strum(serialize = ">=")]
    GEq,

    /// Logical NOT '!'
    #[strum(serialize = "!")]
    Not,

    /// Logical AND '&&'
    #[strum(serialize = "&&")]
    And,

    /// Logical OR '||'
    #[strum(serialize = "||")]
    Or,

    /// Bitwise INV '~'
    #[strum(serialize = "~")]
    BitInv,

    /// Bitwise AND '&'
    #[strum(serialize = "&")]
    BitAnd,

    /// Bitwise OR '|'
    #[strum(serialize = "|")]
    BitOr,

    /// Bitwise XOR '^'
    #[strum(serialize = "^")]
    BitXor,

    /// Shift Left '<<'
    #[strum(serialize = "<<")]
    ShL,

    /// Shift Right (arithmetic) '>>'
    #[strum(serialize = ">>")]
    ShRA,

    /// Shift Right (logical) '>>>'
    #[strum(serialize = ">>>")]
    ShRL,

    /// Rotate bits Left '<+'
    #[strum(serialize = "<+")]
    RotL,

    /// Rotate bits Right '+>'
    #[strum(serialize = "+>")]
    RotR,

    /// Increment '++'
    #[strum(serialize = "++")]
    Inc,

    /// Decrement '--'
    #[strum(serialize = "--")]
    Dec,

    /// Addition '+'
    #[strum(serialize = "+")]
    Add,

    /// Subtraction '-'
    #[strum(serialize = "-")]
    Sub,

    /// Multiplication '*'
    #[strum(serialize = "*")]
    Mul,

    /// Division '/'
    #[strum(serialize = "/")]
    Div,

    /// Remainder '%'
    #[strum(serialize = "%")]
    Rem,
}
impl_display!(OpType, str);

impl OpType {
    /// Provide the precedence of the binary operators.
    #[rustfmt::skip]
    pub const fn precedence_binary(self) -> Precedence {
        match self {
            Self::Mul    |      // *
            Self::Div    |      // /
            Self::Rem    =>  9, // %
            Self::Add    |      // +
            Self::Sub    =>  8, // -
            Self::ShL    |      // <<
            Self::ShRA   |      // >>
            Self::ShRL   |      // >>>
            Self::RotL   |      // <+
            Self::RotR   =>  7, // +>
            Self::BitAnd =>  6, // &
            Self::BitXor =>  5, // ^
            Self::BitOr  =>  4, // |
            Self::Eq     |      // ==
            Self::Diff   |      // !=
            Self::Lt     |      // <
            Self::Gt     |      // >
            Self::LEq    |      // <=
            Self::GEq    =>  3, // >=
            Self::And    =>  2, // &&
            Self::Or     =>  1, // ||
            _ => Precedence::MIN,
        }
    }

    /// Provide the precedence of the unary operators.
    #[rustfmt::skip]
    pub const fn precedence_unary(self) -> Precedence {
        match self {
            Self::Not    |      // !
            Self::BitInv |      // ~
            Self::Inc    |      // ++
            Self::Dec    |      // --
            Self::Add    |      // +
            Self::Sub    |      // -
            Self::Mul    |      // *
            Self::BitAnd => 10, // &
            _ => Precedence::MIN,
        }
    }
}

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
    pub fn open(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: false,
        }
    }

    /// Create a new close delimiter
    #[inline]
    pub fn close(delim_type: DelimType) -> Self {
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

/// Range '..' or '..='
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    inclusive: bool,
}

impl Range {
    /// Exclusive range
    #[inline]
    pub fn excl() -> Self {
        Self { inclusive: false }
    }

    /// Inclusive range
    #[inline]
    pub fn incl() -> Self {
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
pub enum VarDecl {
    /// Variable
    #[strum(serialize = "var")]
    Var,

    /// Let
    #[strum(serialize = "let")]
    Let,

    /// Static variable
    #[strum(serialize = "static")]
    Static,

    /// Constant
    #[strum(serialize = "const")]
    Const,
}
impl_display!(VarDecl, str);

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
}
impl_display!(TypeDef, str);

/// Literal
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Boolean
    Bool(BoolLit),

    /// Character
    Char(CharLit),

    /// Integer
    Int(IntLit),

    /// Floating point number
    Float(FloatLit),

    /// String
    Str(StrLit),
}

impl Literal {
    /// Check if the token can be converted into a literal
    #[rustfmt::skip]
    pub fn try_from_token(token: &Token) -> Option<Self> {
        match token {
            Token::BoolLit  (lit) => Some(Self::Bool  (*lit)),
            Token::CharLit  (lit) => Some(Self::Char  (*lit)),
            Token::IntLit   (lit) => Some(Self::Int   (*lit)),
            Token::FloatLit (lit) => Some(Self::Float (*lit)),
            Token::StrLit   (lit) => Some(Self::Str   (lit.clone())),
            _ => None,
        }
    }
}

/// Boolean literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoolLit(pub bool);

/// Character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLit(pub char);

/// Integer literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntLit(pub u128);

/// Floating point literal
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatLit(pub f64);

/// String literal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit(pub Box<str>);

/// Identifier for register, variable, function, type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident(pub Box<str>);

/// Label identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub Box<str>);

impl_display!(BoolLit);
impl_display!(CharLit, "'{}'");
impl_display!(IntLit);
impl_display!(FloatLit);
impl_display!(StrLit, r#""{}""#);
impl_display!(Ident);
impl_display!(Label, "'{}");

/// Path to a variable, type or function
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path(pub Box<[Ident]>);

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            // A path cannot be empty
            Err(fmt::Error)
        } else {
            // Display all path elements with "::" in between
            write!(f, "{}", self.0[0])?;
            for elem in self.0[1..].iter() {
                write!(f, "::{}", elem)?;
            }
            Ok(())
        }
    }
}
