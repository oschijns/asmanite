use super::convert_span;
use crate::{
    lexer::{
        atom::*,
        token::{IdentOrReg, Token},
    },
    parser::{SubParser, composite_type::TypeUsed, expression::Expr, function::FuncDecl},
};
use asmanite_common::{
    ToBox,
    ident::Ident,
    operator::assign::ASSIGN,
    register::RegIdent,
    span::{Span, Spanned, WithSpan},
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, primitive::just, select,
    span::SimpleSpan,
};

// MARK: AST

/// Block specifying where to store variables or functions
#[derive(Debug, Clone, PartialEq)]
pub struct InBankMod {
    /// Span of the in-bank block
    pub span: Span,

    /// Bank identifier
    pub ident: Ident,

    /// Variables and functions declared in the bank.
    /// The two elements are stored in the same list to preserve the order of declaration.
    pub elements: Box<[BankElement]>,
}

impl InBankMod {
    #[inline]
    pub fn new(span: Span, ident: Ident, elements: Vec<BankElement>) -> Self {
        Self {
            span,
            ident,
            elements: elements.into_boxed_slice(),
        }
    }
}

impl ToBox for InBankMod {}

impl Spanned for InBankMod {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Element in a in-bank statement
/// Can be either a variable or a function
#[derive(Debug, Clone, PartialEq)]
pub enum BankElement {
    /// Variable declaration
    Var(Box<VarDecl>),

    /// Function declaration
    Func(Box<FuncDecl>),
}

impl Spanned for BankElement {
    #[rustfmt::skip]
    fn span(&self) -> Span {
        match self {
            Self::Var(var) => var.span,
            Self::Func(func) => func.span,
        }
    }
}

/// Block specifying where to store variables
#[derive(Debug, Clone, PartialEq)]
pub struct InBankFunc {
    /// Span of the in-bank block
    pub span: Span,

    /// Bank identifier
    pub ident: Ident,

    /// Variables declared in the bank.
    pub variables: Box<[VarDecl]>,
}

impl InBankFunc {
    #[inline]
    pub fn new(span: Span, ident: Ident, variables: Vec<VarDecl>) -> Self {
        Self {
            span,
            ident,
            variables: variables.into_boxed_slice(),
        }
    }
}

impl ToBox for InBankFunc {}

impl Spanned for InBankFunc {
    #[inline]
    fn span(&self) -> Span {
        self.span
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
    /// Span of the variable declaration
    pub span: Span,

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
    pub fn new(
        span: Span,
        var_type: VarType,
        ident: Ident,
        type_used: TypeUsed,
        storage: MemStorage,
    ) -> Self {
        Self {
            span,
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

impl Spanned for VarDecl {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Parameter declaration
/// Handle variable declaration for function parameters
/// The `var` keyword is omitted and a register can be used instead
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDecl {
    /// Span of the parameter declaration
    pub span: Span,

    /// Type of parameter (either variable or register)
    pub param_type: ParamType,

    /// Initialize the parameter with a default value if none provided
    pub default_value: Option<Box<Expr>>,
}

impl ParamDecl {
    /// Create a parameter
    #[inline]
    pub fn new(span: Span, param_type: ParamType) -> Self {
        Self {
            span,
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

impl Spanned for ParamDecl {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Parameter is either bound to a memory location or to a register
/// Note: used in for-loop to declare the iterator
#[derive(Debug, Clone, PartialEq)]
pub enum ParamType {
    /// Parameter is bounded to a memory location
    Memory(ParamMem),

    /// Parameter is bounded to a register
    Register(WithSpan<RegIdent>),
}

impl Spanned for ParamType {
    #[rustfmt::skip]
    fn span(&self) -> Span {
        match self {
            Self::Memory  (mem) => mem.span,
            Self::Register(reg) => reg.span,
        }
    }
}

/// Parameter bounded to a memory location
#[derive(Debug, Clone, PartialEq)]
pub struct ParamMem {
    /// Span of the parameter
    pub span: Span,

    /// Identifier of the variable
    pub ident: Ident,

    /// Type of the variable
    pub type_used: TypeUsed,

    /// Storage location for the variable
    pub storage: ParamStorage,
}

impl ParamMem {
    /// Construct a parameter bound to a memory location
    #[inline]
    pub fn new(span: Span, ident: Ident, type_used: TypeUsed, storage: ParamStorage) -> Self {
        Self {
            span,
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
    Register(WithSpan<RegIdent>),
}

impl ToBox for ReturnType {}

impl Spanned for ReturnType {
    #[rustfmt::skip]
    fn span(&self) -> Span {
        match self {
            Self::Memory  (mem) => mem.span,
            Self::Register(reg) => reg.span,
        }
    }
}

/// Return type bounded to a memory location
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnMem {
    /// Span of the parameter
    pub span: Span,

    /// Type of the variable
    pub type_used: TypeUsed,

    /// Storage location for the variable
    pub storage: ParamStorage,
}

impl ReturnMem {
    /// Construct a parameter bound to a memory location
    #[inline]
    pub fn new(span: Span, type_used: TypeUsed, storage: ParamStorage) -> Self {
        Self {
            span,
            type_used,
            storage,
        }
    }
}

/// Specify where to store the function parameter or the return value
#[derive(Debug, Clone, PartialEq)]
pub enum ParamStorage {
    /// Find an available memory location in the given bank.
    In(Ident),

    /// Fixed address for the parameter.
    /// Provided address must be solvable at compile-time.
    At(Box<Expr>),

    /// Parameter is pulled or pushed into the stack.
    Stack,

    /// Parameter is evaluated at compile-time.
    /// Note: Only used for inline functions, where the parameter is replaced by its value at compile-time.
    CompTime,
}

impl ParamStorage {
    /// Create "at" variant of the parameter storage
    #[inline]
    pub fn new_at(at: Expr) -> Self {
        Self::At(Box::new(at))
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

/// Parse a bank block containing variable and function declarations.
pub fn parser_in_bank_mod<'tok, 'src: 'tok, I>(
    parse_var: SubParser<'tok, I, VarDecl>,
    parse_func: SubParser<'tok, I, FuncDecl>,
) -> impl Parser<'tok, I, InBankMod, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let var_decl = parse_var.map(|v| BankElement::Var(v.to_box()));
    let func_decl = parse_func.map(|f| BankElement::Func(f.to_box()));
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
        .map_with(|(ident, elements), extra| {
            InBankMod::new(convert_span(extra.span()), ident, elements)
        })
}

/// Parse a bank block containing variable declarations.
pub fn parser_in_bank_func<'tok, 'src: 'tok, I>(
    parse_var: SubParser<'tok, I, VarDecl>,
) -> impl Parser<'tok, I, InBankFunc, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident
        })
        .then(
            parse_var
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE)),
        )
        .map_with(|(ident, elements), extra| {
            InBankFunc::new(convert_span(extra.span()), ident, elements)
        })
}

/// Parse a variable, static, or constant declaration.
pub fn parser_var_decl<'tok, 'src: 'tok, I>(
    parse_expr: SubParser<'tok, I, Expr>,
    parse_type: SubParser<'tok, I, TypeUsed>,
    parse_mem: SubParser<'tok, I, MemStorage>,
) -> impl Parser<'tok, I, VarDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    // Type of variable declared (var, static, const)
    // followed by its identifier and by its type.
    // Optionnaly, we may specify where the variable
    // is stored and a value at initialization.
    select! { Token::VarDecl(decl_type) => decl_type }
        .then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident
        })
        .then_ignore(just(Token::Colon))
        .then(parse_type)
        .then(parse_mem)
        .then(just(Token::Assign(ASSIGN)).ignore_then(parse_expr).or_not())
        .then_ignore(just(Token::Semicolon))
        .map_with(
            |((((decl_type, ident), var_type), storage), assign_expr), extra| {
                let mut var = VarDecl::new(
                    convert_span(extra.span()),
                    decl_type,
                    ident,
                    var_type,
                    storage,
                );
                if let Some(assign) = assign_expr {
                    var.with_assign(assign);
                }
                var
            },
        )
}

/// Parse a parameter's type, either a named memory parameter or a register.
pub fn parser_param_type<'tok, 'src: 'tok, I>(
    parse_type: SubParser<'tok, I, TypeUsed>,
    parse_param: SubParser<'tok, I, ParamStorage>,
) -> impl Parser<'tok, I, ParamType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let parse_memory = || {
        select! { Token::Ident(IdentOrReg::Ident(ident)) => ident }
            .then_ignore(just(Token::Colon))
            .then(parse_type.clone())
    };

    let memory =
        parse_memory()
            .then(parse_param)
            .map_with(|((ident, type_used), storage), extra| {
                ParamType::Memory(ParamMem::new(
                    convert_span(extra.span()),
                    ident,
                    type_used,
                    storage,
                ))
            });
    let stack_memory = just(Token::VarDecl(VarType::Stack))
        .ignore_then(parse_memory())
        .map_with(|(ident, type_used), extra| {
            ParamType::Memory(ParamMem::new(
                convert_span(extra.span()),
                ident,
                type_used,
                ParamStorage::Stack,
            ))
        });
    let comptime_memory = just(Token::VarDecl(VarType::Let))
        .ignore_then(parse_memory())
        .map_with(|(ident, type_used), extra| {
            ParamType::Memory(ParamMem::new(
                convert_span(extra.span()),
                ident,
                type_used,
                ParamStorage::CompTime,
            ))
        });
    let register = parser_register().map_with(|register, extra| {
        ParamType::Register(WithSpan::new(convert_span(extra.span()), register))
    });

    memory.or(stack_memory).or(comptime_memory).or(register)
}

/// Parse a parameter declaration, optionally initialized with a default value.
pub fn parser_param_decl<'tok, 'src: 'tok, I>(
    parse_expr: SubParser<'tok, I, Expr>,
    parse_param: SubParser<'tok, I, ParamType>,
) -> impl Parser<'tok, I, ParamDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_param
        .then(just(Token::Assign(ASSIGN)).ignore_then(parse_expr).or_not())
        .map_with(|(param_type, default_value), extra| {
            let mut declaration = ParamDecl::new(convert_span(extra.span()), param_type);
            declaration.default_value = default_value.map(Box::new);
            declaration
        })
}

/// Parse a return type, either a memory location or a register.
pub fn parser_return_type<'tok, 'src: 'tok, I>(
    parse_type: SubParser<'tok, I, TypeUsed>,
    parse_param: SubParser<'tok, I, ParamStorage>,
) -> impl Parser<'tok, I, ReturnType, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let memory = parse_type
        .then(parse_param)
        .map_with(|(type_used, storage), extra| {
            ReturnType::Memory(ReturnMem::new(
                convert_span(extra.span()),
                type_used,
                storage,
            ))
        });

    parser_register()
        .map_with(|register, extra| {
            ReturnType::Register(WithSpan::new(convert_span(extra.span()), register))
        })
        .or(memory)
}

/// Parse an optional memory placement clause.
pub(crate) fn parser_mem_storage<'tok, 'src: 'tok, I>(
    parse_expr_const: SubParser<'tok, I, Expr>,
) -> impl Parser<'tok, I, MemStorage, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => MemStorage::In(ident)
        })
        .or(just(Token::At)
            .ignore_then(parse_expr_const)
            .map(MemStorage::new_at))
        .or_not()
        .map(Option::unwrap_or_default)
}

/// Parse a required memory placement clause for a parameter or return value.
pub(crate) fn parser_param_storage<'tok, 'src: 'tok, I>(
    parse_expr_const: SubParser<'tok, I, Expr>,
) -> impl Parser<'tok, I, ParamStorage, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::In)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ParamStorage::In(ident)
        })
        .or(just(Token::At)
            .ignore_then(parse_expr_const)
            .map(ParamStorage::new_at))
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        lexer::{init_test_registers, token::Token},
        parser::statement::{Statement, StmtInner, parser_statement},
    };
    use asmanite_common::{
        literal::{IntLit, Literal},
        primitive::def,
        span::UNSPANNED,
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
                UNSPANNED,
                "toto".into(),
                TypeUsed::Prim(WithSpan::unspanned(def::U8)),
                ParamStorage::In("zeropage".into()),
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
        assert!(matches!(memory.storage, ParamStorage::At(_)));
        assert_eq!(
            fixed.default_value,
            Some(Expr::Lit(WithSpan::unspanned(Literal::Int(IntLit(50)))).to_box())
        );

        let stack = parser_param_decl()
            .parse(token_input("stack bb: u8"))
            .into_result()
            .expect("stack parameter should parse");
        assert_eq!(
            stack.param_type,
            ParamType::Memory(ParamMem::new(
                UNSPANNED,
                "bb".into(),
                TypeUsed::Prim(WithSpan::unspanned(def::U8)),
                ParamStorage::Stack,
            ))
        );

        let comptime_param = parser_param_decl()
            .parse(token_input("let bb: u8"))
            .into_result()
            .expect("compile-time parameter should parse");
        assert_eq!(
            comptime_param.param_type,
            ParamType::Memory(ParamMem::new(
                UNSPANNED,
                "bb".into(),
                TypeUsed::Prim(WithSpan::unspanned(def::U8)),
                ParamStorage::CompTime,
            ))
        );

        assert!(
            parser_param_decl()
                .parse(token_input("bb: u8"))
                .into_result()
                .is_err()
        );

        let register = parser_param_decl()
            .parse(token_input("a = 20"))
            .into_result()
            .expect("initialized register parameter should parse");
        assert_eq!(
            register.param_type,
            ParamType::Register(WithSpan::unspanned("a".into()))
        );
        assert_eq!(
            register.default_value,
            Some(Expr::Lit(WithSpan::unspanned(Literal::Int(IntLit(20)))).to_box())
        );

        let register = parser_param_decl()
            .parse(token_input("y"))
            .into_result()
            .expect("register parameter should parse");
        assert_eq!(
            register.param_type,
            ParamType::Register(WithSpan::unspanned("y".into()))
        );

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
                UNSPANNED,
                TypeUsed::Prim(WithSpan::unspanned(def::U8)),
                ParamStorage::In("zeropage".into()),
            ))
        );

        let fixed = parser_return_type()
            .parse(token_input("u8 at 0x0100"))
            .into_result()
            .expect("fixed return type should parse");
        assert!(matches!(
            fixed,
            ReturnType::Memory(ReturnMem {
                type_used: TypeUsed::Prim(_),
                storage: ParamStorage::At(_),
                ..
            })
        ));

        for register in ["x", "y"] {
            assert_eq!(
                parser_return_type()
                    .parse(token_input(register))
                    .into_result()
                    .expect("register return type should parse"),
                ReturnType::Register(WithSpan::unspanned(register.into()))
            );
        }

        assert!(
            parser_return_type()
                .parse(token_input("ordinary_type"))
                .into_result()
                .is_err()
        );
    }
}
