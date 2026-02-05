//!
//! Lexer for the extracting tokens
//!

use logos::{Lexer, Logos};

/// Tokens in the language
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // white spaces
#[logos(skip r"//[^\n\r]*[\n\r]")] // single-line comments
#[logos(skip r"/\*([^*]|[\r\n]|(\*+([^*/]|[\r\n])))*\*+/")] // multi-line comments
enum Token {
    /// Documentation comment
    #[regex(r"///[^\n\r]*[\n\r]", parse_single_comment)]
    #[regex(r"/\*\*([^*]|[\r\n]|(\*+([^*/]|[\r\n])))*\*+/", parse_multi_comment)]
    DocComment(String),

    /// Comma
    #[token(",")]
    Comma,

    /// Period
    #[token(".")]
    Period,

    /// Define type, label or access byte in word
    #[token(":")]
    Colon,

    /// Separator between path elements
    #[token("::")]
    PathSep,

    /// Semi-Colon
    #[token(";")]
    Semicolon,

    /// Open (
    #[token("(")]
    ParenOpen,

    /// Close )
    #[token(")")]
    ParenClose,

    /// Open [
    #[token("[")]
    BracketOpen,

    /// Close ]
    #[token("]")]
    BracketClose,

    /// Open {
    #[token("{")]
    BraceOpen,

    /// Close }
    #[token("}")]
    BraceClose,

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

    /// Cast type
    #[token("as")]
    As,

    /// Integer literal
    #[regex(r"-?\d[0-9_]*", parse_integer)]
    #[regex(r"-?0b[01][01_]*", parse_integer)]
    #[regex(r"-?0o[0-7][0-7_]*", parse_integer)]
    #[regex(r"-?0x[0-9A-Fa-f][0-9A-Fa-f_]*", parse_integer)]
    IntLit(i64),

    /// Floating point literal
    #[regex(r"[+-]?\d+\.\d*([eE][+-]?\d+)?", parse_float)]
    #[regex(r"[+-]?\.\d+([eE][+-]?\d+)?", parse_float)]
    FloatLit(f64),

    /// Character literal
    #[regex(r"'[^']'", parse_character)]
    #[regex(r"'\\.'", parse_character_esc)]
    #[regex(r"'\\x[0-9A-Fa-f]{2}'", parse_character_hex)]
    CharLit(char),

    /// String literal
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    StrLit(String),

    /// Label
    #[regex(r"'[A-Za-z_]\w*", parse_label)]
    Label(String),

    /// Identifier
    #[regex(r"[A-Za-z_]\w*", parse_identifier)]
    Ident(String),
}

/// Parse single-line doc-comment `/// ...`
#[inline]
fn parse_single_comment<'s>(lex: &Lexer<'s, Token>) -> String {
    // skip `///`
    lex.slice()[3..].to_string()
}

/// Parse multi-line doc-comment `/** ... */`
#[inline]
fn parse_multi_comment<'s>(lex: &Lexer<'s, Token>) -> String {
    // remove the `/**` and `*/` on both ends
    let seq = lex.slice();
    seq[3..seq.len() - 2].to_string()
}

/// Parse integer literal `0`
#[inline]
fn parse_integer<'s>(lex: &Lexer<'s, Token>) -> i64 {
    lex.slice().parse::<i64>().unwrap()
}

/// Parse floating point number literal `0.0`
#[inline]
fn parse_float<'s>(lex: &Lexer<'s, Token>) -> f64 {
    lex.slice().parse::<f64>().unwrap()
}

/// Parse character literal `'a'`
#[inline]
fn parse_character<'s>(lex: &Lexer<'s, Token>) -> char {
    // skip `'`
    lex.slice().chars().nth(1).unwrap()
}

/// Parse character literal `'\0'`
#[inline]
fn parse_character_esc<'s>(lex: &Lexer<'s, Token>) -> char {
    // skip `'\`
    let c = lex.slice().chars().nth(2).unwrap();
    match c {
        '0' => '\0',   // null
        'n' => '\n',   // new line
        'r' => '\r',   // carriage return
        't' => '\t',   // horizontal tab
        '\'' => '\'',  // single quote
        '\\' => '\\',  // backslash
        'a' => '\x07', // audible bell
        'b' => '\x08', // backspace
        'v' => '\x0B', // vertical tab
        'f' => '\x0C', // form feed
        _ => c,
    }
}

/// Parse character literal `'\x00'`
#[inline]
fn parse_character_hex<'s>(lex: &Lexer<'s, Token>) -> char {
    // skip `'\x`
    u8::from_str_radix(&lex.slice()[3..5], 16).unwrap() as char
}

/// Parse string literal `"..."`
#[inline]
fn parse_string<'s>(lex: &Lexer<'s, Token>) -> String {
    // remove the double quotes on both ends
    let seq = lex.slice();
    seq[1..seq.len() - 1].to_owned()
}

/// Parse label literal `'label`
#[inline]
fn parse_label<'s>(lex: &Lexer<'s, Token>) -> String {
    lex.slice()[1..].to_string()
}

/// Parse identifier `ident`
#[inline]
fn parse_identifier<'s>(lex: &Lexer<'s, Token>) -> String {
    lex.slice().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
}
