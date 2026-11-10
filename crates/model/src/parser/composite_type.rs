use crate::{
    lexer::{atom::*, token::Token},
    parser::{expression::Expr, path::Path},
};
use chumsky::{Parser, error::Rich, extra, input::ValueInput, select, span::SimpleSpan};

// MARK: AST

/// Type used by a variable
#[derive(Debug, Clone, PartialEq)]
pub enum TypeUsed {
    /// Primitive type
    Prim(Primitive),

    /// Pointer to a type
    Pointer(Box<Self>),

    /// Array type
    Array(Box<ArrayType>),

    // Tuple type
    Tuple(TupleType),

    /// Complex type
    Complex(Box<ComplexType>),
}

/// Array type
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    /// Type of each cell
    pub cell_type: TypeUsed,

    /// Size of the array
    pub size: Expr,
}

/// Tuple type
#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    /// Types in the tuple
    pub types: Box<[TypeUsed]>,
}

/// Complex type usage
#[derive(Debug, Clone, PartialEq)]
pub struct ComplexType {
    /// Path to the identifier of the type
    pub ident: Path,

    /// Generics of the type
    /// Note: Most types don't have generics
    pub generics: Option<Box<[TypeGeneric]>>,
}

/// Generic used in a complex type
#[derive(Debug, Clone, PartialEq)]
pub enum TypeGeneric {
    /// Generic is a subtype
    Type(TypeUsed),

    /// Generic is a constant value
    Const(Expr),
}

/// Function type
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    /// Parameters of the function
    pub parameters: Box<[TypeUsed]>,

    /// Return type
    pub return_type: TypeUsed,
}

// MARK: Parsers

/// Open '<' token
const GEN_OPEN: Token = Token::Operator(Operator::new(OpType::Lt));

/// Close '>' token
const GEN_CLOSE: Token = Token::Operator(Operator::new(OpType::Gt));

/// Parse a type usage.
pub fn parser_type_used<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    select! { Token::Primitive(primitive) => TypeUsed::Prim( primitive ) }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
}
