use core::fmt;

/// Type of status flags that can be set by an operation
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
pub enum StatusFlag {
    /// Zero flag is set
    #[strum(serialize = "zero")]
    Zero,

    /// Negative flag is set
    #[strum(serialize = "negative")]
    Negative,

    /// Carry flag is set
    #[strum(serialize = "carry")]
    Carry,

    /// Overflow flag is set
    #[strum(serialize = "overflow")]
    Overflow,
}

impl fmt::Display for StatusFlag {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.into())
    }
}
