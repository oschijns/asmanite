/// Span over source code
pub mod span;

/// Identify scope in block of code
pub mod scope;

/// Literals
pub mod literal;

/// Number type used to resolve expressions
pub mod number;

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

/// Define types for hashmap and hashset using small key types (max 64-bits)
pub mod small_hash;

/// Trivially box a type
pub trait ToBox: Sized {
    #[inline]
    fn to_box(self) -> Box<Self> {
        Box::new(self)
    }
}

// re-export crates
pub use num_traits;
