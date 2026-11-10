/// Lexer and tokens
pub mod lexer;

/// Parser and AST structure
pub mod parser;

/// Other utilities
pub mod utils;

/// Trivially box a type
trait ToBox: Sized {
    #[inline]
    fn to_box(self) -> Box<Self> {
        Box::new(self)
    }
}
