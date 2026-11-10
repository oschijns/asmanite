use crate::{
    ToBox,
    lexer::{atom::*, token::Token},
    parser::{
        expression::{Expr, parser_expr},
        path::{Path, parser_path},
    },
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, prelude::just,
    recursive::recursive, select, span::SimpleSpan,
};

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

    /// Function type
    Function(Box<FunctionType>),
}

/// Array type
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayType {
    /// Type of each cell
    pub cell_type: TypeUsed,

    /// Size of the array
    pub size: Expr,
}

impl ArrayType {
    /// Create a new array type
    #[inline]
    pub fn new(cell_type: TypeUsed, size: Expr) -> Self {
        Self { cell_type, size }
    }
}

impl ToBox for ArrayType {}

/// Tuple type
#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    /// Types in the tuple
    pub types: Box<[TypeUsed]>,
}

impl TupleType {
    /// Create a new tuple type
    #[inline]
    pub fn new(types: Vec<TypeUsed>) -> Self {
        Self {
            types: types.into_boxed_slice(),
        }
    }
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

impl ComplexType {
    /// Create a new complex type
    #[inline]
    pub fn new(ident: Path) -> Self {
        Self {
            ident,
            generics: None,
        }
    }

    /// Add generics to the type
    #[inline]
    pub fn with_generics(&mut self, generics: Vec<TypeGeneric>) -> &Self {
        self.generics = Some(generics.into_boxed_slice());
        self
    }
}

impl ToBox for ComplexType {}

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
    pub return_type: Option<TypeUsed>,
}

impl FunctionType {
    /// Create a function type
    #[inline]
    pub fn new(parameters: Vec<TypeUsed>) -> Self {
        Self {
            parameters: parameters.into_boxed_slice(),
            return_type: None,
        }
    }

    /// Add the return type
    #[inline]
    pub fn with_return(&mut self, return_type: TypeUsed) -> &Self {
        self.return_type = Some(return_type);
        self
    }
}

impl ToBox for FunctionType {}

// MARK: Parsers

/// Open '<' token
const GEN_OPEN: Token = Token::Operator(Operator::new(OpType::Lt));

/// Close '>' token
const GEN_CLOSE: Token = Token::Operator(Operator::new(OpType::Gt));

/// Open parenthesis '('
const PAREN_OPEN: Token = Token::Delimiter(delimiter::PAREN_OPEN);

/// Close parenthesis ')'
const PAREN_CLOSE: Token = Token::Delimiter(delimiter::PAREN_CLOSE);

/// Open bracket '['
const BRACKET_OPEN: Token = Token::Delimiter(delimiter::BRACKET_OPEN);

/// Close bracket ']'
const BRACKET_CLOSE: Token = Token::Delimiter(delimiter::BRACKET_CLOSE);

/// Parse a type usage.
pub fn parser_type_used<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|type_used| {
        let primitive = select! {
            Token::Primitive(primitive) => TypeUsed::Prim(primitive),
        };
        let array = parse_array_type(type_used.clone())
            .map(|array| TypeUsed::Array(array.to_box()))
            .boxed();
        let tuple = parse_tuple_type(type_used.clone())
            .map(TypeUsed::Tuple)
            .boxed();
        let complex = parse_complex_type(type_used.clone())
            .map(|complex| TypeUsed::Complex(complex.to_box()))
            .boxed();
        let function = parse_function_type(type_used.clone())
            .map(|function| TypeUsed::Function(function.to_box()))
            .boxed();

        primitive
            .boxed()
            .or(array)
            .or(tuple)
            .or(function)
            .or(complex)
    })
}

/// Parse an array type such as `[u8; 3]`.
pub fn parser_array_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, ArrayType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_array_type(parser_type_used().boxed())
}

fn parse_array_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, ArrayType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    type_used
        .then_ignore(just(Token::Semicolon))
        .then(parser_expr(false))
        .map(|(cell_type, size)| ArrayType::new(cell_type, size))
        .delimited_by(just(BRACKET_OPEN), just(BRACKET_CLOSE))
}

/// Parse a tuple type such as `(u8, u16, i32)`.
pub fn parser_tuple_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, TupleType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_tuple_type(parser_type_used().boxed())
}

fn parse_tuple_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, TupleType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    type_used
        .separated_by(just(Token::Comma))
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .map(TupleType::new)
        .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE))
}

/// Parse a complex type such as `path::to::MyType<i16, 3>`.
pub fn parser_complex_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, ComplexType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_complex_type(parser_type_used().boxed())
}

fn parse_complex_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, ComplexType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let generic = type_used
        .map(TypeGeneric::Type)
        .or(parser_expr(false).map(TypeGeneric::Const));
    let generics = generic
        .separated_by(just(Token::Comma))
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(GEN_OPEN), just(GEN_CLOSE));

    parser_path()
        .then(generics.or_not())
        .map(|(ident, generics)| {
            let mut complex = ComplexType::new(ident);
            if let Some(generics) = generics {
                complex.with_generics(generics);
            }
            complex
        })
}

/// Parse a function type such as `fn(u8, i16) -> i8`.
pub fn parser_function_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, FunctionType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_function_type(parser_type_used().boxed())
}

fn parse_function_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, FunctionType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let parameters = type_used
        .clone()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE));
    let return_type = just(Token::Arrow).ignore_then(type_used).or_not();

    just(Token::Func)
        .ignore_then(parameters)
        .then(return_type)
        .map(|(parameters, return_type)| {
            let mut function = FunctionType::new(parameters);
            if let Some(return_type) = return_type {
                function.with_return(return_type);
            }
            function
        })
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_type_used(source: &str) -> TypeUsed {
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_type_used()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    #[test]
    fn parses_array_type() {
        assert_eq!(
            parse_type_used("[u8; 3]"),
            TypeUsed::Array(
                ArrayType::new(
                    TypeUsed::Prim(primitive::U8),
                    Expr::Lit(Literal::Int(IntLit(3))),
                )
                .to_box()
            )
        );
    }

    #[test]
    fn parses_tuple_type() {
        assert_eq!(
            parse_type_used("(u8, u16, i32)"),
            TypeUsed::Tuple(TupleType::new(vec![
                TypeUsed::Prim(primitive::U8),
                TypeUsed::Prim(primitive::U16),
                TypeUsed::Prim(primitive::I32),
            ]))
        );
    }

    #[test]
    fn parses_complex_type_with_type_and_constant_generics() {
        assert_eq!(
            parse_type_used("path::to::MyType<i16, 3>"),
            TypeUsed::Complex(
                ComplexType {
                    ident: Path::new(&["path", "to", "MyType"]),
                    generics: Some(
                        vec![
                            TypeGeneric::Type(TypeUsed::Prim(primitive::I16)),
                            TypeGeneric::Const(Expr::Lit(Literal::Int(IntLit(3)))),
                        ]
                        .into_boxed_slice(),
                    ),
                }
                .to_box()
            )
        );
    }

    #[test]
    fn parses_function_type() {
        assert_eq!(
            parse_type_used("fn(u8, i16) -> i8"),
            TypeUsed::Function(
                FunctionType {
                    parameters: vec![
                        TypeUsed::Prim(primitive::U8),
                        TypeUsed::Prim(primitive::I16),
                    ]
                    .into_boxed_slice(),
                    return_type: Some(TypeUsed::Prim(primitive::I8)),
                }
                .to_box()
            )
        );
    }
}
