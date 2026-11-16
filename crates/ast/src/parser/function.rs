use crate::{
    lexer::{
        atom::*,
        token::{IdentOrReg, Token},
    },
    parser::{
        composite_type::{Generics, parser_generics},
        memory::{
            MemStorage, ParamDecl, ReturnType, parser_mem_storage, parser_param_decl,
            parser_return_type,
        },
        statement::{Statement, parser_block, parser_statement},
    },
};
use asmanite_common::{ToBox, ident::Ident};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, prelude::just, select,
    span::SimpleSpan,
};

// MARK: AST

/// Function declaration
#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    /// Define if the function is inline or recursive
    pub marker: FuncMarker,

    /// Function identifier
    pub ident: Ident,

    /// Generics associated with the function
    pub generics: Option<Generics>,

    /// Where to place the function
    pub storage: MemStorage,

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
    pub fn new(
        ident: Ident,
        storage: MemStorage,
        parameters: Vec<ParamDecl>,
        block: Vec<Statement>,
    ) -> Self {
        Self {
            marker: FuncMarker::None,
            ident,
            generics: None,
            storage,
            parameters: parameters.into_boxed_slice(),
            return_type: None,
            block: block.into_boxed_slice(),
        }
    }

    /// Add generics to the function
    #[inline]
    pub fn with_generics(&mut self, generics: Generics) -> &Self {
        self.generics = Some(generics);
        self
    }

    /// Add the return type if any
    #[inline]
    pub fn with_return(&mut self, return_type: ReturnType) -> &Self {
        self.return_type = Some(return_type.to_box());
        self
    }
}

impl ToBox for FuncDecl {}

/// Function marker (inline, recursive, etc.)
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FuncMarker {
    /// Function has no special marker
    #[default]
    None,

    /// Function is inline
    /// Note: This option is incompatible with a storage specifier.
    Inline,

    /// Function is recursive
    Recursive,
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
    let marker = select! {
        Token::Inline => FuncMarker::Inline,
        Token::Recursive => FuncMarker::Recursive,
    }
    .or_not()
    .map(|marker| marker.unwrap_or_default());

    let function = select! {
        Token::Ident(IdentOrReg::Ident(ident)) => ident
    }
    .then(parser_generics().or_not())
    .then(parser_mem_storage())
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
    .map(
        |(((((ident, generics), storage), parameters), return_type), block)| {
            let mut func = FuncDecl::new(ident, storage, parameters, block);
            if let Some(generics) = generics {
                func.with_generics(generics);
            }
            if let Some(rt) = return_type {
                func.with_return(rt);
            }
            func
        },
    );

    marker
        .then_ignore(just(Token::Func))
        .then(function)
        .map(|(marker, mut function)| {
            function.marker = marker;
            function
        })
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::init_test_registers;
    use crate::parser::composite_type::{GenericParam, IdentAndType, TypeUsed};
    use crate::parser::memory::{MemStorage, ParamMem, ParamStorage, ReturnMem};
    use crate::parser::statement::StmtInner;
    use crate::parser::{expression::Expr, memory::ParamType};
    use asmanite_common::literal::{IntLit, Literal};
    use asmanite_common::primitive::def;
    use chumsky::input::{Input as _, Stream};
    use logos::Logos as _;

    fn parse_function(source: &str) -> FuncDecl {
        init_test_registers();
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
        init_test_registers();
        let function = parse_function("fn my_func(toto: u8 in zeropage, a = 10) -> x { return; }");

        assert_eq!(function.ident, "my_func".into());
        assert_eq!(function.parameters.len(), 2);
        assert_eq!(
            function.parameters[0].param_type,
            ParamType::Memory(ParamMem::new(
                "toto".into(),
                TypeUsed::Prim(def::U8,),
                ParamStorage::In("zeropage".into()),
            ))
        );
        assert_eq!(
            function.parameters[1].param_type,
            ParamType::Register("a".into())
        );
        assert_eq!(
            function.return_type,
            Some(ReturnType::Register("x".into()).to_box())
        );
        assert_eq!(function.block.len(), 1);
        assert!(matches!(function.block[0].inner, StmtInner::Return(_)));
    }

    #[test]
    fn parses_function_without_return_type_or_statements() {
        let function = parse_function("fn empty() {}");

        assert_eq!(function.marker, FuncMarker::None);
        assert_eq!(function.ident, "empty".into());
        assert_eq!(function.storage, MemStorage::Undefined);
        assert!(function.parameters.is_empty());
        assert_eq!(function.return_type, None);
        assert!(function.block.is_empty());
    }

    #[test]
    fn parses_inline_and_recursive_function_markers() {
        let inline = parse_function("inline fn my_inline() {}");
        assert_eq!(inline.marker, FuncMarker::Inline);
        assert_eq!(inline.ident, "my_inline".into());

        let recursive = parse_function("recursive fn my_rec() {}");
        assert_eq!(recursive.marker, FuncMarker::Recursive);
        assert_eq!(recursive.ident, "my_rec".into());
    }

    #[test]
    fn parses_function_storage_placement() {
        let banked = parse_function("fn my_func in prog_bank (param: u8 in zeropage) -> a { }");
        assert_eq!(banked.storage, MemStorage::In("prog_bank".into()));
        assert_eq!(banked.parameters.len(), 1);
        assert_eq!(
            banked.parameters[0].param_type,
            ParamType::Memory(ParamMem::new(
                "param".into(),
                TypeUsed::Prim(def::U8),
                ParamStorage::In("zeropage".into()),
            ))
        );
        assert_eq!(
            banked.return_type,
            Some(ReturnType::Register("a".into()).to_box())
        );

        let fixed = parse_function("fn my_other at 0x1000 () -> x { }");
        assert_eq!(
            fixed.storage,
            MemStorage::At(Expr::Lit(Literal::Int(IntLit(0x1000))).to_box())
        );
        assert_eq!(
            fixed.return_type,
            Some(ReturnType::Register("x".into()).to_box())
        );
    }

    #[test]
    fn parses_function_generics_before_storage() {
        let function = parse_function("fn my_func<T, const S: i16> in prog_bank () -> x { }");

        assert_eq!(
            function.generics,
            Some(crate::parser::composite_type::Generics::new(vec![
                GenericParam::Type("T".into()),
                GenericParam::Const(IdentAndType::new("S".into(), TypeUsed::Prim(def::I16),)),
            ]))
        );
        assert_eq!(function.storage, MemStorage::In("prog_bank".into()));
        assert_eq!(
            function.return_type,
            Some(ReturnType::Register("x".into()).to_box())
        );
    }

    #[test]
    fn parses_register_parameters_and_trailing_comma() {
        init_test_registers();
        let function = parse_function("fn add(a, x,) {}");

        assert_eq!(function.parameters.len(), 2);
        assert_eq!(
            function.parameters[0].param_type,
            ParamType::Register("a".into())
        );
        assert_eq!(
            function.parameters[1].param_type,
            ParamType::Register("x".into())
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

        let ParamType::Memory(parameter) = &function.parameters[0].param_type else {
            panic!("expected memory parameter");
        };
        assert_eq!(parameter.ident, "source".into());
        //assert_eq!(parameter.type_used.primitive, primitive::U16);
        assert!(matches!(parameter.storage, ParamStorage::At(_)));
        assert_eq!(
            function.parameters[0].default_value,
            Some(Expr::Lit(Literal::Int(IntLit(7))).to_box())
        );
        assert_eq!(
            function.return_type,
            Some(
                ReturnType::Memory(ReturnMem::new(
                    TypeUsed::Prim(def::U8,),
                    ParamStorage::In("zeropage".into()),
                ))
                .to_box()
            )
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
        //assert_eq!(return_type.type_used.primitive, primitive::U16);
        assert!(matches!(return_type.storage, ParamStorage::At(_)));
    }
}
