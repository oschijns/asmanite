/// Span over source code
pub mod span;

/// Literals
pub mod literal;

/// Status flags
pub mod status;

/// Registers
pub mod register;

/// Identifiers
pub mod ident;

/// Path to a given symbol
pub mod path;

/// Primitive types
pub mod primitive;

/// Operators
pub mod operator;

/// Resolve symbols relative to modules
pub mod symbol_tree;

/// Define types for hashmap and hashset using small key types (max 64-bits)
pub mod small_hash;

/// Trivially box a type
pub trait ToBox: Sized {
    #[inline]
    fn to_box(self) -> Box<Self> {
        Box::new(self)
    }
}
