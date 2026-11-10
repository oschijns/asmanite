/// Resolve symbols relative to modules
pub mod symbol_tree;

/// Keep track of the registers available on a given CPU target
pub mod register_dict;

/// Define types for hashmap and hashset using small key types (max 64-bits)
pub mod small_hash;
