use core::fmt;

/// Predfined values depicting each possible primitive type
#[rustfmt::skip]
pub mod def {
    use super::{Primitive, PrimType};

    pub const U8 : Primitive = Primitive::new_bits(PrimType::UInt ,  8);
    pub const U16: Primitive = Primitive::new_bits(PrimType::UInt , 16);
    pub const U24: Primitive = Primitive::new_bits(PrimType::UInt , 24);
    pub const U32: Primitive = Primitive::new_bits(PrimType::UInt , 32);
    pub const U64: Primitive = Primitive::new_bits(PrimType::UInt , 64);
    pub const I8 : Primitive = Primitive::new_bits(PrimType::Int  ,  8);
    pub const I16: Primitive = Primitive::new_bits(PrimType::Int  , 16);
    pub const I24: Primitive = Primitive::new_bits(PrimType::Int  , 24);
    pub const I32: Primitive = Primitive::new_bits(PrimType::Int  , 32);
    pub const I64: Primitive = Primitive::new_bits(PrimType::Int  , 64);
    pub const F16: Primitive = Primitive::new_bits(PrimType::Float, 16);
    pub const F32: Primitive = Primitive::new_bits(PrimType::Float, 32);
    pub const F64: Primitive = Primitive::new_bits(PrimType::Float, 64);
}

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
    pub const fn new(prim_type: PrimType, size: u8) -> Self {
        Self { prim_type, size }
    }

    /// Define a primitive with a size in bits
    #[inline]
    pub const fn new_bits(prim_type: PrimType, bitsize: u8) -> Self {
        Self {
            prim_type,
            size: bitsize >> 3,
        }
    }
}

/// Display the primitive type
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

/// Display the primitive kind
impl fmt::Display for PrimType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.into())
    }
}
