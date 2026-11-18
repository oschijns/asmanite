use super::convert_span;
use crate::{
    lexer::{atom::*, token::Token},
    parser::{memory::parser_register, module::parser_path},
};
use asmanite_common::{
    ToBox,
    literal::Literal,
    operator::{Assign, OpType, Operator},
    path::Path,
    primitive::Primitive,
    register::RegIdent,
    span::{Span, Spanned, WithSpan},
    status::StatusFlag,
};
use chumsky::{
    IterParser, Parser, error::Rich, extra, input::ValueInput, pratt,
    pratt::Operator as PrattOperator, prelude::just, recursive::recursive, select,
    span::SimpleSpan,
};
use std::collections::HashMap;
use strum::IntoEnumIterator;

// MARK: AST

/// Expression AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal
    Lit(WithSpan<Literal>),

    /// Status flag
    Status(WithSpan<StatusFlag>),

    /// Hardware register
    Register(WithSpan<RegIdent>),

    /// Path to variable identifier
    Var(WithSpan<Path>),

    /// Unary operation
    Unary(Box<UnaryOp>),

    /// Binary operation
    Binary(Box<BinaryOp>),

    /// Assign operation
    Assign(Box<AssignOp>),

    /// Indexing operation
    Index(Box<IndexOp>),

    /// Function call
    Call(Box<FunctionCall>),

    /// Cast an expression into another type primitive
    Cast(Box<TypeCast>),

    /// Parenthesized expression
    Group(Box<WithSpan<Self>>),
}

impl ToBox for Expr {}

impl Spanned for Expr {
    #[rustfmt::skip]
    fn span(&self) -> Span {
        match self {
            Self::Lit     (lit   ) => lit   .span,
            Self::Status  (status) => status.span,
            Self::Register(reg   ) => reg   .span,
            Self::Var     (var   ) => var   .span,
            Self::Unary   (unary ) => unary .span,
            Self::Binary  (binary) => binary.span,
            Self::Assign  (assign) => assign.span,
            Self::Index   (index ) => index .span,
            Self::Call    (func  ) => func  .span,
            Self::Cast    (cast  ) => cast  .span,
            Self::Group   (group ) => group .span,
        }
    }
}

/// Unary operation
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOp {
    /// Span of the unary operation
    pub span: Span,

    /// Operator applied to the expression
    pub op: Operator,

    /// Operand
    pub expr: Expr,
}

impl UnaryOp {
    #[inline]
    pub fn new(span: Span, op: Operator, expr: Expr) -> Self {
        Self { span, op, expr }
    }
}
impl ToBox for UnaryOp {}

impl Spanned for UnaryOp {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Binary operation
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    /// Span of the binary operation
    pub span: Span,

    /// Operator between the operands
    pub op: Operator,

    /// Left operand
    pub lhs: Expr,

    /// Right operand
    pub rhs: Expr,
}

impl BinaryOp {
    #[inline]
    pub fn new(span: Span, op: Operator, lhs: Expr, rhs: Expr) -> Self {
        Self { span, op, lhs, rhs }
    }
}

impl ToBox for BinaryOp {}

impl Spanned for BinaryOp {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Assign operation
#[derive(Debug, Clone, PartialEq)]
pub struct AssignOp {
    /// Span of the assignment
    pub span: Span,

    /// Operator between the operands
    pub assign: Assign,

    /// Left operand
    pub lhs: Expr,

    /// Right operand
    pub rhs: Expr,
}

impl AssignOp {
    #[inline]
    pub fn new(span: Span, assign: Assign, lhs: Expr, rhs: Expr) -> Self {
        Self {
            span,
            assign,
            lhs,
            rhs,
        }
    }
}

impl ToBox for AssignOp {}

impl Spanned for AssignOp {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Indexing operation
#[derive(Debug, Clone, PartialEq)]
pub struct IndexOp {
    /// Span of the indexing
    pub span: Span,

    /// Expression to index
    pub expr: Expr,

    /// Index expression
    pub index: Expr,
}

impl ToBox for IndexOp {}

impl Spanned for IndexOp {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Function call
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Span of the function call
    pub span: Span,

    /// Function being called
    pub callee: Path,

    /// Call arguments
    pub args: Box<[Expr]>,
}

impl FunctionCall {
    #[inline]
    pub fn new(span: Span, callee: Path, args: Vec<Expr>) -> Self {
        Self {
            span,
            callee,
            args: args.into_boxed_slice(),
        }
    }
}

impl ToBox for FunctionCall {}

impl Spanned for FunctionCall {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Type cast using the `as` keyword
#[derive(Debug, Clone, PartialEq)]
pub struct TypeCast {
    /// Span of the type cast
    pub span: Span,

    /// Expression to cast
    pub expr: Expr,

    /// Primitive type to cast into
    pub prim: Primitive,
}

impl TypeCast {
    #[inline]
    pub fn new(span: Span, expr: Expr, prim: Primitive) -> Self {
        Self { span, expr, prim }
    }
}

impl ToBox for TypeCast {}

impl Spanned for TypeCast {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

// MARK: Parsers

/// Open parenthesis token
const PAREN_OPEN: Token = Token::Delimiter(delimiter::PAREN_OPEN);

/// Close parenthesis token
const PAREN_CLOSE: Token = Token::Delimiter(delimiter::PAREN_CLOSE);

/// Open bracket token
const BRACKET_OPEN: Token = Token::Delimiter(delimiter::BRACKET_OPEN);

/// Close bracket token
const BRACKET_CLOSE: Token = Token::Delimiter(delimiter::BRACKET_CLOSE);

/// Read tokens to produce an expression.
/// Provide a flag to indicate if assignments are allowed in the expression.
/// Assignments can introduce syntax ambiguities in some contextes.
pub fn parser_expr<'tok, 'src: 'tok, I>(
    allow_assign: bool,
) -> impl Parser<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|expr| {
        let literal = select! { Token::Lit(literal) => literal }.map_with(|literal, extra| {
            Expr::Lit(WithSpan::new(convert_span(extra.span()), literal))
        });
        let status = select! { Token::Status(status) => status }.map_with(|status, extra| {
            Expr::Status(WithSpan::new(convert_span(extra.span()), status))
        });

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
                .map_with(|(callee, args), extra| {
                    let span = convert_span(extra.span());
                    match args {
                        Some(args) => Expr::Call(FunctionCall::new(span, callee, args).to_box()),
                        None => Expr::Var(WithSpan::new(span, callee)),
                    }
                });

        let grouped = expr
            .clone()
            .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE))
            .map_with(|inner, extra| {
                Expr::Group(Box::new(WithSpan::new(convert_span(extra.span()), inner)))
            });

        // node tokens to be associated with operators
        let register = parser_register()
            .map_with(|register, extra| {
                Expr::Register(WithSpan::new(convert_span(extra.span()), register))
            })
            .boxed();
        let atom = literal.or(status).or(register).or(path).or(grouped);

        // Collect parsers for operators
        // Binary operators have various level of precedences
        let bitmasks = gen_op_bitmasks();

        // Allocate storage for all the operator types
        let mut operators = Vec::with_capacity(bitmasks.len() + 3);

        // Add the binary operators, then the unary ones and the assignments.
        operators.push(parse_type_cast().boxed());
        operators.push(parse_unary_op().boxed());
        for (&(assoc, prec), &mask) in bitmasks.iter() {
            operators.push(parse_binary_op(assoc, prec, mask).boxed());
        }
        if allow_assign {
            operators.push(parse_assign_op().boxed());
        }

        // Set the possible operators encountered between atoms
        atom.pratt(operators)
    })
}

/// Parse an index operation
pub fn parser_index_op<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, IndexOp, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parser_expr(true)
        .then_ignore(just(BRACKET_OPEN))
        .then(parser_expr(true))
        .then_ignore(just(BRACKET_CLOSE))
        .map_with(|(expr, index), extra| IndexOp {
            span: convert_span(extra.span()),
            expr,
            index,
        })
}

/// Parse a function call
pub fn parser_function_call<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, FunctionCall, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let arguments = parser_expr(true)
        .separated_by(just(Token::Comma))
        .allow_trailing()
        .collect::<Vec<_>>()
        .delimited_by(just(PAREN_OPEN), just(PAREN_CLOSE));

    parser_path()
        .then(arguments)
        .map_with(|(callee, args), extra| {
            FunctionCall::new(convert_span(extra.span()), callee, args)
        })
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
    pratt::prefix(PREC_UNARY, operator, |op, operand, extra| {
        Expr::Unary(UnaryOp::new(convert_span(extra.span()), op, operand).to_box())
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
        Token::Operator(operator) if (op_bitmask(operator.op_type) & bitmask) != 0 => operator,
    };

    // Binary operators have various associativity and precedence
    pratt::infix(assoc.to_chumsky(prec), operator, |lhs, op, rhs, extra| {
        Expr::Binary(BinaryOp::new(convert_span(extra.span()), op, lhs, rhs).to_box())
    })
}

/// Parse a assign operation
pub fn parse_assign_op<'tok, 'src: 'tok, I>()
-> impl PrattOperator<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let assign = select! { Token::Assign(assign) => assign };

    // All assign operations have the same precedence level and are associative from right to left
    pratt::infix(
        ASSOC_ASSIGN.to_chumsky(PREC_ASSIGN),
        assign,
        |lhs, assign, rhs, extra| {
            Expr::Assign(AssignOp::new(convert_span(extra.span()), assign, lhs, rhs).to_box())
        },
    )
}

/// Parse a type cast operation
pub fn parse_type_cast<'tok, 'src: 'tok, I>()
-> impl PrattOperator<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let cast = just(Token::As).ignore_then(select! {
        Token::Primitive(primitive) => primitive,
    });

    pratt::postfix(PREC_TYPE_CAST, cast, |expr, prim, extra| {
        Expr::Cast(TypeCast::new(convert_span(extra.span()), expr, prim).to_box())
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
            let assoc = associativity_binary(op_type);
            let prec = precedence_binary(op_type);
            let mask = masks.entry((assoc, prec)).or_default();
            *mask |= op_bitmask(op_type);
        }
    }
    masks
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use crate::lexer::init_test_registers;

    use super::*;
    use asmanite_common::{
        literal::IntLit,
        operator::{
            OpType,
            assign::{self, ASSIGN},
        },
        primitive::def,
    };
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_expr(source: &str) -> Expr {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_expr(true)
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    fn parse_call(source: &str) -> FunctionCall {
        init_test_registers();
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

    fn parse_index(source: &str) -> IndexOp {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_index_op()
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    #[test]
    fn parses_function_call() {
        assert_eq!(
            parse_call("my::func(a, 2)"),
            FunctionCall::new("my::func".into(), vec![parse_expr("a"), parse_expr("2")])
        );
    }

    #[test]
    fn parses_index_operation() {
        assert_eq!(
            parse_index("a[1 + 2]"),
            IndexOp {
                expr: Expr::Register("a".into()),
                index: Expr::Binary(
                    BinaryOp::new(
                        Operator::new(OpType::Add),
                        Expr::Lit(Literal::Int(IntLit(1))),
                        Expr::Lit(Literal::Int(IntLit(2))),
                    )
                    .to_box(),
                ),
            }
        );
    }

    #[test]
    fn parses_status_flags() {
        for (source, status) in [
            ("zero", StatusFlag::Zero),
            ("negative", StatusFlag::Negative),
            ("carry", StatusFlag::Carry),
            ("overflow", StatusFlag::Overflow),
        ] {
            assert_eq!(parse_expr(source), Expr::Status(status));
        }
    }

    #[test]
    fn distinguishes_registers_from_variables() {
        init_test_registers();

        assert_eq!(parse_expr("a"), Expr::Register("a".into()));
        assert_eq!(
            parse_expr("ordinary_name"),
            Expr::Var("ordinary_name".into())
        );
        assert_eq!(parse_expr("module::func"), Expr::Var("module::func".into()));
        assert!(matches!(parse_expr("func()"), Expr::Call(_)));
    }

    #[test]
    fn parses_operator_precedence() {
        let expr = parse_expr("a + b * c");

        assert_eq!(
            expr,
            Expr::Binary(
                BinaryOp::new(
                    Operator::new(OpType::Add),
                    Expr::Register("a".into()),
                    Expr::Binary(
                        BinaryOp::new(
                            Operator::new(OpType::Mul),
                            Expr::Var("b".into()),
                            Expr::Var("c".into())
                        )
                        .to_box(),
                    ),
                )
                .to_box()
            )
        );
    }

    #[test]
    fn parses_type_cast_of_grouped_expression() {
        assert_eq!(
            parse_expr("(a + b) as u8"),
            Expr::Cast(
                TypeCast::new(
                    Expr::Group(
                        Expr::Binary(
                            BinaryOp::new(
                                Operator::new(OpType::Add),
                                Expr::Register("a".into()),
                                Expr::Var("b".into()),
                            )
                            .to_box()
                        )
                        .to_box()
                    ),
                    def::U8,
                )
                .to_box()
            )
        );
    }

    #[test]
    fn parses_unary_minus_after_binary_multiplication() {
        assert_eq!(
            parse_expr("a * -3"),
            Expr::Binary(
                BinaryOp::new(
                    Operator::new(OpType::Mul),
                    Expr::Register("a".into()),
                    Expr::Unary(
                        UnaryOp::new(
                            Operator::new(OpType::Sub),
                            Expr::Lit(Literal::Int(IntLit(3))),
                        )
                        .to_box()
                    ),
                )
                .to_box()
            )
        );
    }

    #[test]
    fn parses_left_and_non_associative_operators() {
        let Expr::Binary(binary) = parse_expr("a - b - c") else {
            panic!("expected binary expression");
        };
        assert_eq!(binary.op.op_type, OpType::Sub);
        assert!(matches!(
            binary.lhs,
            Expr::Binary(lhs) if lhs.op.op_type == OpType::Sub
        ));

        let source = "a < b < c";
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));
        assert!(parser_expr(true).parse(input).into_result().is_err());
    }

    #[test]
    fn parses_chained_assignments_right_associatively() {
        let a = Expr::Register("a".into());
        let b = Expr::Var("b".into());
        let c = Expr::Var("c".into());

        assert_eq!(
            parse_expr("a = b = c"),
            Expr::Assign(
                AssignOp::new(
                    ASSIGN,
                    a.clone(),
                    Expr::Assign(AssignOp::new(ASSIGN, b.clone(), c.clone()).to_box()),
                )
                .to_box()
            )
        );

        assert_eq!(
            parse_expr("a = b += c"),
            Expr::Assign(
                AssignOp::new(
                    ASSIGN,
                    a,
                    Expr::Assign(AssignOp::new(assign::ADD, b, c).to_box()),
                )
                .to_box()
            )
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
                Expr::Binary(
                    BinaryOp::new(
                        Operator::carry(op_type),
                        Expr::Register("a".into()),
                        Expr::Var("b".into()),
                    )
                    .to_box()
                ),
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
                Expr::Assign(
                    AssignOp::new(
                        Assign::op_carry(op_type),
                        Expr::Register("a".into()),
                        Expr::Var("b".into()),
                    )
                    .to_box()
                ),
                "failed to parse carry assignment in {source}",
            );
        }
    }

    #[test]
    fn parses_prefix_unary_operators_and_calls() {
        let Expr::Unary(unary) = parse_expr("+(d::b(1))") else {
            panic!("expected unary expression");
        };
        assert_eq!(unary.op.op_type, OpType::Add);

        let Expr::Group(group) = unary.expr else {
            panic!("expected grouped expression");
        };
        assert!(matches!(*group, Expr::Call(_)));
    }
}
