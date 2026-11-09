use crate::{
    lexer::{atom::*, token::Token},
    parser::{
        composite_type::{TypeUsed, parser_type_used},
        expression::{Expr, parser_expr},
    },
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, primitive::just, select,
    span::SimpleSpan,
};

// MARK: AST

/// Block specifying where to store a bunch of variables
#[derive(Debug, Clone, PartialEq)]
pub struct InBank {
    /// Bank identifier
    pub ident: Ident,

    /// Set of variables to declare in the given bank
    pub variables: Box<[VarDecl]>,
}

impl InBank {
    #[inline]
    pub fn new(ident: Ident, vars: Vec<VarDecl>) -> Self {
        Self {
            ident,
            variables: vars.into_boxed_slice(),
        }
    }
}

/// Specify where to store the variable
#[derive(Debug, Default, Clone, PartialEq)]
pub enum MemStorage {
    /// If unspecified, the variable will fallback to the parent's scope bank.
    #[default]
    Undefined,

    /// Find an available memory location in the given bank.
    In(Ident),

    /// Fixed address for the variable.
    /// Provided address must be solvable at compile-time.
    At(Box<Expr>),
}

impl MemStorage {
    /// Create "at" variant of the variable storage
    #[inline]
    pub fn new_at(at: Expr) -> Self {
        Self::At(Box::new(at))
    }
}

/// Variable declaration.
/// This structure can be used in various context:
/// - Outside of functions for constants and static variables
/// - Inside of functions' bodies
/// - As function parameters (`decl_type` is then omitted)
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    /// Type of variable declaration
    pub var_type: VarType,

    /// Identifier of the variable
    pub ident: Ident,

    /// Type of the variable
    pub type_used: TypeUsed,

    /// Storage location for the variable
    pub storage: MemStorage,

    /// Assign a value to the variable when declaring it.
    /// Note: Only regular assignments (`=`) are allowed here.
    pub assign_expr: Option<Box<Expr>>,
}

impl VarDecl {
    /// Construct the variable declaration
    #[inline]
    pub fn new(var_type: VarType, ident: Ident, type_used: TypeUsed, storage: MemStorage) -> Self {
        Self {
            var_type,
            ident,
            type_used,
            storage,
            assign_expr: None,
        }
    }

    /// Add the assignment expression
    #[inline]
    pub fn with_assign(&mut self, assign: Expr) -> &Self {
        self.assign_expr = Some(Box::new(assign));
        self
    }
}

/// Parameter declaration
/// Handle variable declaration for function parameters
/// The `var` keyword is omitted and a register can be used instead
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDecl {
    /// Type of parameter (either variable or register)
    pub param_type: ParamType,

    /// Initialize the parameter with a default value if none provided
    pub default_value: Option<Box<Expr>>,
}

impl ParamDecl {
    /// Create a parameter
    #[inline]
    pub fn new(param_type: ParamType) -> Self {
        Self {
            param_type,
            default_value: None,
        }
    }

    /// Add initializer expression
    #[inline]
    pub fn with_default(&mut self, default: Expr) -> &Self {
        self.default_value = Some(Box::new(default));
        self
    }
}

/// Parameter is either bound to a memory location or to a register
/// Note: used in for-loop to declare the iterator
#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    /// Parameter is bounded to a memory location
    Memory(ParamMem),

    /// Parameter is bounded to a register
    Register(Ident),
}

/// Parameter bounded to a memory location
#[derive(Debug, Clone, PartialEq)]
pub struct ParamMem {
    /// Identifier of the variable
    pub ident: Ident,

    /// Type of the variable
    pub type_used: TypeUsed,

    /// Storage location for the variable
    pub storage: MemStorage,
}

impl ParamMem {
    /// Construct a parameter bound to a memory location
    #[inline]
    pub fn new(ident: Ident, type_used: TypeUsed, storage: MemStorage) -> Self {
        Self {
            ident,
            type_used,
            storage,
        }
    }
}

/// Return type is either bound to a memory location or to a register
#[derive(Debug, Clone, PartialEq)]
pub enum ReturnType {
    /// Parameter is bounded to a memory location
    Memory(ReturnMem),

    /// Parameter is bounded to a register
    Register(Ident),
}

/// Return type bounded to a memory location
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnMem {
    /// Type of the variable
    pub type_used: TypeUsed,

    /// Storage location for the variable
    pub storage: MemStorage,
}

impl ReturnMem {
    /// Construct a parameter bound to a memory location
    #[inline]
    pub fn new(type_used: TypeUsed, storage: MemStorage) -> Self {
        Self { type_used, storage }
    }
}

// MARK: Parsers

/// Open brace token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close brace token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

/// Parse a bank block containing variable declarations.
pub fn parser_in_bank<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, InBank, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! { Token::Ident(ident) => ident })
        .then(
            parser_var_decl()
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE)),
        )
        .map(|(ident, variables)| InBank::new(ident, variables))
}

/// Parse a variable, static, or constant declaration.
pub fn parser_var_decl<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, VarDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    // Parse a type usage
    let type_used = parser_type_used();

    // Type of variable declared (var, static, const)
    // followed by its identifier and by its type.
    // Optionnaly, we may specify where the variable
    // is stored and a value at initialization.
    select! { Token::VarDecl(decl_type) => decl_type }
        .then(select! { Token::Ident(ident) => ident })
        .then_ignore(just(Token::Colon))
        .then(type_used)
        .then(parser_mem_storage())
        .then(
            just(Token::Assign(Assign::new()))
                .ignore_then(parser_expr(true))
                .or_not(),
        )
        .then_ignore(just(Token::Semicolon))
        .map(|((((decl_type, ident), var_type), storage), assign_expr)| {
            let mut var = VarDecl::new(decl_type, ident, var_type, storage);
            if let Some(assign) = assign_expr {
                var.with_assign(assign);
            }
            var
        })
}

/// Parse a parameter's type, either a named memory parameter or a register.
pub fn parser_param_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, ParamType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let memory = select! { Token::Ident(ident) => ident }
        .then_ignore(just(Token::Colon))
        .then(parser_type_used())
        .then(parser_mem_storage())
        .map(|((ident, type_used), storage)| {
            ParamType::Memory(ParamMem::new(ident, type_used, storage))
        });

    memory.or(select! { Token::Ident(register) => ParamType::Register(register) })
}

/// Parse a parameter declaration, optionally initialized with a default value.
pub fn parser_param_decl<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, ParamDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parser_param_type()
        .then(
            just(Token::Assign(Assign::new()))
                .ignore_then(parser_expr(true))
                .or_not(),
        )
        .map(|(param_type, default_value)| {
            let mut declaration = ParamDecl::new(param_type);
            declaration.default_value = default_value.map(Box::new);
            declaration
        })
}

/// Parse a return type, either a memory location or a register.
pub fn parser_return_type<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, ReturnType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let memory = parser_type_used()
        .then(parser_mem_storage())
        .map(|(type_used, storage)| ReturnType::Memory(ReturnMem::new(type_used, storage)));

    memory.or(select! { Token::Ident(register) => ReturnType::Register(register) })
}

/// Parse an optional memory placement clause.
fn parser_mem_storage<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, MemStorage, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! { Token::Ident(ident) => MemStorage::In(ident) })
        .or(just(Token::At)
            .ignore_then(parser_expr(false))
            .map(MemStorage::new_at))
        .or_not()
        .map(Option::unwrap_or_default)
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lexer::token::Token,
        parser::statement::{Statement, StmtInner, parser_statement},
    };
    use chumsky::input::{Input as _, Stream};
    use logos::Logos as _;

    fn token_input(
        source: &str,
    ) -> impl chumsky::input::ValueInput<'static, Token = Token, Span = chumsky::span::SimpleSpan>
    {
        let tokens: Vec<_> = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()))
            .collect();
        Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span))
    }

    fn parse_statement(source: &str) -> Statement {
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_statement()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    #[test]
    fn parses_in_bank_variable_block_as_statement() {
        let statement = parse_statement("in my_bank { var a: u8; var b: u8; var c: u8; }");

        assert_eq!(statement.label, None);
        let StmtInner::Bank(bank) = statement.inner else {
            panic!("expected in-bank statement");
        };
        assert_eq!(bank.ident, Ident::new("my_bank"));
        assert_eq!(bank.variables.len(), 3);
        assert_eq!(bank.variables[0].ident, Ident::new("a"));
        assert_eq!(bank.variables[1].ident, Ident::new("b"));
        assert_eq!(bank.variables[2].ident, Ident::new("c"));
        assert!(
            bank.variables
                .iter()
                .all(|variable| variable.type_used.primitive == primitive::U8)
        );
    }

    #[test]
    fn parses_memory_and_register_parameter_declarations() {
        let named = parser_param_decl()
            .parse(token_input("toto: u8 in zeropage"))
            .into_result()
            .expect("memory parameter should parse");
        assert_eq!(
            named.param_type,
            ParamType::Memory(ParamMem::new(
                Ident::new("toto"),
                TypeUsed {
                    primitive: primitive::U8
                },
                MemStorage::In(Ident::new("zeropage")),
            ))
        );
        assert_eq!(named.default_value, None);

        let fixed = parser_param_decl()
            .parse(token_input("tata: u8 at 0x1000 = 50"))
            .into_result()
            .expect("initialized memory parameter should parse");
        let ParamType::Memory(memory) = fixed.param_type else {
            panic!("expected memory parameter");
        };
        assert_eq!(memory.ident, Ident::new("tata"));
        assert_eq!(memory.type_used.primitive, primitive::U8);
        assert!(matches!(memory.storage, MemStorage::At(_)));
        assert_eq!(
            fixed.default_value,
            Some(Box::new(Expr::Lit(Literal::Int(IntLit(50)))))
        );

        let register = parser_param_decl()
            .parse(token_input("a = 20"))
            .into_result()
            .expect("initialized register parameter should parse");
        assert_eq!(register.param_type, ParamType::Register(Ident::new("a")));
        assert_eq!(
            register.default_value,
            Some(Box::new(Expr::Lit(Literal::Int(IntLit(20)))))
        );

        let register = parser_param_decl()
            .parse(token_input("x"))
            .into_result()
            .expect("register parameter should parse");
        assert_eq!(register.param_type, ParamType::Register(Ident::new("x")));
        assert_eq!(register.default_value, None);
    }

    #[test]
    fn parses_memory_and_register_return_types() {
        let banked = parser_return_type()
            .parse(token_input("u8 in zeropage"))
            .into_result()
            .expect("banked return type should parse");
        assert_eq!(
            banked,
            ReturnType::Memory(ReturnMem::new(
                TypeUsed {
                    primitive: primitive::U8
                },
                MemStorage::In(Ident::new("zeropage")),
            ))
        );

        let fixed = parser_return_type()
            .parse(token_input("u8 at 0x0100"))
            .into_result()
            .expect("fixed return type should parse");
        assert!(matches!(
            fixed,
            ReturnType::Memory(ReturnMem {
                type_used: TypeUsed {
                    primitive: primitive::U8
                },
                storage: MemStorage::At(_)
            })
        ));

        for register in ["a", "x"] {
            assert_eq!(
                parser_return_type()
                    .parse(token_input(register))
                    .into_result()
                    .expect("register return type should parse"),
                ReturnType::Register(Ident::new(register))
            );
        }
    }
}
