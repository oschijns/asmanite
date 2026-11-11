use crate::{lexer::atom::Primitive, utils::small_hash::SmallHashMap};
use core::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
};
use std::sync::OnceLock;

/// List of registers availabel for the given target with their corresponding primitive type
pub static REGISTERS: OnceLock<RegSet> = OnceLock::new();

/// Initialize the set of registers
#[inline]
pub fn registers_init(registers: &[(&str, RegDef)]) -> Result<(), RegInitError> {
    REGISTERS
        .set(RegSet::new(registers))
        .map_err(|_| RegInitError)
}

/// Is the given identifier a register ?
#[inline]
pub fn is_register(name: &str) -> Option<RegIdent> {
    REGISTERS.get().and_then(|set| set.get_register(name))
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
#[error("Failed to initialize register set")]
pub struct RegInitError;

/// Set of registers with their definition
pub struct RegSet(SmallHashMap<RegIdent, RegDef>);

impl RegSet {
    /// Given the set of registers and their definition
    /// Instantiate a global set
    pub fn new(registers: &[(&str, RegDef)]) -> Self {
        let mut map = SmallHashMap::with_capacity_and_hasher(registers.len(), Default::default());
        for (name, def) in registers.iter() {
            map.insert(RegIdent::new(name), *def);
        }
        Self(map)
    }

    /// Check if a given identifier is actually a register
    /// If so, return the corresponding register identifier
    pub fn get_register(&self, ident: &str) -> Option<RegIdent> {
        if ident.len() <= MAX_REG_NAME {
            let reg = RegIdent::new(ident);
            if self.0.contains_key(&reg) {
                return Some(reg);
            }
        }
        None
    }
}

/// Name of the register as a single CPU word
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegIdent([u8; MAX_REG_NAME]);

/// Maximum size of a register name
/// Note: This value may be increased if we want
/// to support a CPU with longer register names.
const MAX_REG_NAME: usize = 8;

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

/// Create a new register identifier from a raw string
impl From<&str> for RegIdent {
    #[inline]
    fn from(name: &str) -> Self {
        Self::new(name)
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

/// Definition of a given register
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegDef {
    /// Primitive type corresponding to the register
    pub primitive: Primitive,
}

impl RegDef {
    /// Definition of the register
    #[inline]
    pub fn new(primitive: Primitive) -> Self {
        Self { primitive }
    }
}

// MARK: TESTS

#[cfg(test)]
pub(crate) fn init_test_registers() {
    use crate::lexer::atom::primitive::U8;

    REGISTERS.get_or_init(|| {
        RegSet::new(&[
            ("a", RegDef::new(U8)),
            ("x", RegDef::new(U8)),
            ("y", RegDef::new(U8)),
        ])
    });
}
