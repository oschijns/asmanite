use crate::lexer::{
    atom::{assign, delimiter::*, operator::*, primitive::*, *},
    register::{RegIdent, is_register},
};
use core::hash::Hash;
use logos::{Lexer, Logos};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LexError;

impl From<()> for LexError {
    fn from(_: ()) -> Self {
        LexError
    }
}

// MARK: Token

/// Tokens in the language
#[repr(u16)]
#[derive(Logos, Debug, Clone, PartialEq, strum::EnumDiscriminants)]
#[logos(crate = logos, skip r"[ \t\n\f]+", error = LexError)]
#[strum_discriminants(name(TokenKind), derive(strum::Display, Hash))]
pub enum Token {
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

    /// Arrow for function's return type
    #[token("->")]
    Arrow,

    /// Fat-arrow for switch statements
    #[token("=>")]
    FatArrow,

    /// Anonymous identifier
    #[token("_", priority = 10)]
    Anonymous,

    /// Ampersat marker
    #[token("@")]
    Ampersat,

    /// Hash sign marker
    #[token("#")]
    Hash,

    /// Semi-Colon
    #[token(";")]
    Semicolon,

    /// Identify primitive types
    #[token("u8" , |_| U8 )]
    #[token("u16", |_| U16)]
    #[token("u24", |_| U24)]
    #[token("u32", |_| U32)]
    #[token("u64", |_| U64)]
    #[token("i8" , |_| I8 )]
    #[token("i16", |_| I16)]
    #[token("i24", |_| I24)]
    #[token("i32", |_| I32)]
    #[token("i64", |_| I64)]
    #[token("f16", |_| F16)]
    #[token("f32", |_| F32)]
    #[token("f64", |_| F64)]
    Primitive(Primitive),

    /// Delimitier '(', ')', '[', ']', '{', '}'
    #[token("(", |_| PAREN_OPEN   )]
    #[token(")", |_| PAREN_CLOSE  )]
    #[token("[", |_| BRACKET_OPEN )]
    #[token("]", |_| BRACKET_CLOSE)]
    #[token("{", |_| BRACE_OPEN   )]
    #[token("}", |_| BRACE_CLOSE  )]
    Delimiter(Delimiter),

    /// Operator
    #[token("==" , |_| EQ     )]
    #[token("!=" , |_| DIFF   )]
    #[token("<"  , |_| LT     )]
    #[token(">"  , |_| GT     )]
    #[token("<=" , |_| L_EQ   )]
    #[token(">=" , |_| G_EQ   )]
    #[token("!"  , |_| NOT    )]
    #[token("&&" , |_| AND    )]
    #[token("||" , |_| OR     )]
    #[token("~"  , |_| BIT_INV)]
    #[token("&"  , |_| BIT_AND)]
    #[token("|"  , |_| BIT_OR )]
    #[token("^"  , |_| BIT_XOR)]
    #[token("<<" , |_| SH_L   )]
    #[token(">>" , |_| SH_RA  )]
    #[token(">>>", |_| SH_RL  )]
    #[token("<+" , |_| ROT_L  )]
    #[token("+>" , |_| ROT_R  )]
    #[token("<+'", |_| ROT_L_C)]
    #[token("+>'", |_| ROT_R_C)]
    #[token("++" , |_| INC    )]
    #[token("--" , |_| DEC    )]
    #[token("+"  , |_| ADD    )]
    #[token("-"  , |_| SUB    )]
    #[token("*"  , |_| MUL    )]
    #[token("/"  , |_| DIV    )]
    #[token("%"  , |_| REM    )]
    #[token("+'" , |_| ADD_C  )]
    #[token("-'" , |_| SUB_C  )]
    #[token("*'" , |_| MUL_C  )]
    #[token("/'" , |_| DIV_C  )]
    #[token("%'" , |_| REM_C  )]
    Operator(Operator),

    /// Assign
    #[token("="   , |_| Assign::new())]
    #[token("&&=" , |_| assign::AND    )]
    #[token("||=" , |_| assign::OR     )]
    #[token("&="  , |_| assign::BIT_AND)]
    #[token("|="  , |_| assign::BIT_OR )]
    #[token("^="  , |_| assign::BIT_XOR)]
    #[token("<<=" , |_| assign::SH_L   )]
    #[token(">>=" , |_| assign::SH_RA  )]
    #[token(">>>=", |_| assign::SH_RL  )]
    #[token("<+=" , |_| assign::ROT_L  )]
    #[token("+>=" , |_| assign::ROT_R  )]
    #[token("<+'=", |_| assign::ROT_L_C)]
    #[token("+>'=", |_| assign::ROT_R_C)]
    #[token("+="  , |_| assign::ADD    )]
    #[token("-="  , |_| assign::SUB    )]
    #[token("*="  , |_| assign::MUL    )]
    #[token("/="  , |_| assign::DIV    )]
    #[token("%="  , |_| assign::REM    )]
    #[token("+'=" , |_| assign::ADD_C  )]
    #[token("-'=" , |_| assign::SUB_C  )]
    #[token("*'=" , |_| assign::MUL_C  )]
    #[token("/'=" , |_| assign::DIV_C  )]
    #[token("%'=" , |_| assign::REM_C  )]
    Assign(Assign),

    /// Range is only used in for-loop
    #[token(".." , |_| Range::excl())]
    #[token("..=", |_| Range::incl())]
    Range(Range),

    /// Declare a variable/constant
    #[token("var"   , |_| VarType::Var   )]
    #[token("let"   , |_| VarType::Let   )]
    #[token("static", |_| VarType::Static)]
    #[token("const" , |_| VarType::Const )]
    VarDecl(VarType),

    /// Declare a variable/constant
    #[token("struct", |_| TypeDef::Struct)]
    #[token("enum"  , |_| TypeDef::Enum  )]
    #[token("union" , |_| TypeDef::Union )]
    TypeDef(TypeDef),

    /// Declare a module
    #[token("mod")]
    Mod,

    /// Inline marker for function or for-loop
    #[token("inline")]
    Inline,

    /// Implementation block
    #[token("impl")]
    Impl,

    /// Operator overloading
    #[token("op")]
    Overloading,

    /// Declare a function
    #[token("fn")]
    Func,

    /// Conditional branching
    #[token("if")]
    If,

    /// Conditional branching
    #[token("else")]
    Else,

    /// Switch statement
    #[token("switch")]
    Switch,

    /// Infinite loop
    #[token("loop")]
    Loop,

    /// For-loop
    #[token("for")]
    For,

    /// For-loop range
    #[token("of")]
    Of,

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

    /// Fallthrough switch case
    #[token("fallthrough")]
    Fallthrough,

    /// Goto label
    #[token("goto")]
    Goto,

    /// Defer operations when the scope is exited
    #[token("defer")]
    Defer,

    /// Return a value
    #[token("return")]
    Return,

    /// Include raw assembly
    #[token("asm")]
    Asm,

    /// Cast type
    #[token("as")]
    As,

    /// Specify which bank to use to store the variable
    #[token("in")]
    In,

    /// Specify fixed address location
    #[token("at")]
    At,

    /// Literal
    #[token("false"                      , |_| Literal::Bool (BoolLit(false)))]
    #[token("true"                       , |_| Literal::Bool (BoolLit(true )))]
    #[regex(r"'[^']'"                    , |c| Literal::Char (CharLit ::parse    (c)))]
    #[regex(r"'\\.'"                     , |c| Literal::Char (CharLit ::parse_esc(c)))]
    #[regex(r"'\\x[0-9A-Fa-f]{2}'"       , |c| Literal::Char (CharLit ::parse_hex(c)))]
    #[regex(r"\d[0-9_]*"                 , |i| Literal::Int  (IntLit  ::parse    (i)))]
    #[regex(r"0b[01][01_]*"              , |i| Literal::Int  (IntLit  ::parse    (i)))]
    #[regex(r"0o[0-7][0-7_]*"            , |i| Literal::Int  (IntLit  ::parse    (i)))]
    #[regex(r"0x[0-9A-Fa-f][0-9A-Fa-f_]*", |i| Literal::Int  (IntLit  ::parse    (i)))]
    #[regex(r"\d+\.\d+([eE][+-]?\d+)?"   , |f| Literal::Float(FloatLit::parse    (f)))]
    #[regex(r"\.\d+([eE][+-]?\d+)?"      , |f| Literal::Float(FloatLit::parse    (f)))]
    #[regex(r#""([^"\\]|\\.)*""#         , |s| Literal::Str  (StrLit  ::parse    (s)))]
    Lit(Literal),

    /// Label
    #[regex(r"'[A-Za-z_]\w*", Label::parse)]
    Label(Label),

    /// Identifier
    #[regex(r"[A-Za-z_]\w*", IdentOrReg::parse)]
    Ident(IdentOrReg),
}

/// Successfully identified register or common identifier
#[derive(Debug, Clone, PartialEq)]
pub enum IdentOrReg {
    /// The token is a register name
    /// Note: register names are reserved on initialization
    Reg(RegIdent),

    /// The token is a generic identifier
    Ident(Ident),
}

// MARK: Parse token

/// Parse single-line doc-comment `/// ...`
#[allow(unused)]
#[inline]
fn parse_single_comment<'s>(lex: &Lexer<'s, Token>) -> String {
    // skip `///`
    lex.slice()[3..].to_string()
}

/// Parse multi-line doc-comment `/** ... */`
#[allow(unused)]
#[inline]
fn parse_multi_comment<'s>(lex: &Lexer<'s, Token>) -> String {
    // remove the `/**` and `*/` on both ends
    let seq = lex.slice();
    seq[3..seq.len() - 2].to_string()
}

impl CharLit {
    /// Parse character literal `'a'`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        // skip `'`
        let c = lex.slice().chars().nth(1).unwrap();
        Self(c)
    }

    /// Parse character literal `'\0'`
    #[inline]
    fn parse_esc<'s>(lex: &Lexer<'s, Token>) -> Self {
        // skip `'\`
        let c = lex.slice().chars().nth(2).unwrap();
        let c = match c {
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
        };
        Self(c)
    }

    /// Parse character literal `'\x00'`
    #[inline]
    fn parse_hex<'s>(lex: &Lexer<'s, Token>) -> Self {
        // skip `'\x`
        let c = u8::from_str_radix(&lex.slice()[3..5], 16).unwrap() as char;
        Self(c)
    }
}

impl IntLit {
    /// Parse integer literal `0`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        let literal = lex.slice().replace('_', "");

        // Check if the literal has a prefix
        let (digits, radix) = if literal.len() >= 2 {
            match &literal[..2] {
                "0b" => (&literal[2..], 2),
                "0o" => (&literal[2..], 8),
                "0x" => (&literal[2..], 16),
                _ => (literal.as_str(), 10),
            }
        } else {
            (literal.as_str(), 10)
        };
        Self(
            u128::from_str_radix(digits, radix)
                .expect("digits should have been validated by regex beforehand"),
        )
    }
}

impl FloatLit {
    /// Parse floating point number literal `0.0`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        let f = lex.slice().parse::<f64>().unwrap();
        Self(f)
    }
}

impl StrLit {
    /// Parse string literal `"..."`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        // remove the double quotes on both ends
        let seq = lex.slice();
        let s = seq[1..seq.len() - 1].to_string();
        Self(s.into_boxed_str())
    }
}

impl Label {
    /// Parse label literal `'label`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        let l = lex.slice()[1..].to_string();
        Self(l.into_boxed_str())
    }
}

impl IdentOrReg {
    /// Parse register name 'a' or identifier `ident`
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        let i = lex.slice().to_string();

        if let Some(reg) = is_register(&i) {
            Self::Reg(reg)
        } else {
            Self::Ident(Ident(i.into_boxed_str()))
        }
    }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    //use super::*;
}
