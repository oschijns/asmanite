use crate::{
    ToBox,
    lexer::{
        atom::*,
        token::{IdentOrReg, Token},
    },
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

    /// Function type
    Function(Box<FunctionType>),

    /// Named type
    Named(Box<NamedType>),

    /// Composite type
    Composite(Box<CompositeType>),

    /// Enum type
    Enum(Box<EnumType>),
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

/// Complex named type defined in another location
#[derive(Debug, Clone, PartialEq)]
pub struct NamedType {
    /// Path to the identifier of the type
    pub ident: Path,

    /// Generics of the type
    /// Note: Most types don't have generics
    pub generics: Option<Box<[GenericUsed]>>,
}

impl NamedType {
    /// Create a new complex named type
    #[inline]
    pub fn new(ident: Path) -> Self {
        Self {
            ident,
            generics: None,
        }
    }

    /// Add generics to the type
    #[inline]
    pub fn with_generics(&mut self, generics: Vec<GenericUsed>) -> &Self {
        self.generics = Some(generics.into_boxed_slice());
        self
    }
}

impl ToBox for NamedType {}

/// Generic used in a complex type
#[derive(Debug, Clone, PartialEq)]
pub enum GenericUsed {
    /// Generic is a subtype
    Type(TypeUsed),

    /// Generic is a constant value
    Const(Expr),
}

/// Struct / Union type
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeType {
    /// Name of the composite type
    pub name: Option<Ident>,

    /// Generic parameters declared by this type.
    pub generics: Option<Generics>,

    /// Fields in the struct
    pub fields: Box<[IdentAndType]>,

    /// Is the type an union and not a struct
    pub is_union: bool,
}

impl CompositeType {
    /// Create a new composite type
    #[inline]
    pub fn new(fields: Vec<IdentAndType>, is_union: bool) -> Self {
        Self {
            name: None,
            generics: None,
            fields: fields.into_boxed_slice(),
            is_union,
        }
    }

    /// Attach a name to the composite type
    #[inline]
    pub fn with_name(&mut self, name: Ident) -> &Self {
        self.name = Some(name);
        self
    }

    /// Add generics to the type
    #[inline]
    pub fn with_generics(&mut self, generics: Generics) -> &Self {
        self.generics = Some(generics);
        self
    }
}

impl ToBox for CompositeType {}

/// List of generics associated with a type or a function
#[derive(Debug, Clone, PartialEq)]
pub struct Generics(pub Box<[GenericParam]>);

impl Generics {
    /// Create a new set of generic parameters
    #[inline]
    pub fn new(generics: Vec<GenericParam>) -> Self {
        Self(generics.into_boxed_slice())
    }
}

/// Generic parameter declared by a struct or union.
#[derive(Debug, Clone, PartialEq)]
pub enum GenericParam {
    /// Type parameter, such as `T`.
    Type(Ident),

    /// Constant parameter, such as `const SIZE: u8`.
    Const(IdentAndType),
}

/// Enum type
#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    /// Name of the enum type
    pub name: Option<Ident>,

    /// Primitive type used to represent the enum; defaults to `u8`.
    pub primitive: Primitive,

    /// Variants in the enum with their values
    pub variants: Box<[(Ident, Option<Expr>)]>,
}

impl EnumType {
    /// Create a new enum type
    #[inline]
    pub fn new(primitive: Primitive, variants: Vec<(Ident, Option<Expr>)>) -> Self {
        Self {
            name: None,
            primitive,
            variants: variants.into_boxed_slice(),
        }
    }

    /// Attach a name to the enum type
    #[inline]
    pub fn with_name(&mut self, name: Ident) -> &Self {
        self.name = Some(name);
        self
    }
}

impl ToBox for EnumType {}

/// Pair a identifier with a type
/// Note: used by fields in a composite type or as generic constant
#[derive(Debug, Clone, PartialEq)]
pub struct IdentAndType {
    pub ident: Ident,
    pub type_used: TypeUsed,
}

impl IdentAndType {
    /// Pair an identifier with a type
    pub fn new(ident: Ident, type_used: TypeUsed) -> Self {
        Self { ident, type_used }
    }
}

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

/// Open '{' token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close '}' token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

/// Parse a list of declared generic parameters, such as `<T, const SIZE: u8>`.
pub fn parser_generics<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Generics, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_generics(parser_type_used().boxed())
}

fn parse_generics<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, Generics, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let type_param = select! {
        Token::Ident(IdentOrReg::Ident(ident)) => GenericParam::Type(ident),
    };
    let const_param = just(Token::VarDecl(VarType::Const))
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident,
        })
        .then_ignore(just(Token::Colon))
        .then(type_used)
        .map(|(ident, type_used)| GenericParam::Const(IdentAndType::new(ident, type_used)));

    type_param
        .or(const_param)
        .separated_by(just(Token::Comma))
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(GEN_OPEN), just(GEN_CLOSE))
        .map(Generics::new)
}

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
        let complex = parse_named_type(type_used.clone())
            .map(|complex| TypeUsed::Named(complex.to_box()))
            .boxed();
        let composite = parse_composite_type(type_used.clone())
            .map(|composite| TypeUsed::Composite(composite.to_box()))
            .boxed();
        let enumeration = parser_enum_type()
            .map(|enumeration| TypeUsed::Enum(enumeration.to_box()))
            .boxed();
        let function = parse_function_type(type_used.clone())
            .map(|function| TypeUsed::Function(function.to_box()))
            .boxed();

        primitive
            .boxed()
            .or(array)
            .or(tuple)
            .or(function)
            .or(composite)
            .or(enumeration)
            .or(complex)
    })
}

/// Parse a struct declaration or anonymous struct type.
pub fn parser_struct_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, CompositeType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_composite_type(parser_type_used().boxed()).try_map(|composite, span| {
        if composite.is_union {
            Err(Rich::custom(span, "expected struct"))
        } else {
            Ok(composite)
        }
    })
}

/// Parse a union declaration or anonymous union type.
pub fn parser_union_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, CompositeType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_composite_type(parser_type_used().boxed()).try_map(|composite, span| {
        if composite.is_union {
            Ok(composite)
        } else {
            Err(Rich::custom(span, "expected union"))
        }
    })
}

fn parse_composite_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, CompositeType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let field = select! {
        Token::Ident(IdentOrReg::Ident(ident)) => ident,
    }
    .then_ignore(just(Token::Colon))
    .then(type_used.clone())
    .map(|(ident, type_used)| IdentAndType::new(ident, type_used));

    select! {
        Token::TypeDef(kind @ (TypeDef::Struct | TypeDef::Union)) => kind,
    }
    .then(
        select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident,
        }
        .or_not(),
    )
    .then(parse_generics(type_used.clone()).or_not())
    .then(
        field
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE)),
    )
    .map(|(((kind, name), generics), fields)| {
        let mut composite = CompositeType::new(fields, kind == TypeDef::Union);
        if let Some(name) = name {
            composite.with_name(name);
        }
        if let Some(generics) = generics {
            composite.with_generics(generics);
        }
        composite
    })
}

/// Parse an enum declaration or anonymous enum type.
pub fn parser_enum_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, EnumType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let variant = select! {
        Token::Ident(IdentOrReg::Ident(ident)) => ident,
    }
    .then(
        just(Token::Assign(Assign::new()))
            .ignore_then(parser_expr(false))
            .or_not(),
    );
    let variants = variant
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE));

    just(Token::TypeDef(TypeDef::Enum))
        .ignore_then(
            select! {
                Token::Ident(IdentOrReg::Ident(ident)) => ident,
            }
            .or_not(),
        )
        .then(
            just(Token::As)
                .ignore_then(select! {
                    Token::Primitive(primitive) => primitive,
                })
                .or_not(),
        )
        .then(variants)
        .map(|((name, primitive), variants)| {
            let mut enumeration = EnumType::new(primitive.unwrap_or(primitive::U8), variants);
            if let Some(name) = name {
                enumeration.with_name(name);
            }
            enumeration
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

/// Parse a named type such as `path::to::MyType<i16, 3>`.
pub fn parser_named_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, NamedType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_named_type(parser_type_used().boxed())
}

fn parse_named_type<'tok, 'src: 'tok, I, T>(
    type_used: T,
) -> impl Parser<'tok, I, NamedType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    T: Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let generic = type_used
        .map(GenericUsed::Type)
        .or(parser_expr(false).map(GenericUsed::Const));
    let generics = generic
        .separated_by(just(Token::Comma))
        .at_least(1)
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(GEN_OPEN), just(GEN_CLOSE));

    parser_path()
        .then(generics.or_not())
        .map(|(ident, generics)| {
            let mut complex = NamedType::new(ident);
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

    fn parse_generics_source(source: &str) -> Generics {
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_generics()
            .parse(input)
            .into_result()
            .expect("generic parameters should parse")
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
            TypeUsed::Named(
                NamedType {
                    ident: Path::new(&["path", "to", "MyType"]),
                    generics: Some(
                        vec![
                            GenericUsed::Type(TypeUsed::Prim(primitive::I16)),
                            GenericUsed::Const(Expr::Lit(Literal::Int(IntLit(3)))),
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

    #[test]
    fn parses_struct_and_union_types() {
        let TypeUsed::Composite(structure) =
            parse_type_used("struct MyType { toto: u8, tata: i16 }")
        else {
            panic!("expected struct type");
        };
        assert_eq!(structure.name, Some("MyType".into()));
        assert!(!structure.is_union);
        assert_eq!(
            structure.fields.as_ref(),
            &[
                IdentAndType::new("toto".into(), TypeUsed::Prim(primitive::U8)),
                IdentAndType::new("tata".into(), TypeUsed::Prim(primitive::I16)),
            ]
        );

        let TypeUsed::Composite(union) = parse_type_used("union MyType2 { var1: u8, var2: i8 }")
        else {
            panic!("expected union type");
        };
        assert_eq!(union.name, Some("MyType2".into()));
        assert!(union.is_union);
        assert_eq!(
            union.fields.as_ref(),
            &[
                IdentAndType::new("var1".into(), TypeUsed::Prim(primitive::U8)),
                IdentAndType::new("var2".into(), TypeUsed::Prim(primitive::I8)),
            ]
        );
    }

    #[test]
    fn parses_struct_type_and_const_generics() {
        let TypeUsed::Composite(array) =
            parse_type_used("struct Array<T, const S: u8> { array: [T; S] }")
        else {
            panic!("expected struct type");
        };
        assert_eq!(array.name, Some("Array".into()));
        assert_eq!(
            array.generics,
            Some(Generics::new(vec![
                GenericParam::Type("T".into()),
                GenericParam::Const(IdentAndType::new("S".into(), TypeUsed::Prim(primitive::U8),)),
            ]))
        );
        assert!(matches!(
            &array.fields[0].type_used,
            TypeUsed::Array(array)
                if array.cell_type == TypeUsed::Named(NamedType::new("T".into()).to_box())
                    && array.size == Expr::Var("S".into())
        ));
    }

    #[test]
    fn parses_declared_generics_with_dedicated_parser() {
        assert_eq!(
            parse_generics_source("<T, const S: i16>"),
            Generics::new(vec![
                GenericParam::Type("T".into()),
                GenericParam::Const(IdentAndType::new(
                    "S".into(),
                    TypeUsed::Prim(primitive::I16),
                )),
            ])
        );
    }

    #[test]
    fn parses_generic_union_and_nested_anonymous_struct() {
        let TypeUsed::Composite(result) = parse_type_used("union Result<O, E> { ok: O, err: E }")
        else {
            panic!("expected union type");
        };
        assert!(result.is_union);
        assert_eq!(
            result.generics,
            Some(Generics::new(vec![
                GenericParam::Type("O".into()),
                GenericParam::Type("E".into()),
            ]))
        );

        let TypeUsed::Composite(tree) =
            parse_type_used("struct Tree { branches: struct { leaf1: u8, leaf2: u8 } }")
        else {
            panic!("expected struct type");
        };
        assert_eq!(tree.name, Some("Tree".into()));
        let TypeUsed::Composite(branches) = &tree.fields[0].type_used else {
            panic!("expected nested anonymous struct");
        };
        assert_eq!(branches.name, None);
        assert!(!branches.is_union);
        assert_eq!(branches.fields.len(), 2);
    }

    #[test]
    fn rejects_keyword_as_composite_field_name() {
        let tokens = Token::lexer("struct Invalid { var: u8 }")
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input = Stream::from_iter(tokens).map(
            (0.."struct Invalid { var: u8 }".len()).into(),
            |(token, span)| (token, span),
        );

        assert!(parser_type_used().parse(input).into_result().is_err());
    }

    #[test]
    fn parses_enum_variants_and_optional_primitive() {
        let TypeUsed::Enum(enumeration) = parse_type_used("enum MyEnum { INT, FLOAT = 2, STRING }")
        else {
            panic!("expected enum type");
        };
        assert_eq!(enumeration.name, Some("MyEnum".into()));
        assert_eq!(enumeration.primitive, primitive::U8);
        assert_eq!(
            enumeration.variants.as_ref(),
            &[
                ("INT".into(), None),
                ("FLOAT".into(), Some(Expr::Lit(Literal::Int(IntLit(2))))),
                ("STRING".into(), None),
            ]
        );

        let TypeUsed::Enum(enumeration) = parse_type_used("enum Enum2 as u16 { OK, ERR, }") else {
            panic!("expected enum type");
        };
        assert_eq!(enumeration.name, Some("Enum2".into()));
        assert_eq!(enumeration.primitive, primitive::U16);
        assert_eq!(enumeration.variants.len(), 2);
    }
}
