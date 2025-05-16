//!
//! Lexer for the extracting tokens
//!

use logos::Logos;

/// Tokens in the language
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    /// Script Escape
    #[regex(r"#\{.*\}#", |lex| lex.slice().to_string())]
    EmbeddedScript(String),

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

    /// Define type
    #[token(":")]
    TypeDef,

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

    /// Bitwise OR
    #[token("|")]
    BitOr,

    /// Bitwise XOR
    #[token("^")]
    BitXor,

    /// Increment
    #[token("++")]
    Incr,

    /// Decrement
    #[token("--")]
    Decr,

    /// Add
    #[token("+")]
    Plus,

    /// Add with carry
    #[token("+'")]
    PlusCarry,

    /// Subtract or negation
    #[token("-")]
    Minus,

    /// Subtract with carry
    #[token("-'")]
    MinusCarry,

    /// Multiply or pointer
    #[token("*")]
    Asterix,

    /// Arithmetic Operator
    #[token("/")]
    Divide,

    /// Arithmetic Operator
    #[token("%")]
    Modulo,

    /// Left Bitshift (logical or arithmetic)
    #[token("<<")]
    ShiftLeft,

    /// Right Bitshift (arithmetic)
    #[token(">>")]
    ShiftRightArithmetic,

    /// Right Bitshift (logical)
    #[token(">>>")]
    ShiftRightLogical,

    /// Left Bitroll
    #[token("<+")]
    RotateLeft,

    /// Left Bitroll with carry
    #[token("<+'")]
    RotateLeftCarry,

    /// Right Bitroll
    #[token("+>")]
    RotateRight,

    /// Right Bitroll with carry
    #[token("+>'")]
    RotateRightCarry,

    /// Address Operator
    #[token("@")]
    Address,

    /// Range Operator
    #[token("..")]
    RangeExclusive,

    /// Range Operator
    #[token("..=")]
    RangeInclusive,

    /// Access the low byte of a word
    #[token("lo:")]
    ByteLow,

    /// Access the high byte of a word
    #[token("hi:")]
    ByteHigh,

    /// Access the bank byte of a word
    #[token("bk:")]
    ByteBank,

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

    /// Label
    #[regex(r"'\w+", |lex| lex.slice().to_string())]
    Label(String),

    /// Identifier
    #[regex(r"\w+", |lex| lex.slice().to_string())]
    Ident(String),
}
