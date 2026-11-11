use crate::{
    ToBox,
    lexer::{
        atom::*,
        register::RegIdent,
        token::{IdentOrReg, Token},
    },
    parser::{
        composite_type::{TypeUsed, parser_type_used},
        expression::{Expr, parser_expr},
        function::{FuncDecl, parser_function},
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

    /// Variables and functions declared in the bank.
    /// The two elements are stored in the same list to preserve the order of declaration.
    pub elements: Box<[BankElement]>,
}

impl InBank {
    #[inline]
    pub fn new(ident: Ident, elements: Vec<BankElement>) -> Self {
        Self {
            ident,
            elements: elements.into_boxed_slice(),
        }
    }
}

impl ToBox for InBank {}

/// Element in a in-bank statement
/// Can be either a variable or a function
#[derive(Debug, Clone, PartialEq)]
pub enum BankElement {
    /// Variable declaration
    Var(Box<VarDecl>),

    /// Function declaration
    Func(Box<FuncDecl>),
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

impl ToBox for VarDecl {}

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
    Register(RegIdent),
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
    Register(RegIdent),
}

impl ToBox for ReturnType {}

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

/// Parse a register identifier token.
pub fn parser_register<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, RegIdent, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    select! { Token::Ident(IdentOrReg::Reg(register)) => register }
}

/// Parse a bank block containing variable declarations.
#[inline]
pub fn parser_in_bank<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, InBank, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parser_in_bank_with_function(parser_function())
}

pub(crate) fn parser_in_bank_with_function<'tok, 'src: 'tok, I, F>(
    function: F,
) -> impl Parser<'tok, I, InBank, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    F: Parser<'tok, I, FuncDecl, extra::Err<Rich<'tok, Token>>>,
{
    let var_decl = parser_var_decl().map(|v| BankElement::Var(v.to_box()));
    let func_decl = function.map(|f| BankElement::Func(f.to_box()));
    let element = var_decl.or(func_decl);

    just(Token::In)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident
        })
        .then(
            element
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE)),
        )
        .map(|(ident, elements)| InBank::new(ident, elements))
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
        .then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident
        })
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
    let memory = select! { Token::Ident(IdentOrReg::Ident(ident)) => ident }
        .then_ignore(just(Token::Colon))
        .then(parser_type_used())
        .then(parser_mem_storage())
        .map(|((ident, type_used), storage)| {
            ParamType::Memory(ParamMem::new(ident, type_used, storage))
        });

    memory.or(parser_register().map(ParamType::Register))
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

    parser_register().map(ReturnType::Register).or(memory)
}

/// Parse an optional memory placement clause.
pub(crate) fn parser_mem_storage<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, MemStorage, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => MemStorage::In(ident)
        })
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
        lexer::{register::*, token::Token},
        parser::statement::{Statement, StmtInner, parser_statement},
    };
    use chumsky::input::{Input as _, Stream};
    use logos::Logos as _;

    fn token_input(
        source: &str,
    ) -> impl chumsky::input::ValueInput<'static, Token = Token, Span = chumsky::span::SimpleSpan>
    {
        init_test_registers();
        let tokens: Vec<_> = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()))
            .collect();
        Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span))
    }

    fn parse_statement(source: &str) -> Statement {
        init_test_registers();
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
    fn parses_in_bank_variable_and_function_block_as_statement() {
        let statement =
            parse_statement("in my_bank { var first: u8; fn b() {} var second: u8; fn d() {} }");

        assert_eq!(statement.label, None);
        let StmtInner::Bank(bank) = statement.inner else {
            panic!("expected in-bank statement");
        };
        assert_eq!(bank.ident, "my_bank".into());
        assert_eq!(bank.elements.len(), 4);
        assert!(matches!(
            &bank.elements[0],
            BankElement::Var(variable)
                if variable.ident == "first".into()
                    //&& variable.type_used.primitive == primitive::U8
        ));
        assert!(matches!(
            &bank.elements[1],
            BankElement::Func(function)
                if function.ident == "b".into()
                    && function.parameters.is_empty()
                    && function.block.is_empty()
        ));
        assert!(matches!(
            &bank.elements[2],
            BankElement::Var(variable)
                if variable.ident == "second".into()
                    //&& variable.type_used.primitive == primitive::U8
        ));
        assert!(matches!(
            &bank.elements[3],
            BankElement::Func(function)
                if function.ident == "d".into()
                    && function.parameters.is_empty()
                    && function.block.is_empty()
        ));
    }

    #[test]
    fn parses_memory_and_register_parameter_declarations() {
        init_test_registers();

        let named = parser_param_decl()
            .parse(token_input("toto: u8 in zeropage"))
            .into_result()
            .expect("memory parameter should parse");
        assert_eq!(
            named.param_type,
            ParamType::Memory(ParamMem::new(
                "toto".into(),
                TypeUsed::Prim(primitive::U8),
                MemStorage::In("zeropage".into()),
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
        assert_eq!(memory.ident, "tata".into());
        //assert_eq!(memory.type_used.primitive, primitive::U8);
        assert!(matches!(memory.storage, MemStorage::At(_)));
        assert_eq!(
            fixed.default_value,
            Some(Expr::Lit(Literal::Int(IntLit(50))).to_box())
        );

        let register = parser_param_decl()
            .parse(token_input("a = 20"))
            .into_result()
            .expect("initialized register parameter should parse");
        assert_eq!(register.param_type, ParamType::Register("a".into()));
        assert_eq!(
            register.default_value,
            Some(Expr::Lit(Literal::Int(IntLit(20))).to_box())
        );

        let register = parser_param_decl()
            .parse(token_input("y"))
            .into_result()
            .expect("register parameter should parse");
        assert_eq!(register.param_type, ParamType::Register("y".into()));

        assert!(
            parser_param_decl()
                .parse(token_input("ordinary_name"))
                .into_result()
                .is_err()
        );
        assert_eq!(register.default_value, None);
    }

    #[test]
    fn parses_memory_and_register_return_types() {
        init_test_registers();

        let banked = parser_return_type()
            .parse(token_input("u8 in zeropage"))
            .into_result()
            .expect("banked return type should parse");
        assert_eq!(
            banked,
            ReturnType::Memory(ReturnMem::new(
                TypeUsed::Prim(primitive::U8),
                MemStorage::In("zeropage".into()),
            ))
        );

        let fixed = parser_return_type()
            .parse(token_input("u8 at 0x0100"))
            .into_result()
            .expect("fixed return type should parse");
        assert!(matches!(
            fixed,
            ReturnType::Memory(ReturnMem {
                type_used: TypeUsed::Prim(primitive::U8),
                storage: MemStorage::At(_)
            })
        ));

        for register in ["x", "y"] {
            assert_eq!(
                parser_return_type()
                    .parse(token_input(register))
                    .into_result()
                    .expect("register return type should parse"),
                ReturnType::Register(register.into())
            );
        }

        assert!(matches!(
            parser_return_type()
                .parse(token_input("ordinary_type"))
                .into_result()
                .expect("ordinary named return type should remain a memory type"),
            ReturnType::Memory(ReturnMem {
                type_used: TypeUsed::Complex(complex),
                storage: MemStorage::Undefined,
            }) if complex.ident == "ordinary_type".into()
        ));
    }
}
