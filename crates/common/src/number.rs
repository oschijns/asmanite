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
