//!
//! Lexer for the extracting tokens
//!

use logos::Logos;

use crate::ast::Precedence;

/// Tokens in the language
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    /// Multi-line comment
    #[regex(r"/\*([^*]|[\r\n]|(\*+([^*/]|[\r\n])))*\*+/", |lex| lex.slice().to_string())]
    MultiComment(String),

    /// Single-line comment
    #[regex(r"//[^\n\r]*[\n\r]", |lex| lex.slice().to_string())]
    SingleComment(String),

    /// Comma separator
    #[token(",")]
    Comma,

    /// Member access
    #[token(".")]
    MemberAccess,

    /// Define type, label or access byte in word
    #[token(":")]
    Colon,

    /// Separator between path elements
    #[token("::")]
    PathSep,

    /// Expression separator
    #[token(";")]
    Semicolon,

    /// Open (
    #[token("(")]
    ParenStart,

    /// Close )
    #[token(")")]
    ParenEnd,

    /// Open [
    #[token("[")]
    BracketStart,

    /// Close ]
    #[token("]")]
    BracketEnd,

    /// Open {
    #[token("{")]
    BraceStart,

    /// Close }
    #[token("}")]
    BraceEnd,

    /// Assign
    #[token("=")]
    Assign,

    /// Equals
    #[token("==")]
    Equal,

    /// Different
    #[token("!=")]
    Diff,

    /// Lesser than
    #[token("<")]
    Lesser,

    /// Greater than
    #[token(">")]
    Greater,

    /// Lesser or equal
    #[token("<=")]
    LesserEq,

    /// Greater or equal
    #[token(">=")]
    GreaterEq,

    /// Boolean NOT
    #[token("!")]
    BoolNot,

    /// Boolean AND
    #[token("&&")]
    BoolAnd,

    /// Boolean OR
    #[token("||")]
    BoolOr,

    /// Bitwise inversion
    #[token("~")]
    BitInv,

    /// Bitwise AND or reference
    #[token("&")]
    Ampersand,

    /// Bitwise AND
    #[token("&=")]
    BitAndAssign,

    /// Bitwise OR
    #[token("|")]
    BitOr,

    /// Bitwise OR
    #[token("|=")]
    BitOrAssign,

    /// Bitwise XOR
    #[token("^")]
    BitXor,

    /// Bitwise XOR
    #[token("^=")]
    BitXorAssign,

    /// Increment
    #[token("++")]
    Incr,

    /// Decrement
    #[token("--")]
    Decr,

    /// Add
    #[token("+")]
    Plus,

    /// Add
    #[token("+=")]
    PlusAssign,

    /// Add with carry
    #[token("+'")]
    PlusCarry,

    /// Add with carry
    #[token("+'=")]
    PlusCarryAssign,

    /// Subtract or negation
    #[token("-")]
    Minus,

    /// Subtract or negation
    #[token("-=")]
    MinusAssign,

    /// Subtract with carry
    #[token("-'")]
    MinusCarry,

    /// Subtract with carry
    #[token("-'=")]
    MinusCarryAssign,

    /// Multiply or pointer
    #[token("*")]
    Asterix,

    /// Multiply
    #[token("*=")]
    MulAssign,

    /// Divide
    #[token("/")]
    Div,

    /// Divide
    #[token("/=")]
    DivAssign,

    /// Remainder
    #[token("%")]
    Rem,

    /// Remainder
    #[token("%=")]
    RemAssign,

    /// Left Bitshift (logical or arithmetic)
    #[token("<<")]
    ShiftLeft,

    /// Left Bitshift (logical or arithmetic)
    #[token("<<=")]
    ShiftLeftAssign,

    /// Right Bitshift (arithmetic)
    #[token(">>")]
    ShiftRightArith,

    /// Right Bitshift (arithmetic)
    #[token(">>=")]
    ShiftRightArithAssign,

    /// Right Bitshift (logical)
    #[token(">>>")]
    ShiftRightLogic,

    /// Right Bitshift (logical)
    #[token(">>>=")]
    ShiftRightLogicAssign,

    /// Left Bitroll
    #[token("<+")]
    RotateLeft,

    /// Left Bitroll
    #[token("<+=")]
    RotateLeftAssign,

    /// Left Bitroll with carry
    #[token("<+'")]
    RotateLeftCarry,

    /// Left Bitroll with carry
    #[token("<+'=")]
    RotateLeftCarryAssign,

    /// Right Bitroll
    #[token("+>")]
    RotateRight,

    /// Right Bitroll
    #[token("+>=")]
    RotateRightAssign,

    /// Right Bitroll with carry
    #[token("+>'")]
    RotateRightCarry,

    /// Right Bitroll with carry
    #[token("+>'=")]
    RotateRightCarryAssign,

    /// Address Operator
    #[token("@")]
    Address,

    /// Range Operator
    #[token("..")]
    RangeExc,

    /// Range Operator
    #[token("..=")]
    RangeInc,

    /// Boolean literal: False
    #[token("false")]
    False,

    /// Boolean literal: True
    #[token("true")]
    True,

    /// Declare a variable
    #[token("var")]
    Var,

    /// Declare a macro variable
    #[token("let")]
    Let,

    /// Declare a constant
    #[token("const")]
    Const,

    /// Declare a function
    #[token("fn")]
    Func,

    /// Declare a module
    #[token("mod")]
    Mod,

    /// Conditional branching
    #[token("if")]
    If,

    /// Conditional branching
    #[token("else")]
    Else,

    /// Infinite loop
    #[token("loop")]
    Loop,

    /// For-loop
    #[token("for")]
    For,

    /// In
    #[token("in")]
    In,

    /// For-loop step
    #[token("step")]
    Step,

    /// Loop with exit condition
    #[token("while")]
    While,

    /// Loop with exit condition
    #[token("do")]
    Do,

    /// Break out of loop
    #[token("break")]
    Break,

    /// Continue loop
    #[token("continue")]
    Continue,

    /// Return a value
    #[token("return")]
    Return,

    /// Goto label
    #[token("goto")]
    Goto,

    /// Integer literal
    #[regex(r"-?(\d[0-9_]*|0b[01][01_]*|0o[0-7][0-7_]*|0x[0-9A-Fa-f][0-9A-Fa-f_]*)", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLit(i64),

    /// String literal
    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| lex.slice().to_owned())]
    StrLit(String),

    /// Label
    #[regex(r"'[A-Za-z_]\w*", |lex| lex.slice()[1..].to_string())]
    Label(String),

    /// Identifier
    #[regex(r"[A-Za-z_]\w*", |lex| lex.slice().to_string())]
    Ident(String),
}
