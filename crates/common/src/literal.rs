use core::fmt;

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
