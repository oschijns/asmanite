use crate::{
    lexer::{atom::*, token::Token},
    parser::{
        expression::Expr,
        statement::{VarDecl, VarStorage, parser_var_decl},
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

/// Parameter declaration
/// Handle variable declaration for function parameters
/// The `var` keyword is omitted and a register can be used instead
#[derive(Debug, Clone, PartialEq)]
pub enum ParamDecl {
    /// Declare a variable with a memory location
    Var(VarDecl),

    /// Use a register directly
    Register {
        /// Register to use
        register: Ident,

        /// Initialize the register with a value
        init_expr: Option<Box<Expr>>,
    },
}

impl ParamDecl {
    /// Create a parameter binded to a memory location
    #[inline]
    pub fn new_var(ident: Ident, var_type: Primitive, storage: VarStorage) -> Self {
        Self::Var(VarDecl::new(VarDeclType::Var, ident, var_type, storage))
    }

    /// Create a parameter binded to a register
    #[inline]
    pub fn new_reg(register: Ident) -> Self {
        Self::Register {
            register,
            init_expr: None,
        }
    }

    /// Add initializer expression
    #[inline]
    pub fn with_init(&mut self, init: Expr) -> &Self {
        match self {
            Self::Var(var) => {
                var.with_assign(init);
            }
            Self::Register {
                register: _,
                init_expr,
            } => {
                *init_expr = Some(Box::new(init));
            }
        }
        self
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
                .all(|variable| variable.var_type == primitive::U8)
        );
    }
}
