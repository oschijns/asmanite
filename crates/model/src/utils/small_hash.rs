use std::hash::Hasher;

/// Integer Hasher for faster look up on small types
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct SmallHasher(u64);

impl Hasher for SmallHasher {
    /// Output the value as is
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    /// Should not be used
    fn write(&mut self, bytes: &[u8]) {
        panic!("Should not be used")
    }

    /// Set the value as is
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0 = i;
    }
}

/// Generate hasher for small type values
pub(crate) type BuildSmallHasher = std::hash::BuildHasherDefault<SmallHasher>;

/// Small key type hash-set
pub(crate) type SmallHashSet<T> = std::collections::HashSet<T, BuildSmallHasher>;

/// Small key type hash-map
pub(crate) type SmallHashMap<K, V> = std::collections::HashMap<K, V, BuildSmallHasher>;
