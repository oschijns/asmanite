use std::collections::HashMap;

use crate::lexer::{define::*, token::Token};
use crate::parser::ast::{AssignOp, BinaryOp, Expr, FunctionCall, UnaryOp};
use chumsky::{
    IterParser, Parser, error::Rich, extra, input::ValueInput, pratt,
    pratt::Operator as PrattOperator, prelude::just, recursive::recursive, select,
    span::SimpleSpan,
};
use strum::IntoEnumIterator;

/// Open parenthesis token
const PAREN_OPEN: Token = Token::Delimiter(delimiter::PAREN_OPEN);

/// Close parenthesis token
const PAREN_CLOSE: Token = Token::Delimiter(delimiter::PAREN_CLOSE);

/// Parse a sequence of identifiers and path separator into a path
pub fn parser_path<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Path, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    // collect a sequence of identifiers separated by `::`
    select! {
        Token::Ident(ident) => ident,
    }
    .separated_by(just(Token::PathSep))
    .at_least(1)
    .collect::<Box<[_]>>()
    .map(Path)
}

/// Parse a function call
pub fn parser_function_call<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, FunctionCall, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let arguments = parser_expr()
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE));

    parser_path()
        .then(arguments)
        .map(|(callee, args)| FunctionCall::new(callee, args))
}

/// Parse a unary operator
pub fn parse_unary_op<'tok, 'src: 'tok, I>()
-> impl PrattOperator<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let operator = select! {
        Token::Operator(operator) if operator.op_type.is_unary() => operator,
    };

    // All unary operators share the same precedence level (for now...)
    pratt::prefix(PREC_UNARY, operator, |op, operand, _| {
        Expr::Unary(UnaryOp::new(op, operand))
    })
}

/// Parse a binary operator
pub fn parse_binary_op<'tok, 'src: 'tok, I>(
    assoc: Associativity,
    prec: Precedence,
    bitmask: u64,
) -> impl PrattOperator<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let operator = select! {
        Token::Operator(operator) if (operator.op_type.bitmask() & bitmask) != 0 => operator,
    };

    // Binary operators have various associativity and precedence
    pratt::infix(assoc.to_chumsky(prec), operator, |lhs, op, rhs, _| {
        Expr::Binary(BinaryOp::new(op, lhs, rhs))
    })
}

/// Parse a assign operation
pub fn parse_assign_op<'tok, 'src: 'tok, I>()
-> impl PrattOperator<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let assign = select! {
        Token::Assign(assign) => assign,
    };

    // All assign operations have the same precedence level and are associative from right to left
    pratt::infix(
        ASSOC_ASSIGN.to_chumsky(PREC_ASSIGN),
        assign,
        |lhs, assign, rhs, _| Expr::Assign(AssignOp::new(assign, lhs, rhs)),
    )
}

/// Read tokens to produce an expression
pub fn parser_expr<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|expr| {
        let literal = select! {
            Token::Lit(literal) => Expr::Lit(literal),
        };

        let arguments = expr
            .clone()
            .separated_by(just(Token::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE));

        let path =
            parser_path()
                .boxed()
                .then(arguments.or_not())
                .map(|(callee, args)| match args {
                    Some(args) => Expr::Call(FunctionCall::new(callee, args)),
                    None => Expr::Var(callee),
                });

        let grouped = expr
            .clone()
            .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE))
            .map(|inner| Expr::Group(Box::new(inner)));

        // node tokens to be associated with operators
        let atom = literal.or(path).or(grouped);

        // Collect parsers for operators
        // Binary operators have various level of precedences
        let bitmasks = gen_op_bitmasks();

        // Allocate storage for all the operator types
        let mut operators = Vec::with_capacity(bitmasks.len() + 2);

        // Add the binary operators, then the unary ones and the assignments
        for (&(assoc, prec), &mask) in bitmasks.iter() {
            operators.push(parse_binary_op(assoc, prec, mask).boxed());
        }
        operators.push(parse_unary_op().boxed());
        operators.push(parse_assign_op().boxed());

        // Set the possible operators encountered between atoms
        atom.pratt(operators)
    })
}

/// Iterate over the list of operators. On binary operators,
/// identify those that share the same associativity and the same precedence.
/// Construct a bitmask for each set of common operators.
fn gen_op_bitmasks() -> HashMap<(Associativity, Precedence), u64> {
    let mut masks = HashMap::with_capacity(24);
    for op_type in OpType::iter() {
        // Can the operator be used as a binary operator
        if op_type.is_binary() {
            let assoc = op_type.associativity_binary();
            let prec = op_type.precedence_binary();
            let mask = masks.entry((assoc, prec)).or_default();
            *mask |= op_type.bitmask();
        }
    }
    masks
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_expr(source: &str) -> Expr {
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_expr()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    fn parse_call(source: &str) -> FunctionCall {
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_function_call()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    #[test]
    fn parses_function_call() {
        assert_eq!(
            parse_call("my::func(a, 2)"),
            FunctionCall::new(
                Path::new(&["my", "func"]),
                vec![parse_expr("a"), parse_expr("2")]
            )
        );
    }

    #[test]
    fn parses_operator_precedence() {
        let expr = parse_expr("a + b * c");

        assert_eq!(
            expr,
            Expr::Binary(BinaryOp {
                op: Operator::new(OpType::Add),
                lhs: Box::new(Expr::Var(Path::new(&["a"]))),
                rhs: Box::new(Expr::Binary(BinaryOp {
                    op: Operator::new(OpType::Mul),
                    lhs: Box::new(Expr::Var(Path::new(&["b"]))),
                    rhs: Box::new(Expr::Var(Path::new(&["c"]))),
                })),
            })
        );
    }

    #[test]
    fn parses_unary_minus_after_binary_multiplication() {
        assert_eq!(
            parse_expr("a * -3"),
            Expr::Binary(BinaryOp::new(
                Operator::new(OpType::Mul),
                Expr::Var(Path::new(&["a"])),
                Expr::Unary(UnaryOp::new(
                    Operator::new(OpType::Sub),
                    Expr::Lit(Literal::Int(IntLit(3))),
                )),
            ))
        );
    }

    #[test]
    fn parses_left_and_non_associative_operators() {
        assert!(matches!(
            parse_expr("a - b - c"),
            Expr::Binary(BinaryOp {
                lhs,
                op: Operator {
                    op_type: OpType::Sub,
                    ..
                },
                ..
            }) if matches!(*lhs, Expr::Binary(BinaryOp { op: Operator { op_type: OpType::Sub, .. }, .. }))
        ));

        let source = "a < b < c";
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));
        assert!(parser_expr().parse(input).into_result().is_err());
    }

    #[test]
    fn parses_chained_assignments_right_associatively() {
        let a = Expr::Var(Path::new(&["a"]));
        let b = Expr::Var(Path::new(&["b"]));
        let c = Expr::Var(Path::new(&["c"]));

        assert_eq!(
            parse_expr("a = b = c"),
            Expr::Assign(AssignOp::new(
                Assign::new(),
                a.clone(),
                Expr::Assign(AssignOp::new(Assign::new(), b.clone(), c.clone())),
            ))
        );

        assert_eq!(
            parse_expr("a = b += c"),
            Expr::Assign(AssignOp::new(
                Assign::new(),
                a,
                Expr::Assign(AssignOp::new(assign::ADD, b, c)),
            ))
        );
    }

    #[test]
    fn parses_binary_operators_with_carry_flags() {
        for (source, op_type) in [
            ("a +' b", OpType::Add),
            ("a -' b", OpType::Sub),
            ("a *' b", OpType::Mul),
            ("a /' b", OpType::Div),
            ("a %' b", OpType::Rem),
            ("a <+' b", OpType::RotL),
            ("a +>' b", OpType::RotR),
        ] {
            assert_eq!(
                parse_expr(source),
                Expr::Binary(BinaryOp::new(
                    Operator::carry(op_type),
                    Expr::Var(Path::new(&["a"])),
                    Expr::Var(Path::new(&["b"])),
                )),
                "failed to parse carry operator in {source}",
            );
        }
    }

    #[test]
    fn parses_assignment_operators_with_carry_flags() {
        for (source, op_type) in [
            ("a +'= b", OpType::Add),
            ("a -'= b", OpType::Sub),
            ("a *'= b", OpType::Mul),
            ("a /'= b", OpType::Div),
            ("a %'= b", OpType::Rem),
            ("a <+'= b", OpType::RotL),
            ("a +>'= b", OpType::RotR),
        ] {
            assert_eq!(
                parse_expr(source),
                Expr::Assign(AssignOp::new(
                    Assign::op_carry(op_type),
                    Expr::Var(Path::new(&["a"])),
                    Expr::Var(Path::new(&["b"])),
                )),
                "failed to parse carry assignment in {source}",
            );
        }
    }

    #[test]
    fn parses_prefix_unary_operators_and_calls() {
        let Expr::Unary(unary) = parse_expr("+(a::b(1))") else {
            panic!("expected unary expression");
        };
        assert_eq!(unary.op.op_type, OpType::Add);

        let Expr::Group(group) = *unary.expr else {
            panic!("expected grouped expression");
        };
        assert!(matches!(*group, Expr::Call(FunctionCall { .. })));
    }
}
