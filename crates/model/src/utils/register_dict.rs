use core::fmt;
use std::{
    fmt::Write,
    hash::{Hash, Hasher},
};

/// Maximum size of a register name
/// Note: This value may be increased if we want
/// to support a CPU with longer register names.
const MAX_REG_NAME: usize = 8;

/// Name of the register as a single CPU word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegIdent([u8; MAX_REG_NAME]);

impl RegIdent {
    /// Create a new register identifier
    /// If the name is too long, it will be truncated
    #[inline]
    pub fn new(name: &str) -> Self {
        let mut chars = [0; MAX_REG_NAME];
        for (c, t) in chars.iter_mut().zip(name.chars()) {
            *c = t as u8;
        }
        Self(chars)
    }
}

impl Hash for RegIdent {
    /// Write the identifier value as is
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(u64::from_ne_bytes(self.0));
    }
}

impl fmt::Display for RegIdent {
    // Print the register name
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for &c in self.0.iter() {
            if c == 0 {
                break;
            }
            f.write_char(c as char)?;
        }
        Ok(())
    }
}
