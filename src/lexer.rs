//!
//! Lexer for the Asmanite language
//!

use logos::{Lexer, Logos};
use std::fmt::{self, write};

/*
use std::ops::Range;

/// Spanned token
#[derive(Debug, Clone, PartialEq)]
pub struct TokenSpan {
    /// Token extracted from Logos
    pub token: Token,

    /// Span of the token
    pub span: Range<usize>,
}

/// Pass the input string into the lexer
pub fn lex(input: &str) -> Vec<TokenSpan> {
    let mut lexer = Token::lexer(input);

    // Prepare storage for the tokens
    let mut tokens = Vec::with_capacity(lexer.count());
    while let Some(token) = lexer.next() {
        let span = lexer.span();
        tokens.push(TokenSpan { token, span });
    }
    tokens
}
*/

/// Tokens in the language
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\n\f]+")] // white spaces
#[logos(skip r"//[^\n\r]*[\n\r]")] // single-line comments
#[logos(skip r"/\*([^*]|[\r\n]|(\*+([^*/]|[\r\n])))*\*+/")] // multi-line comments
pub enum Token {
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

    /// Unsigned 8-bits integer type
    #[token("u8")]
    U8,

    /// Unsigned 16-bits integer type
    #[token("u16")]
    U16,

    /// Unsigned 24-bits integer type
    #[token("u24")]
    U24,

    /// Unsigned 32-bits integer type
    #[token("u32")]
    U32,

    /// Unsigned 64-bits integer type
    #[token("u64")]
    U64,

    /// Signed 8-bits integer type
    #[token("i8")]
    I8,

    /// Signed 16-bits integer type
    #[token("i16")]
    I16,

    /// Signed 24-bits integer type
    #[token("i24")]
    I24,

    /// Signed 32-bits integer type
    #[token("i32")]
    I32,

    /// Signed 64-bits integer type
    #[token("i64")]
    I64,

    /// Floating point 16-bits number type
    #[token("f16")]
    F16,

    /// Floating point 32-bits number type
    #[token("f32")]
    F32,

    /// Floating point 64-bits number type
    #[token("f64")]
    F64,

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

/// Print the token
impl fmt::Display for Token {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DocComment(comment)    => write!(f, "/** {comment} */"),
            Self::Comma                  => write!(f, ","               ),
            Self::Period                 => write!(f, "."               ),
            Self::Colon                  => write!(f, ":"               ),
            Self::PathSep                => write!(f, "::"              ),
            Self::Semicolon              => write!(f, ";"               ),
            Self::ParenOpen              => write!(f, "("               ),
            Self::ParenClose             => write!(f, ")"               ),
            Self::BracketOpen            => write!(f, "["               ),
            Self::BracketClose           => write!(f, "]"               ),
            Self::BraceOpen              => write!(f, "{{"              ),
            Self::BraceClose             => write!(f, "}}"              ),
            Self::Assign                 => write!(f, "="               ),
            Self::Equal                  => write!(f, "=="              ),
            Self::Diff                   => write!(f, "!="              ),
            Self::Lesser                 => write!(f, "<"               ),
            Self::Greater                => write!(f, ">"               ),
            Self::LesserEq               => write!(f, "<="              ),
            Self::GreaterEq              => write!(f, ">="              ),
            Self::BoolNot                => write!(f, "!"               ),
            Self::BoolAnd                => write!(f, "&&"              ),
            Self::BoolOr                 => write!(f, "||"              ),
            Self::BitInv                 => write!(f, "~"               ),
            Self::Ampersand              => write!(f, "&"               ),
            Self::BitAndAssign           => write!(f, "&="              ),
            Self::BitOr                  => write!(f, "|"               ),
            Self::BitOrAssign            => write!(f, "|="              ),
            Self::BitXor                 => write!(f, "^"               ),
            Self::BitXorAssign           => write!(f, "^="              ),
            Self::Incr                   => write!(f, "++"              ),
            Self::Decr                   => write!(f, "--"              ),
            Self::Plus                   => write!(f, "+"               ),
            Self::PlusAssign             => write!(f, "+="              ),
            Self::PlusCarry              => write!(f, "+'"              ),
            Self::PlusCarryAssign        => write!(f, "+'="             ),
            Self::Minus                  => write!(f, "-"               ),
            Self::MinusAssign            => write!(f, "-="              ),
            Self::MinusCarry             => write!(f, "-'"              ),
            Self::MinusCarryAssign       => write!(f, "-'="             ),
            Self::Asterix                => write!(f, "*"               ),
            Self::MulAssign              => write!(f, "*="              ),
            Self::Div                    => write!(f, "/"               ),
            Self::DivAssign              => write!(f, "/="              ),
            Self::Rem                    => write!(f, "%"               ),
            Self::RemAssign              => write!(f, "%="              ),
            Self::ShiftLeft              => write!(f, "<<"              ),
            Self::ShiftLeftAssign        => write!(f, "<<="             ),
            Self::ShiftRightArith        => write!(f, ">>"              ),
            Self::ShiftRightArithAssign  => write!(f, ">>="             ),
            Self::ShiftRightLogic        => write!(f, ">>>"             ),
            Self::ShiftRightLogicAssign  => write!(f, ">>>="            ),
            Self::RotateLeft             => write!(f, "<+"              ),
            Self::RotateLeftAssign       => write!(f, "<+="             ),
            Self::RotateLeftCarry        => write!(f, "<+'"             ),
            Self::RotateLeftCarryAssign  => write!(f, "<+'="            ),
            Self::RotateRight            => write!(f, "+>"              ),
            Self::RotateRightAssign      => write!(f, "+>="             ),
            Self::RotateRightCarry       => write!(f, "+>'"             ),
            Self::RotateRightCarryAssign => write!(f, "+>'="            ),
            Self::Address                => write!(f, "@"               ),
            Self::RangeExc               => write!(f, ".."              ),
            Self::RangeInc               => write!(f, "..="             ),
            Self::False                  => write!(f, "false"           ),
            Self::True                   => write!(f, "true"            ),
            Self::Var                    => write!(f, "var"             ),
            Self::Let                    => write!(f, "let"             ),
            Self::Const                  => write!(f, "const"           ),
            Self::Func                   => write!(f, "fn"              ),
            Self::Mod                    => write!(f, "mod"             ),
            Self::If                     => write!(f, "if"              ),
            Self::Else                   => write!(f, "else"            ),
            Self::Loop                   => write!(f, "loop"            ),
            Self::For                    => write!(f, "for"             ),
            Self::In                     => write!(f, "in"              ),
            Self::Step                   => write!(f, "step"            ),
            Self::While                  => write!(f, "while"           ),
            Self::Do                     => write!(f, "do"              ),
            Self::Break                  => write!(f, "break"           ),
            Self::Continue               => write!(f, "continue"        ),
            Self::Return                 => write!(f, "return"          ),
            Self::Goto                   => write!(f, "goto"            ),
            Self::As                     => write!(f, "as"              ),
            Self::U8                     => write!(f, "u8"              ),
            Self::U16                    => write!(f, "u16"             ),
            Self::U24                    => write!(f, "u24"             ),
            Self::U32                    => write!(f, "u32"             ),
            Self::U64                    => write!(f, "u64"             ),
            Self::I8                     => write!(f, "i8"              ),
            Self::I16                    => write!(f, "i16"             ),
            Self::I24                    => write!(f, "i24"             ),
            Self::I32                    => write!(f, "i32"             ),
            Self::I64                    => write!(f, "i64"             ),
            Self::F16                    => write!(f, "f16"             ),
            Self::F32                    => write!(f, "f32"             ),
            Self::F64                    => write!(f, "f64"             ),
            Self::IntLit(int)            => write!(f, "{int}"           ),
            Self::FloatLit(float)        => write!(f, "{float}"         ),
            Self::CharLit(chr)           => write!(f, "'{chr}'"         ),
            Self::StrLit(string)         => write!(f, "\"{string}\""    ),
            Self::Label(label)           => write!(f, "'{label}"        ),
            Self::Ident(ident)           => write!(f, "{ident}"         ),
        }
    }
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
