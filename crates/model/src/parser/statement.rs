use crate::{
    lexer::{atom::*, token::Token},
    parser::expression::{Expr, parser_expr},
};
use chumsky::{
    Parser, error::Rich, extra, input::ValueInput, prelude::just, select, span::SimpleSpan,
};

// MARK: AST

/// Statement AST
/// Instruction that can appear in a block of code
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Variable declaration
    Var(VarDecl),

    /// If-else chain
    If(IfElse),
}

/// Parameter declaration
/// Handle variable declaration for function parameters and for-loop iterator
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

/// Variable declaration.
/// This structure can be used in various context:
/// - Outside of functions for constants and static variables
/// - Inside of functions' bodies
/// - As function parameters (`decl_type` is then omitted)
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    /// Type of variable declaration
    pub decl_type: VarDeclType,

    /// Identifier of the variable
    pub ident: Ident,

    // TODO: for now we are onyl supporting primitives
    /// Type of the variable
    pub var_type: Primitive,

    /// Storage location for the variable
    pub storage: VarStorage,

    /// Assign a value to the variable when declaring it.
    /// Note: Only regular assignments (`=`) are allowed here.
    pub assign_expr: Option<Box<Expr>>,
}

/// Specify where to store the variable
#[derive(Debug, Default, Clone, PartialEq)]
pub enum VarStorage {
    /// If unspecified, the variable will fallback to the parent's scope bank.
    #[default]
    Undefined,

    /// Find an available memory location in the given bank.
    In(Ident),

    /// Fixed address for the variable.
    /// Provided address must be solvable at compile-time.
    At(Box<Expr>),
}

/// If-else branching chain
#[derive(Debug, Clone, PartialEq)]
pub struct IfElse {
    /// Branches in the if-else chain
    /// Atleast one must be present and only the last one may omit a condition
    pub branches: Box<[Branching]>,
}

/// Switch statement
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    /// Variable or register to check
    pub variable: Box<Expr>,

    /// Cases to compare with
    /// Atleast one must be present and only the last one may omit a condition
    pub cases: Box<[Branching]>,
}

/// Branching element
#[derive(Debug, Clone, PartialEq)]
pub struct Branching {
    /// Condition / case
    /// If none provided, default to true
    pub condition: Option<Box<Expr>>,

    /// Block of code to execute if the condition is true
    pub block: Box<[Statement]>,
}

/// Loop
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    /// The type of loop
    pub loop_type: LoopType,

    /// Block of code to execute as long as the condition is true
    pub block: Box<[Statement]>,
}

/// Define the type of loop
#[derive(Debug, Clone, PartialEq)]
pub enum LoopType {
    /// Common loop without any conditions
    Loop,

    /// While loop
    While(WhileLoopHeader),

    /// For loop
    /// A variable is initialized and incremented / decremented while iterating over the loop
    For(ForLoopHeader),
}

/// Header of a while loop
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoopHeader {
    /// Condition to stop iterating
    pub condition: Box<Expr>,

    /// Specify if the condition is checked only after a first iteration
    pub do_while: bool,
}

/// Header of a for loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoopHeader {
    /// Initialize the iterator
    pub init: ParamDecl,

    /// Condition to stop iterating
    pub condition: Box<Expr>,

    /// Increment or decrement the iterator by the given step
    pub step: i64,
}

// MARK: Parsers

/// Parse a variable, static, or constant declaration.
pub fn parser_var_decl<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, VarDecl, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let storage = just(Token::In)
        .ignore_then(select! {
            Token::Ident(ident) => VarStorage::In(ident),
        })
        .or(just(Token::At)
            .ignore_then(parser_expr(true))
            .map(|expr| VarStorage::At(Box::new(expr))))
        .or_not()
        .map(Option::unwrap_or_default);

    select! {
        Token::VarDecl(decl_type) => decl_type,
    }
    .then(select! {
        Token::Ident(ident) => ident,
    })
    .then_ignore(just(Token::Colon))
    .then(select! {
        Token::Primitive(var_type) => var_type,
    })
    .then(storage)
    .then(
        just(Token::Assign(Assign::new()))
            .ignore_then(parser_expr(true))
            .or_not(),
    )
    .then_ignore(just(Token::Semicolon))
    .map(
        |((((decl_type, ident), var_type), storage), assign_expr)| VarDecl {
            decl_type,
            ident,
            var_type,
            storage,
            assign_expr: assign_expr.map(Box::new),
        },
    )
}

// MARK: Notes

// TODO:
// if-else chains
// loops (loop, for, while, do-while)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::token::Token, parser::expression::AssignOp};
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_var_decl(source: &str) -> VarDecl {
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
            VarStorage::At(Box::new(Expr::Var(crate::parser::path::Path::new(&[
                "ADDRESS"
            ]))))
        );
        assert!(matches!(
            declaration.assign_expr,
            Some(expr) if matches!(*expr, Expr::Assign(AssignOp { .. }))
        ));
    }

    #[test]
    fn parses_hexadecimal_fixed_address() {
        let declaration = parse_var_decl("static MY_STATIC: u8 at 0x0010;");

        assert_eq!(
            declaration.storage,
            VarStorage::At(Box::new(Expr::Lit(Literal::Int(IntLit(16)))))
        );
    }

    #[test]
    fn requires_a_semicolon() {
        let source = "var my_var: u8";
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        assert!(parser_var_decl().parse(input).into_result().is_err());
    }
}
