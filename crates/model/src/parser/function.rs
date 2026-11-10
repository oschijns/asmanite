use crate::{
    lexer::{atom::*, token::Token},
    parser::{
        memory::{ParamDecl, ReturnType, parser_param_decl, parser_return_type},
        statement::{Statement, parser_block, parser_statement},
    },
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, prelude::just, select,
    span::SimpleSpan,
};

// MARK: AST

/// Function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    /// Function identifier
    pub ident: Ident,

    /// List of parameters
    pub parameters: Box<[ParamDecl]>,

    /// Return type of the function
    pub return_type: Option<Box<ReturnType>>,

    /// Block of code in the function
    pub block: Box<[Statement]>,
}

impl FuncDecl {
    /// Declare a function
    #[inline]
    pub fn new(ident: Ident, parameters: Vec<ParamDecl>, block: Vec<Statement>) -> Self {
        Self {
            ident,
            parameters: parameters.into_boxed_slice(),
            return_type: None,
            block: block.into_boxed_slice(),
        }
    }

    /// Add the return type if any
    #[inline]
    pub fn with_return(&mut self, return_type: ReturnType) -> &Self {
        self.return_type = Some(Box::new(return_type));
        self
    }
}

// MARK: Parsers

/// Open parenthesis token
const PAREN_OPEN: Token = Token::Delimiter(delimiter::PAREN_OPEN);

/// Close parenthesis token
const PAREN_CLOSE: Token = Token::Delimiter(delimiter::PAREN_CLOSE);

/// Parse a function declaration.
#[inline]
pub fn parser_function<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, FuncDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parser_function_with_statement(parser_statement())
}

pub(crate) fn parser_function_with_statement<'tok, 'src: 'tok, I, S>(
    statement: S,
) -> impl Parser<'tok, I, FuncDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    S: Parser<'tok, I, Statement, extra::Err<Rich<'tok, Token>>>,
{
    just(Token::Func)
        .ignore_then(select! { Token::Ident(ident) => ident })
        .then(
            parser_param_decl()
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE)),
        )
        .then(
            just(Token::Arrow)
                .ignore_then(parser_return_type())
                .or_not(),
        )
        .then(parser_block(statement))
        .map(|(((ident, parameters), return_type), block)| {
            let mut func = FuncDecl::new(ident, parameters, block);
            if let Some(rt) = return_type {
                func.with_return(rt);
            }
            func
        })
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expression::Expr;
    use crate::parser::statement::StmtInner;
    use chumsky::input::{Input as _, Stream};
    use logos::Logos as _;

    fn parse_function(source: &str) -> FuncDecl {
        let tokens: Vec<_> = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()))
            .collect();
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_function()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    #[test]
    fn parses_function_with_parameters_return_type_and_statements() {
        let function = parse_function("fn my_func(toto: u8 in zeropage, a = 10) -> x { return; }");

        assert_eq!(function.ident, Ident::new("my_func"));
        assert_eq!(function.parameters.len(), 2);
        assert_eq!(
            function.parameters[0].param_type,
            crate::parser::memory::ParamType::Memory(crate::parser::memory::ParamMem::new(
                Ident::new("toto"),
                crate::parser::composite_type::TypeUsed {
                    primitive: primitive::U8,
                },
                crate::parser::memory::MemStorage::In(Ident::new("zeropage")),
            ))
        );
        assert_eq!(
            function.parameters[1].param_type,
            crate::parser::memory::ParamType::Register(Ident::new("a"))
        );
        assert_eq!(
            function.return_type,
            Some(Box::new(ReturnType::Register(Ident::new("x"))))
        );
        assert_eq!(function.block.len(), 1);
        assert!(matches!(function.block[0].inner, StmtInner::Return(_)));
    }

    #[test]
    fn parses_function_without_return_type_or_statements() {
        let function = parse_function("fn empty() {}");

        assert_eq!(function.ident, Ident::new("empty"));
        assert!(function.parameters.is_empty());
        assert_eq!(function.return_type, None);
        assert!(function.block.is_empty());
    }

    #[test]
    fn parses_register_parameters_and_trailing_comma() {
        let function = parse_function("fn add(a, x,) {}");

        assert_eq!(function.parameters.len(), 2);
        assert_eq!(
            function.parameters[0].param_type,
            crate::parser::memory::ParamType::Register(Ident::new("a"))
        );
        assert_eq!(
            function.parameters[1].param_type,
            crate::parser::memory::ParamType::Register(Ident::new("x"))
        );
        assert!(
            function
                .parameters
                .iter()
                .all(|param| param.default_value.is_none())
        );
        assert_eq!(function.return_type, None);
    }

    #[test]
    fn parses_function_with_fixed_address_parameter_and_memory_return() {
        let function = parse_function("fn read(source: u16 at 0x1000 = 7) -> u8 in zeropage {}");

        let crate::parser::memory::ParamType::Memory(parameter) =
            &function.parameters[0].param_type
        else {
            panic!("expected memory parameter");
        };
        assert_eq!(parameter.ident, Ident::new("source"));
        assert_eq!(parameter.type_used.primitive, primitive::U16);
        assert!(matches!(
            parameter.storage,
            crate::parser::memory::MemStorage::At(_)
        ));
        assert_eq!(
            function.parameters[0].default_value,
            Some(Box::new(Expr::Lit(Literal::Int(IntLit(7)))))
        );
        assert_eq!(
            function.return_type,
            Some(Box::new(ReturnType::Memory(
                crate::parser::memory::ReturnMem::new(
                    crate::parser::composite_type::TypeUsed {
                        primitive: primitive::U8,
                    },
                    crate::parser::memory::MemStorage::In(Ident::new("zeropage")),
                )
            )))
        );
    }

    #[test]
    fn parses_function_with_fixed_address_return_type() {
        let function = parse_function("fn get_value() -> u16 at 0x0100 {}");

        let Some(return_type) = function.return_type else {
            panic!("expected a return type");
        };
        let ReturnType::Memory(return_type) = *return_type else {
            panic!("expected a memory return type");
        };
        assert_eq!(return_type.type_used.primitive, primitive::U16);
        assert!(matches!(
            return_type.storage,
            crate::parser::memory::MemStorage::At(_)
        ));
    }
}
