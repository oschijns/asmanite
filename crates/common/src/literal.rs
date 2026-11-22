use core::fmt;
use num_traits::ToPrimitive;

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
}

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

/// Boolean literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoolLit(pub bool);
impl_display!(BoolLit);

/// Character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLit(pub char);
impl_display!(CharLit, "'{}'");

/// Integer literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntLit(pub u128);
impl_display!(IntLit);

/// Floating point literal
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatLit(pub f64);
impl_display!(FloatLit);

/// String literal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit(pub Box<str>);
impl_display!(StrLit, r#""{}""#);

impl ToPrimitive for Literal {
    /// Cast the literal into a signed 64-bits integer
    #[rustfmt::skip]
    fn to_i64(&self) -> Option<i64> {
        let int = match self {
            Self::Bool (lit) => lit.0 as i64,
            Self::Char (lit) => lit.0 as i64,
            Self::Int  (lit) => lit.0 as i64,
            Self::Float(lit) => lit.0 as i64,
            Self::Str  ( _ ) => { return None; }
        };
        Some(int)
    }

    /// Cast the literal into a unsigned 64-bits integer
    #[rustfmt::skip]
    fn to_u64(&self) -> Option<u64> {
        let int = match self {
            Self::Bool (lit) => lit.0 as u64,
            Self::Char (lit) => lit.0 as u64,
            Self::Int  (lit) => lit.0 as u64,
            Self::Float(lit) => lit.0 as u64,
            Self::Str  ( _ ) => { return None; }
        };
        Some(int)
    }
}
