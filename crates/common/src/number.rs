use crate::literal::Literal;
use core::fmt;

/// Type used for immediate values
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum Number {
    /// Unsigned integer
    UInt(u64),

    /// Signed integer
    Int(i64),

    /// Floating point number
    Float(f64),
}

impl fmt::Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UInt(uint) => write!(f, "{}", uint),
            Self::Int(int) => write!(f, "{}", int),
            Self::Float(float) => write!(f, "{}", float),
        }
    }
}

impl Number {
    /// Value of a literal (strings are not numbers)
    #[rustfmt::skip]
    pub fn from_literal(lit: &Literal) -> Option<Self> {
        let value = match lit {
            Literal::Bool (lit) => Self::from(lit.0),
            Literal::Char (lit) => Self::from(u32::from(lit.0)),
            Literal::Int  (lit) => Self::from(u64::try_from(lit.0).ok()?),
            Literal::Float(lit) => Self::from(lit.0),
            Literal::Str  ( _ ) => { return None; }
        };
        Some(value)
    }

    /// Truth value of the number (non-zero)
    pub fn is_true(self) -> bool {
        match self {
            Self::UInt(uint) => uint != 0,
            Self::Int(int) => int != 0,
            Self::Float(float) => float != 0.0,
        }
    }

    /// Convert the number into a signed integer (wrapping, floats are truncated)
    pub fn wrapping_to_i64(self) -> i64 {
        match self {
            Self::UInt(uint) => uint as i64,
            Self::Int(int) => int,
            Self::Float(float) => float as i64,
        }
    }

    /// Convert the number into a floating point number
    pub fn to_f64(self) -> f64 {
        match self {
            Self::UInt(uint) => uint as f64,
            Self::Int(int) => int as f64,
            Self::Float(float) => float,
        }
    }

    /// Promote two numbers to a common representation (see [`Promoted`])
    pub fn promote(self, rhs: Self) -> Promoted {
        match (self, rhs) {
            (Self::UInt(lhs), Self::UInt(rhs)) => Promoted::UInt(lhs, rhs),
            (Self::Float(_), _) | (_, Self::Float(_)) => {
                Promoted::Float(self.to_f64(), rhs.to_f64())
            }
            _ => Promoted::Int(self.wrapping_to_i64(), rhs.wrapping_to_i64()),
        }
    }
}

/// Operands of a binary operation, promoted to a common representation:
/// floats win over signed integers, which win over unsigned integers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Promoted {
    UInt(u64, u64),
    Int(i64, i64),
    Float(f64, f64),
}

/// Implement conversion from primitive into Number
macro_rules! impl_num_from {
    ($variant:ident ; $type:ty) => {
        impl From<$type> for Number {
            #[inline]
            fn from(value: $type) -> Self {
                Self::$variant(value)
            }
        }
    };
    ($variant:ident ; $type:ty as $cast:ty) => {
        impl From<$type> for Number {
            #[inline]
            fn from(value: $type) -> Self {
                Self::$variant(value as $cast)
            }
        }
    };
}
impl_num_from!(UInt  ; bool  as u64);
impl_num_from!(UInt  ; u8    as u64);
impl_num_from!(UInt  ; u16   as u64);
impl_num_from!(UInt  ; u32   as u64);
impl_num_from!(UInt  ; usize as u64);
impl_num_from!(UInt  ; u64);
impl_num_from!(Int   ; i8    as i64);
impl_num_from!(Int   ; i16   as i64);
impl_num_from!(Int   ; i32   as i64);
impl_num_from!(Int   ; isize as i64);
impl_num_from!(Int   ; i64);
impl_num_from!(Float ; f32 as f64);
impl_num_from!(Float ; f64);

/// Error encountered when trying to convert a number into the target primitive
#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Failed to cast {value} into {target} type.")]
pub struct NumCastError {
    /// Target type
    target: &'static str,

    /// Value which caused the error
    value: Number,
}
impl NumCastError {
    #[inline]
    fn new(target: &'static str, value: Number) -> Self {
        Self { target, value }
    }
}

/// Implement conversion from the number type into the target primitive type
macro_rules! impl_num_try_from {
    ( $type:ty ; "float" ) => {
        impl TryFrom<Number> for $type {
            type Error = NumCastError;

            fn try_from(value: Number) -> Result<Self, Self::Error> {
                let value = match value {
                    Number::UInt(uint) => uint as $type,
                    Number::Int(int) => int as $type,
                    Number::Float(float) => float as $type,
                };
                Ok(value)
            }
        }
    };
    ( $type:ty ; $target:literal ) => {
        impl TryFrom<Number> for $type {
            type Error = NumCastError;

            fn try_from(value: Number) -> Result<Self, Self::Error> {
                const TARGET: &'static str = $target;
                match value {
                    Number::UInt(uint) => {
                        Self::try_from(uint).map_err(|_| NumCastError::new(TARGET, value))
                    }
                    Number::Int(int) => {
                        Self::try_from(int).map_err(|_| NumCastError::new(TARGET, value))
                    }
                    Number::Float(float) => {
                        let int = float.round() as i64;
                        Self::try_from(int).map_err(|_| NumCastError::new(TARGET, value))
                    }
                }
            }
        }
    };
}
impl_num_try_from!( u8    ; "u8"    );
impl_num_try_from!( u16   ; "u16"   );
impl_num_try_from!( u32   ; "u32"   );
impl_num_try_from!( u64   ; "u64"   );
impl_num_try_from!( usize ; "usize" );
impl_num_try_from!( i8    ; "i8"    );
impl_num_try_from!( i16   ; "i16"   );
impl_num_try_from!( i32   ; "i32"   );
impl_num_try_from!( i64   ; "i64"   );
impl_num_try_from!( isize ; "isize" );
impl_num_try_from!( f32   ; "float" );
impl_num_try_from!( f64   ; "float" );

#[cfg(test)]
mod tests {
    use super::{Number, Promoted};
    use crate::literal::{BoolLit, CharLit, FloatLit, IntLit, Literal, StrLit};

    #[test]
    fn converts_literals() {
        let lit = |lit| Number::from_literal(&lit);
        assert_eq!(lit(Literal::Bool(BoolLit(true))), Some(Number::UInt(1)));
        assert_eq!(lit(Literal::Char(CharLit('A'))), Some(Number::UInt(65)));
        assert_eq!(lit(Literal::Int(IntLit(42))), Some(Number::UInt(42)));
        assert_eq!(lit(Literal::Int(IntLit(u128::MAX))), None);
        assert_eq!(lit(Literal::Float(FloatLit(1.5))), Some(Number::Float(1.5)));
        assert_eq!(lit(Literal::Str(StrLit("a".into()))), None);
    }

    #[test]
    fn promotes_to_the_widest_representation() {
        let (u, i, f) = (Number::UInt(2), Number::Int(-1), Number::Float(0.5));
        assert_eq!(u.promote(u), Promoted::UInt(2, 2));
        assert_eq!(u.promote(i), Promoted::Int(2, -1));
        assert_eq!(i.promote(f), Promoted::Float(-1.0, 0.5));
        assert_eq!(f.promote(u), Promoted::Float(0.5, 2.0));
    }

    #[test]
    fn truth_value_is_non_zero() {
        assert!(Number::UInt(3).is_true());
        assert!(Number::Int(-1).is_true());
        assert!(!Number::Float(0.0).is_true());
    }
}
