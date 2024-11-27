use logos::Logos;

// https://github.com/wiz-lang/wiz?tab=readme-ov-file

/// Tokens in the language
#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    /// Script Escape
    #[regex("#[[.*]]#")]
    EmbeddedScript,

    /// Token
    #[token(",")]
    Comma,

    /// Token
    #[token(".")]
    Period,

    /// Token
    #[token(":")]
    Colon,

    /// Token
    #[token(";")]
    Semicolon,

    /// Token
    #[token("(")]
    ParenStart,

    /// Token
    #[token(")")]
    ParenEnd,

    /// Token
    #[token("[")]
    BracketStart,

    /// Token
    #[token("]")]
    BracketEnd,

    /// Token
    #[token("{")]
    BraceStart,

    /// Token
    #[token("}")]
    BraceEnd,

    /// Operator
    #[token("=")]
    Assign,

    /// Comparison Operator
    #[token("==")]
    Equal,

    /// Comparison Operator
    #[token("!=")]
    Diff,

    /// Comparison Operator
    #[token("<")]
    Lesser,

    /// Comparison Operator
    #[token(">")]
    Greater,

    /// Comparison Operator
    #[token("<=")]
    LesserEq,

    /// Comparison Operator
    #[token(">=")]
    GreaterEq,

    /// Boolean Operator
    #[token("not")]
    #[token("!")]
    BoolNot,

    /// Boolean Operator
    #[token("and")]
    #[token("&&")]
    BoolAnd,

    /// Boolean Operator
    #[token("or")]
    #[token("||")]
    BoolOr,

    /// Bitwise Operator
    #[token("~")]
    BitInv,

    /// Bitwise Operator
    #[token("&")]
    BitAnd,

    /// Bitwise Operator
    #[token("|")]
    BitOr,

    /// Bitwise Operator
    #[token("xor")]
    #[token("^")]
    BitXor,

    /// Arithmetic Operator
    #[token("+")]
    Plus,

    /// Arithmetic Operator
    #[token("-")]
    Minus,

    /// Logical Bitshift
    #[token("<<")]
    ShiftLeft,

    /// Logical Bitshift
    #[token(">>")]
    ShiftRight,

    /// Operator or pointer
    #[token("*")]
    Asterix,

    /// Address Operator
    #[token("@")]
    Address,

    /// Range Operator
    #[token("..")]
    RangeExclusive,

    /// Range Operator
    #[token("..=")]
    RangeInclusive,

    /// Keyword
    #[token("low")]
    ByteLow,

    /// Keyword
    #[token("high")]
    ByteHigh,

    /// Keyword
    #[token("mod")]
    Mod,

    /// Keyword
    #[token("if")]
    If,

    /// Keyword
    #[token("else")]
    Else,

    /// Keyword
    #[token("loop")]
    Loop,

    /// Keyword
    #[token("for")]
    For,

    /// Keyword
    #[token("while")]
    While,

    /// Keyword
    #[token("do")]
    Do,

    /// Keyword
    #[token("break")]
    Break,

    /// Keyword
    #[token("continue")]
    Continue,

    /// Keyword
    #[token("let")]
    Let,

    /// Keyword
    #[token("const")]
    Const,

    /// Keyword
    #[token("fn")]
    Func,

    /// Keyword
    #[token("return")]
    Return,

    /// Keyword
    #[token("goto")]
    Goto,
}
