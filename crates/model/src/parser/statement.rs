use crate::{
    ToBox,
    lexer::{atom::*, token::Token},
    parser::{
        expression::{Expr, parser_expr},
        function::parser_function_with_statement,
        memory::{
            InBank, ParamType, VarDecl, parser_in_bank_with_function, parser_param_type,
            parser_var_decl,
        },
    },
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, prelude::just,
    recursive::recursive, select, span::SimpleSpan,
};

// MARK: AST

/// Statement AST
/// Instruction that can appear in a block of code
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// Optional label on the statement
    pub label: Option<Label>,

    /// The actual statement / instruction
    pub inner: StmtInner,
}

impl Statement {
    /// Create a new statement
    #[inline]
    pub fn new(inner: StmtInner) -> Self {
        Self { label: None, inner }
    }
}

/// Statement AST
/// Instruction that can appear in a block of code
#[derive(Debug, Clone, PartialEq)]
pub enum StmtInner {
    /// In-bank block for variable declaration
    Bank(Box<InBank>),

    /// Variable declaration
    Var(Box<VarDecl>),

    /// Simple instruction
    Instr(Box<Instr>),

    /// Defer block
    Defer(Defer),

    /// If-else chain
    If(IfElse),

    /// Switch statement
    Switch(Box<Switch>),

    /// Loop block
    Loop(Box<Loop>),

    /// Break or continue instruction
    LoopCtrl(LoopCtrl),

    /// Goto statement
    Goto(Goto),

    /// Return statement
    Return(Return),
}

/// Simple instruction
#[derive(Debug, Clone, PartialEq)]
pub struct Instr {
    /// Expression to evaluate as an instruction
    pub expr: Expr,
}

impl ToBox for Instr {}

/// Defer block
#[derive(Debug, Clone, PartialEq)]
pub struct Defer {
    /// Block of code to execute when the current scope is exited
    pub block: Box<[Statement]>,
}

/// If-else branching chain
#[derive(Debug, Clone, PartialEq)]
pub struct IfElse {
    /// Branches in the if-else chain
    /// Atleast one must be present and only the last one may omit a condition
    pub branches: Box<[Branching]>,
}

impl IfElse {
    #[inline]
    pub fn new(branches: Vec<Branching>) -> Self {
        Self {
            branches: branches.into_boxed_slice(),
        }
    }
}

/// Switch statement
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    /// Variable or register to check
    pub variable: Expr,

    /// Cases to compare with
    /// Atleast one must be present and only the last one may omit a condition
    pub cases: Box<[Branching]>,
}

impl Switch {
    #[inline]
    pub fn new(variable: Expr, cases: Vec<Branching>) -> Self {
        Self {
            variable,
            cases: cases.into_boxed_slice(),
        }
    }
}

impl ToBox for Switch {}

/// Branching element
#[derive(Debug, Clone, PartialEq)]
pub struct Branching {
    /// Condition / case
    /// If none provided, default to true
    pub condition: Option<Box<Expr>>,

    /// Block of code to execute if the condition is true
    pub block: Box<[Statement]>,
}

impl Branching {
    // New branch of an if-else chain
    #[inline]
    pub fn new(condition: Expr, block: Vec<Statement>) -> Self {
        Self {
            condition: Some(Box::new(condition)),
            block: block.into_boxed_slice(),
        }
    }

    // Final else branch
    #[inline]
    pub fn new_else(block: Vec<Statement>) -> Self {
        Self {
            condition: None,
            block: block.into_boxed_slice(),
        }
    }
}

/// Loop
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    /// The type of loop
    pub loop_type: LoopType,

    /// Block of code to execute as long as the condition is true
    pub block: Box<[Statement]>,
}

impl Loop {
    // Create a new loop
    #[inline]
    pub fn new(loop_type: LoopType, block: Vec<Statement>) -> Self {
        Self {
            loop_type,
            block: block.into_boxed_slice(),
        }
    }
}

impl ToBox for Loop {}

/// Define the type of loop
#[derive(Debug, Clone, PartialEq)]
pub enum LoopType {
    /// Common loop without any conditions
    Loop,

    /// While loop
    While(Box<WhileLoopHeader>),

    /// For loop
    /// A variable is initialized and incremented / decremented while iterating over the loop
    For(Box<ForLoopHeader>),
}

/// Header of a while loop
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoopHeader {
    /// Condition to stop iterating
    pub condition: Expr,

    /// Specify if the condition is checked only after a first iteration
    pub do_while: bool,
}

impl WhileLoopHeader {
    // Create a new loop
    #[inline]
    pub fn new(condition: Expr, do_while: bool) -> Self {
        Self {
            condition,
            do_while,
        }
    }
}

impl ToBox for WhileLoopHeader {}

/// Header of a for loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoopHeader {
    /// Define the iterator used
    pub iter: ParamType,

    /// Type of range
    pub range: Range,

    /// Start value of the range
    pub start: Expr,

    /// End value of the range
    pub end: Expr,

    /// Increment or decrement the iterator by the given step
    /// If unspecified, this will default to +1
    pub step: Option<Box<Expr>>,
}

impl ForLoopHeader {
    // Create a new for loop header
    #[inline]
    pub fn new(iter: ParamType, range: Range, start: Expr, end: Expr) -> Self {
        Self {
            iter,
            range,
            start,
            end,
            step: None,
        }
    }

    /// Add the step expression
    #[inline]
    pub fn with_step(&mut self, step: Expr) -> &Self {
        self.step = Some(step.to_box());
        self
    }
}

impl ToBox for ForLoopHeader {}

/// Loop control instruction
#[derive(Debug, Clone, PartialEq)]
pub struct LoopCtrl {
    /// Type of the instruction
    pub ctrl_type: LoopCtrlType,

    /// Label of the loop affected
    pub label: Option<Label>,
}

/// Loop control types
#[derive(Debug, Clone, PartialEq)]
pub enum LoopCtrlType {
    /// Break out of the current loop
    Break,

    /// Continue to the next iteration of the loop
    Continue,
}

/// Goto jump instruction
#[derive(Debug, Clone, PartialEq)]
pub struct Goto {
    /// Label to jump to
    pub label: Label,
}

/// Return from a function call
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Return {
    pub value: Option<Box<Expr>>,
}

impl Return {
    /// Return from the function
    #[inline]
    pub fn new() -> Self {
        Self { value: None }
    }

    /// Return from the function with a value
    #[inline]
    pub fn with_value(value: Expr) -> Self {
        Self {
            value: Some(Box::new(value)),
        }
    }
}

// MARK: Parsers

/// Open brace token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close brace token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

/// Parse a statement.
pub fn parser_statement<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Statement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|statement| {
        let block = parser_block(statement.clone()).boxed();
        let var_decl = parser_var_decl()
            .map(|v| StmtInner::Var(v.to_box()))
            .boxed();
        let bank = parser_in_bank_with_function(parser_function_with_statement(statement.clone()))
            .map(|b| StmtInner::Bank(b.to_box()))
            .boxed();
        let instr = parser_instr().map(|i| StmtInner::Instr(i.to_box())).boxed();
        let defer = parser_defer(block.clone()).map(StmtInner::Defer).boxed();
        let if_else = parser_if_else(block.clone()).map(StmtInner::If).boxed();
        let loop_stmt = parser_loop(block.clone())
            .map(|l| StmtInner::Loop(l.to_box()))
            .boxed();
        let switch = parser_switch(block)
            .map(|s| StmtInner::Switch(s.to_box()))
            .boxed();
        let loop_ctrl = parser_loop_ctrl().map(StmtInner::LoopCtrl).boxed();
        let goto = parser_goto().map(StmtInner::Goto).boxed();
        let return_stmt = parser_return().map(StmtInner::Return).boxed();

        let statement = bank
            .or(var_decl)
            .or(if_else)
            .or(loop_stmt)
            .or(switch)
            .or(loop_ctrl)
            .or(goto)
            .or(return_stmt)
            .or(defer)
            .or(instr);
        select! { Token::Label(label) => label }
            .then_ignore(just(Token::Colon))
            .or_not()
            .then(statement)
            .map(|(label, inner)| Statement { label, inner })
    })
}

/// Parse a break or continue instruction, optionally targeting a loop label.
pub fn parser_loop_ctrl<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, LoopCtrl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let ctrl_b = just(Token::Break).to(LoopCtrlType::Break);
    let ctrl_c = just(Token::Continue).to(LoopCtrlType::Continue);

    ctrl_b
        .or(ctrl_c)
        .then(select! { Token::Label(label) => label }.or_not())
        .then_ignore(just(Token::Semicolon))
        .map(|(ctrl_type, label)| LoopCtrl { ctrl_type, label })
}

/// Parse a goto statement, which must target a label.
pub fn parser_goto<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Goto, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::Goto)
        .ignore_then(select! { Token::Label(label) => label })
        .then_ignore(just(Token::Semicolon))
        .map(|label| Goto { label })
}

/// Parse a return statement with an optional value expression.
pub fn parser_return<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Return, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::Return)
        .ignore_then(parser_expr(true).or_not())
        .then_ignore(just(Token::Semicolon))
        .map(|value| match value {
            Some(value) => Return::with_value(value),
            None => Return::new(),
        })
}

/// Parse a simple expression instruction terminated by a semicolon.
pub fn parser_instr<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Instr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parser_expr(true)
        .then_ignore(just(Token::Semicolon))
        .map(|expr| Instr { expr })
}

/// Parse a defer block, which executes its statements when the current scope exits.
pub fn parser_defer<'tok, 'src: 'tok, I, B>(
    block: B,
) -> impl Parser<'tok, I, Defer, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    B: Parser<'tok, I, Vec<Statement>, extra::Err<Rich<'tok, Token>>> + 'tok,
{
    just(Token::Defer).ignore_then(block).map(|block| Defer {
        block: block.into_boxed_slice(),
    })
}

/// Parse an if-else chain, including nested statements in each block.
pub fn parser_if_else<'tok, 'src: 'tok, I, B>(
    block: B,
) -> impl Parser<'tok, I, IfElse, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    B: Parser<'tok, I, Vec<Statement>, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    // Condition is a non constant expression
    let condition = parser_expr(true).boxed();

    // if <condition> { ... }
    let if_branch = just(Token::If)
        .ignore_then(condition.clone())
        .then(block.clone())
        .map(|(condition, block)| Branching::new(condition, block));

    // else if <condition> { ... }
    let else_if_branch = just(Token::Else)
        .ignore_then(just(Token::If))
        .ignore_then(condition)
        .then(block.clone())
        .map(|(condition, block)| Branching::new(condition, block));

    // else { ... }
    let else_branch = just(Token::Else)
        .ignore_then(block)
        .map(Branching::new_else);

    // chain the three types of elements
    if_branch
        .then(else_if_branch.repeated().collect::<Vec<_>>())
        .then(else_branch.or_not())
        .map(|((first, mut else_if), else_branch)| {
            let mut branches =
                Vec::with_capacity(1 + else_if.len() + usize::from(else_branch.is_some()));
            branches.push(first);
            branches.append(&mut else_if);
            if let Some(else_branch) = else_branch {
                branches.push(else_branch);
            }
            IfElse::new(branches)
        })
}

/// Parse a switch statement with braced case bodies.
pub fn parser_switch<'tok, 'src: 'tok, I, B>(
    block: B,
) -> impl Parser<'tok, I, Switch, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    B: Parser<'tok, I, Vec<Statement>, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    let case = parser_expr(false)
        .then_ignore(just(Token::FatArrow))
        .then(block.clone())
        .map(|(value, block)| Branching::new(value, block));
    let default_case = just(Token::Anonymous)
        .ignore_then(just(Token::FatArrow))
        .ignore_then(block);

    just(Token::Switch)
        .ignore_then(parser_expr(true))
        .then(
            case.repeated()
                .at_least(1)
                .collect::<Vec<_>>()
                .then(default_case.or_not())
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE)),
        )
        .map(|(variable, (mut cases, default_case))| {
            if let Some(block) = default_case {
                cases.push(Branching::new_else(block));
            }
            Switch::new(variable, cases)
        })
}

/// Parse infinite, while, do-while, and range-based for loops.
pub fn parser_loop<'tok, 'src: 'tok, I, B>(
    block: B,
) -> impl Parser<'tok, I, Loop, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    B: Parser<'tok, I, Vec<Statement>, extra::Err<Rich<'tok, Token>>> + Clone + 'tok,
{
    // simple infinite loop
    let infinite = just(Token::Loop)
        .ignore_then(block.clone())
        .map(|block| Loop::new(LoopType::Loop, block));

    // while loop
    let while_loop = just(Token::While)
        .ignore_then(parser_expr(true))
        .then(block.clone())
        .map(|(condition, block)| {
            let header = WhileLoopHeader::new(condition, false);
            Loop::new(LoopType::While(header.to_box()), block)
        });

    // do-while loop
    let do_while_loop = just(Token::Do)
        .ignore_then(block.clone())
        .then_ignore(just(Token::While))
        .then(parser_expr(true))
        .then_ignore(just(Token::Semicolon))
        .map(|(block, condition)| {
            let header = WhileLoopHeader::new(condition, true);
            Loop::new(LoopType::While(header.to_box()), block)
        });

    // for loop
    let for_iterator = parser_param_type();
    let for_range = parser_expr(false)
        .then(select! { Token::Range(range) => range })
        .then(parser_expr(false))
        .map(|((start, range), end)| (start, range, end));
    let for_step = just(Token::Step).ignore_then(parser_expr(false)).or_not();
    let for_loop = just(Token::For)
        .ignore_then(for_iterator)
        .then_ignore(just(Token::Of))
        .then(for_range)
        .then(for_step)
        .then(block)
        .map(|(((iter, (start, range, end)), step), block)| {
            let mut header = ForLoopHeader::new(iter, range, start, end);
            if let Some(step) = step {
                header.with_step(step);
            }
            Loop::new(LoopType::For(header.to_box()), block)
        });

    infinite.or(while_loop).or(do_while_loop).or(for_loop)
}

/// Parse a block of code delimited by braces
pub fn parser_block<'tok, 'src: 'tok, I, S>(
    statement: S,
) -> impl Parser<'tok, I, Vec<Statement>, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
    S: Parser<'tok, I, Statement, extra::Err<Rich<'tok, Token>>>,
{
    // Accept a list of statements enclosed in braces `{ ... }`
    statement
        .repeated()
        .collect::<Vec<_>>()
        .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE))
}

// MARK: Notes

// TODO:
// switch
// return       Return from a function
// break        Exit a loop or switch
// continue     Skip to the next loop iteration
// fallthrough  Fall to next switch case
// goto         Jump to a label
// 'label       Define jump destinations
// defer
// asm

/*
    consider macro block for inserting raw assembly or calling a scripting language ?

    #[macro]
    {%
        ...
    %}

    #[asm]
    {{
        ...
    }}

    #[macro]
    (*
        ...
    *)

    #[asm]
    #{
        ...
    }#
*/

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ToBox,
        lexer::{register::*, token::Token},
        parser::{
            composite_type::TypeUsed,
            expression::{AssignOp, BinaryOp},
            memory::{MemStorage, ParamMem},
        },
    };
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_var_decl(source: &str) -> VarDecl {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_var_decl()
            .parse(input)
            .into_result()
            .expect("test source should parse")
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

    fn parse_if_else(source: &str) -> IfElse {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_if_else(parser_block(parser_statement()).boxed())
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    fn parse_block(source: &str) -> Vec<Statement> {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_block(parser_statement())
            .parse(input)
            .into_result()
            .expect("test source should parse")
    }

    fn parse_loop(source: &str) -> Loop {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        let Statement {
            label: _,
            inner: StmtInner::Loop(loop_stmt),
        } = parser_statement()
            .parse(input)
            .into_result()
            .expect("test source should parse")
        else {
            panic!("expected loop statement");
        };
        *loop_stmt
    }

    #[test]
    fn parses_variable_declarations_with_storage_and_initializers() {
        let declarations = [
            "var my_var: u8 in zeropage;",
            "var my_var: u8 at ADDRESS = a += 25 + 2;",
            "static MY_STATIC: u8 at 0x0010;",
            "static MY_STATIC2: u8 at 0x0010 = 34;",
            "static MY_STATIC3: u8 = 34;",
            "var my_var2: u8 in zeropage = a = 50 + 2;",
            "var my_var3: u8 = 13 + 2;",
            "const MY_CONST: u8 = 56 + 34;",
        ];

        for declaration in declarations {
            parse_var_decl(declaration);
        }
    }

    #[test]
    fn keeps_address_and_initializer_separate() {
        let declaration = parse_var_decl("var my_var: u8 at ADDRESS = a += 25 + 2;");

        assert_eq!(
            declaration.storage,
            MemStorage::At(Expr::Var("ADDRESS".into()).to_box())
        );
        assert!(matches!(
            declaration.assign_expr,
            Some(expr) if matches!(*expr, Expr::Assign(_))
        ));
    }

    #[test]
    fn parses_hexadecimal_fixed_address() {
        let declaration = parse_var_decl("static MY_STATIC: u8 at 0x0010;");

        assert_eq!(
            declaration.storage,
            MemStorage::At(Expr::Lit(Literal::Int(IntLit(16))).to_box())
        );
    }

    #[test]
    fn requires_a_semicolon() {
        init_test_registers();
        let source = "var my_var: u8";
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        assert!(parser_var_decl().parse(input).into_result().is_err());
    }

    #[test]
    fn parses_if_else_chain() {
        let if_else = parse_if_else(
            "
if      toto == true { var first:  u8; } 
else if tata > 0     { var second: u8; } 
else                 { var third:  u8; }
",
        );

        assert_eq!(if_else.branches.len(), 3);
        assert!(if_else.branches[0].condition.is_some());
        assert!(if_else.branches[1].condition.is_some());
        assert!(if_else.branches[2].condition.is_none());
        assert!(matches!(
            if_else.branches[0].block[0].inner,
            StmtInner::Var(_)
        ));
        assert!(matches!(
            if_else.branches[1].block[0].inner,
            StmtInner::Var(_)
        ));
        assert!(matches!(
            if_else.branches[2].block[0].inner,
            StmtInner::Var(_)
        ));
    }

    #[test]
    fn parses_if_without_else_and_nested_if() {
        let if_else = parse_if_else("if outer { if inner { var nested: u8; } }");

        assert_eq!(if_else.branches.len(), 1);
        let [
            Statement {
                label: _,
                inner: StmtInner::If(nested),
            },
        ] = &if_else.branches[0].block[..]
        else {
            panic!("expected nested if statement");
        };
        assert_eq!(nested.branches.len(), 1);
    }

    #[test]
    fn parses_reusable_block_with_nested_statements() {
        let block = parse_block("{ var value: u8; if ready { var nested: u8; } }");

        assert_eq!(block.len(), 2);
        assert!(matches!(block[0].inner, StmtInner::Var(_)));
        assert!(matches!(block[1].inner, StmtInner::If(_)));
        assert!(parse_block("{}").is_empty());
    }

    #[test]
    fn parses_labels_on_statements() {
        let variable = parse_statement("'variable_label: var value: u8;");
        assert_eq!(variable.label, Some(Label("variable_label".into())));
        assert!(matches!(variable.inner, StmtInner::Var(_)));

        let loop_stmt = parse_statement("'loop_label: loop { }");
        assert_eq!(loop_stmt.label, Some(Label("loop_label".into())));
        assert!(matches!(loop_stmt.inner, StmtInner::Loop(_)));

        let conditional = parse_statement("'if_label: if ready { }");
        assert_eq!(conditional.label, Some(Label("if_label".into())));
        assert!(matches!(conditional.inner, StmtInner::If(_)));

        assert_eq!(parse_statement("loop { }").label, None);
    }

    #[test]
    fn parses_loop_control_with_optional_target_label() {
        for (source, ctrl_type, label) in [
            ("break;", LoopCtrlType::Break, None),
            (
                "break 'label1;",
                LoopCtrlType::Break,
                Some(Label("label1".into())),
            ),
            ("continue;", LoopCtrlType::Continue, None),
            (
                "continue 'label2;",
                LoopCtrlType::Continue,
                Some(Label("label2".into())),
            ),
        ] {
            let statement = parse_statement(source);
            assert_eq!(statement.label, None);
            assert_eq!(
                statement.inner,
                StmtInner::LoopCtrl(LoopCtrl { ctrl_type, label })
            );
        }
    }

    #[test]
    fn parses_goto_with_required_target_label() {
        let statement = parse_statement("goto 'target;");
        assert_eq!(
            statement.inner,
            StmtInner::Goto(Goto {
                label: Label("target".into())
            })
        );

        let tokens = Token::lexer("goto;")
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0.."goto;".len()).into(), |(token, span)| (token, span));
        assert!(parser_statement().parse(input).has_errors());
    }

    #[test]
    fn parses_return_with_or_without_value() {
        assert_eq!(
            parse_statement("return;").inner,
            StmtInner::Return(Return::new())
        );

        assert_eq!(
            parse_statement("return result + 1;").inner,
            StmtInner::Return(Return::with_value(Expr::Binary(
                BinaryOp::new(
                    operator::ADD,
                    Expr::Var("result".into()),
                    Expr::Lit(Literal::Int(IntLit(1))),
                )
                .to_box()
            )))
        );
    }

    #[test]
    fn parses_simple_expression_instructions() {
        let statement = parse_statement("a = 10;");
        let StmtInner::Instr(instr) = statement.inner else {
            panic!("expected instruction statement");
        };
        assert_eq!(
            instr.expr,
            Expr::Assign(
                AssignOp::new(
                    Assign::new(),
                    Expr::Register("a".into()),
                    Expr::Lit(Literal::Int(IntLit(10))),
                )
                .to_box()
            )
        );
    }

    #[test]
    fn parses_defer_blocks_with_nested_statements() {
        let statement = parse_statement("defer { a = 10; b = 20; }");
        let StmtInner::Defer(defer) = statement.inner else {
            panic!("expected defer statement");
        };

        assert_eq!(defer.block.len(), 2);
        assert!(matches!(defer.block[0].inner, StmtInner::Instr(_)));
        assert!(matches!(defer.block[1].inner, StmtInner::Instr(_)));

        let tokens = Token::lexer("defer;")
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input = Stream::from_iter(tokens)
            .map((0.."defer;".len()).into(), |(token, span)| (token, span));
        assert!(parser_statement().parse(input).has_errors());
    }

    #[test]
    fn parses_switch_cases_and_default_case() {
        let statement = parse_statement(
            "switch value { 0 => { var zero: u8; } 1 => { } _ => { var other: u8; } }",
        );
        let StmtInner::Switch(switch) = statement.inner else {
            panic!("expected switch statement");
        };

        assert_eq!(
            switch.variable,
            Expr::Var("value".into())
        );
        assert_eq!(switch.cases.len(), 3);
        assert_eq!(
            switch.cases[0].condition,
            Some(Expr::Lit(Literal::Int(IntLit(0))).to_box())
        );
        assert_eq!(
            switch.cases[1].condition,
            Some(Expr::Lit(Literal::Int(IntLit(1))).to_box())
        );
        assert!(switch.cases[2].condition.is_none());
        assert!(matches!(switch.cases[0].block[0].inner, StmtInner::Var(_)));
        assert!(switch.cases[1].block.is_empty());
        assert!(matches!(switch.cases[2].block[0].inner, StmtInner::Var(_)));
    }

    #[test]
    fn parses_switch_without_default_case() {
        let statement = parse_statement("switch value { 0 => { } 1 => { } }");
        let StmtInner::Switch(switch) = statement.inner else {
            panic!("expected switch statement");
        };

        assert_eq!(switch.cases.len(), 2);
        assert!(switch.cases.iter().all(|case| case.condition.is_some()));
    }

    #[test]
    fn parses_infinite_while_and_do_while_loops() {
        let infinite = parse_loop("loop { var value: u8; }");
        assert!(matches!(infinite.loop_type, LoopType::Loop));
        assert_eq!(infinite.block.len(), 1);

        let while_loop = parse_loop("while ready { }");
        assert!(matches!(while_loop.loop_type, LoopType::While(header) if !header.do_while));

        let do_while = parse_loop("do { } while ready;");
        assert!(matches!(do_while.loop_type, LoopType::While(header) if header.do_while));
    }

    #[test]
    fn parses_exclusive_for_range_with_default_step() {
        let loop_stmt = parse_loop("for i: u8 in zeropage of 0..10 { }");
        let LoopType::For(header) = loop_stmt.loop_type else {
            panic!("expected for loop");
        };

        let ParamType::Memory(ParamMem {
            ident,
            type_used,
            storage,
        }) = header.iter
        else {
            panic!("expected variable iterator");
        };
        assert_eq!(ident, "i".into());
        assert_eq!(type_used, TypeUsed::Prim(primitive::U8));
        assert_eq!(storage, MemStorage::In("zeropage".into()));
        assert_eq!(header.range, Range::excl());
        assert_eq!(header.start, Expr::Lit(Literal::Int(IntLit(0))));
        assert_eq!(header.end, Expr::Lit(Literal::Int(IntLit(10))));
        assert_eq!(header.step, None);
    }

    #[test]
    fn parses_inclusive_for_range_with_fixed_address_and_step() {
        let loop_stmt = parse_loop("for i: u8 at 0x0100 of 10..=100 step 2 { }");
        let LoopType::For(header) = loop_stmt.loop_type else {
            panic!("expected for loop");
        };

        let ParamType::Memory(ParamMem {
            ident,
            type_used,
            storage,
        }) = header.iter
        else {
            panic!("expected variable iterator");
        };
        assert_eq!(
            storage,
            MemStorage::At(Expr::Lit(Literal::Int(IntLit(256))).to_box())
        );
        assert_eq!(ident, "i".into());
        assert_eq!(type_used, TypeUsed::Prim(primitive::U8));
        assert_eq!(header.range, Range::incl());
        assert_eq!(header.start, Expr::Lit(Literal::Int(IntLit(10))));
        assert_eq!(header.end, Expr::Lit(Literal::Int(IntLit(100))));
        assert_eq!(
            header.step,
            Some(Expr::Lit(Literal::Int(IntLit(2))).to_box())
        );
    }

    #[test]
    fn parses_for_loop_using_register_iterator() {
        init_test_registers();
        let loop_stmt = parse_loop("for x of 0..10 { }");
        let LoopType::For(header) = loop_stmt.loop_type else {
            panic!("expected for loop");
        };

        assert_eq!(header.iter, ParamType::Register("x".into()));
    }
}
