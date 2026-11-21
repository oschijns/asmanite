use asmanite_ast::{
    lexer::token::Token,
    parser::{
        composite_type::{TypeUsed, parser_composite_type, parser_type_alias, parser_type_used},
        expression::{Expr, parser_expr},
        function::parser_function,
        memory::{ParamType, parser_param_decl, parser_var_decl},
        module::parser_use,
        statement::{LoopType, StmtInner, parser_statement},
    },
};
use asmanite_common::span::{Span, Spanned};
use chumsky::{
    Parser,
    input::{Input, Stream},
    span::SimpleSpan,
};
use logos::Logos;

fn token_input(
    source: &str,
) -> impl chumsky::input::ValueInput<'static, Token = Token, Span = SimpleSpan> {
    let tokens: Vec<_> = Token::lexer(source)
        .spanned()
        .map(|(token, span)| (token.expect("test source should lex"), span.into()))
        .collect();
    Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span))
}

fn assert_span(span: Span, source: &str, start: usize, end: usize) {
    assert_eq!(span, Span::new(start, end));
    assert!(end <= source.len());
}

#[test]
fn expressions_and_operators_have_complete_spans() {
    let source = "1 + 2";
    let expr = parser_expr(false)
        .parse(token_input(source))
        .into_result()
        .expect("expression should parse");

    let Expr::Binary(binary) = expr else {
        panic!("expected binary expression");
    };
    assert_span(binary.span, source, 0, source.len());

    let Expr::Lit(lhs) = binary.lhs else {
        panic!("expected left literal");
    };
    assert_span(lhs.span, source, 0, 1);

    let Expr::Lit(rhs) = binary.rhs else {
        panic!("expected right literal");
    };
    assert_span(rhs.span, source, 4, 5);
}

#[test]
fn field_and_index_accesses_have_complete_spans() {
    let source = "my_other.sub_struct[ID + 1].field";
    let expr = parser_expr(false)
        .parse(token_input(source))
        .into_result()
        .expect("field and index expression should parse");

    let Expr::Access(field) = expr else {
        panic!("expected field access");
    };
    assert_span(field.span, source, 0, source.len());

    let Expr::Index(index) = field.expr else {
        panic!("expected index operation");
    };
    assert_span(
        index.span,
        source,
        0,
        source.find("].").expect("index should close") + 1,
    );

    let Expr::Access(parent) = index.expr else {
        panic!("expected field access before index");
    };
    assert_span(
        parent.span,
        source,
        0,
        source.find('[').expect("index operation should open"),
    );
}

#[test]
fn unary_cast_and_group_spans_cover_the_full_expression() {
    let source = "-(1 as u8)";
    let expr = parser_expr(false)
        .parse(token_input(source))
        .into_result()
        .expect("expression should parse");

    let Expr::Unary(unary) = expr else {
        panic!("expected unary expression");
    };
    assert_span(unary.span, source, 0, source.len());
    let Expr::Group(group) = unary.expr else {
        panic!("expected grouped expression");
    };
    assert_span(group.span, source, 1, source.len());
    let Expr::Cast(cast) = &group.atom else {
        panic!("expected cast expression");
    };
    assert_span(cast.span, source, 2, source.len() - 1);
}

#[test]
fn type_and_declaration_nodes_have_complete_spans() {
    let type_source = "[u8;3]";
    let ty = parser_type_used()
        .parse(token_input(type_source))
        .into_result()
        .expect("array type should parse");
    let TypeUsed::Array(array) = ty else {
        panic!("expected array type");
    };
    assert_span(array.span, type_source, 0, type_source.len());

    let declaration_source = "var value: u8;";
    let declaration = parser_var_decl()
        .parse(token_input(declaration_source))
        .into_result()
        .expect("variable declaration should parse");
    assert_span(
        declaration.span,
        declaration_source,
        0,
        declaration_source.len(),
    );

    let parameter_source = "value: u8 in ram = 3";
    let parameter = parser_param_decl()
        .parse(token_input(parameter_source))
        .into_result()
        .expect("parameter declaration should parse");
    assert_span(parameter.span, parameter_source, 0, parameter_source.len());
    let ParamType::Memory(memory) = parameter.param_type else {
        panic!("expected memory parameter");
    };
    assert_span(memory.span, parameter_source, 0, "value: u8 in ram".len());

    let composite_source = "struct Pair { left: u8 }";
    let composite = parser_composite_type(parser_type_used().boxed())
        .parse(token_input(composite_source))
        .into_result()
        .expect("struct type should parse");
    assert_span(composite.span, composite_source, 0, composite_source.len());

    let alias_source = "type Alias = u8;";
    let alias = parser_type_alias()
        .parse(token_input(alias_source))
        .into_result()
        .expect("type alias should parse");
    assert_span(alias.span, alias_source, 0, alias_source.len());
}

#[test]
fn use_statement_and_function_spans_include_delimiters() {
    let use_source = "use root::foo;";
    let use_node = parser_use()
        .parse(token_input(use_source))
        .into_result()
        .expect("use statement should parse");
    assert_span(use_node.span, use_source, 0, use_source.len());

    let function_source = "fn f() {}";
    let function = parser_function()
        .parse(token_input(function_source))
        .into_result()
        .expect("function should parse");
    assert_span(function.span, function_source, 0, function_source.len());
}

#[test]
fn statement_nodes_include_their_terminators() {
    let source = "return;";
    let statement = parser_statement()
        .parse(token_input(source))
        .into_result()
        .expect("return statement should parse");

    let StmtInner::Return(return_node) = &statement.inner else {
        panic!("expected return statement");
    };
    assert_span(return_node.span, source, 0, source.len());
    assert_span(statement.inner.span(), source, 0, source.len());
}

#[test]
fn loop_spans_cover_their_headers_and_bodies() {
    for source in ["loop {}", "while true {}", "do {} while true;"] {
        let statement = parser_statement()
            .parse(token_input(source))
            .into_result()
            .expect("loop statement should parse");
        let StmtInner::Loop(loop_node) = statement.inner else {
            panic!("expected loop statement");
        };
        assert_span(loop_node.span, source, 0, source.len());

        match (&loop_node.loop_type, source) {
            (LoopType::Loop, "loop {}") => {}
            (LoopType::While(header), "while true {}") => {
                assert_span(header.span, source, 0, "while true".len());
            }
            (LoopType::While(header), "do {} while true;") => {
                assert_span(header.span, source, "do {} ".len(), source.len());
            }
            _ => panic!("unexpected loop form"),
        }
    }

    let source = "for index: u8 in ram of 0..3 {}";
    let statement = parser_statement()
        .parse(token_input(source))
        .into_result()
        .expect("for loop should parse");
    let StmtInner::Loop(loop_node) = statement.inner else {
        panic!("expected loop statement");
    };
    let LoopType::For(header) = loop_node.loop_type else {
        panic!("expected for loop");
    };
    let header_end = source[..source.find('{').expect("loop body should open")]
        .trim_end()
        .len();
    assert_span(header.span, source, 0, header_end);
}
