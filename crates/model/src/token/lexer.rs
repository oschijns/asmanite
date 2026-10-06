use crate::token::define::*;
use core::hash::Hash;
use tokora::logos::{self, Lexer, Logos};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LexError;

impl From<()> for LexError {
    fn from(_: ()) -> Self {
        LexError
    }
}

/// Tokens in the language
#[repr(u16)]
#[derive(Logos, Debug, Clone, strum::EnumDiscriminants)]
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

    /// Semi-Colon
    #[token(";")]
    Semicolon,

    /// Identify primitive types
    #[token("u8" , |_| Primitive::new_bits(PrimType::UInt ,  8))]
    #[token("u16", |_| Primitive::new_bits(PrimType::UInt , 16))]
    #[token("u24", |_| Primitive::new_bits(PrimType::UInt , 24))]
    #[token("u32", |_| Primitive::new_bits(PrimType::UInt , 32))]
    #[token("u64", |_| Primitive::new_bits(PrimType::UInt , 64))]
    #[token("i8" , |_| Primitive::new_bits(PrimType::Int  ,  8))]
    #[token("i16", |_| Primitive::new_bits(PrimType::Int  , 16))]
    #[token("i24", |_| Primitive::new_bits(PrimType::Int  , 24))]
    #[token("i32", |_| Primitive::new_bits(PrimType::Int  , 32))]
    #[token("i64", |_| Primitive::new_bits(PrimType::Int  , 64))]
    #[token("f16", |_| Primitive::new_bits(PrimType::Float, 16))]
    #[token("f32", |_| Primitive::new_bits(PrimType::Float, 32))]
    #[token("f64", |_| Primitive::new_bits(PrimType::Float, 64))]
    Primitive(Primitive),

    /// Delimitier '(', ')', '[', ']', '{', '}'
    #[token("(", |_| Delimiter::open (DelimType::Paren  ))]
    #[token(")", |_| Delimiter::close(DelimType::Paren  ))]
    #[token("[", |_| Delimiter::open (DelimType::Bracket))]
    #[token("]", |_| Delimiter::close(DelimType::Bracket))]
    #[token("{", |_| Delimiter::open (DelimType::Brace  ))]
    #[token("}", |_| Delimiter::close(DelimType::Brace  ))]
    Delimiter(Delimiter),

    /// Operator
    #[token("==" , |_| Operator::new  (OpType::Eq    ))]
    #[token("!=" , |_| Operator::new  (OpType::Diff  ))]
    #[token("<"  , |_| Operator::new  (OpType::Lt    ))]
    #[token(">"  , |_| Operator::new  (OpType::Gt    ))]
    #[token("<=" , |_| Operator::new  (OpType::LEq   ))]
    #[token(">=" , |_| Operator::new  (OpType::GEq   ))]
    #[token("!"  , |_| Operator::new  (OpType::Not   ))]
    #[token("&&" , |_| Operator::new  (OpType::And   ))]
    #[token("||" , |_| Operator::new  (OpType::Or    ))]
    #[token("~"  , |_| Operator::new  (OpType::BitInv))]
    #[token("&"  , |_| Operator::new  (OpType::BitAnd))]
    #[token("|"  , |_| Operator::new  (OpType::BitOr ))]
    #[token("^"  , |_| Operator::new  (OpType::BitXor))]
    #[token("<<" , |_| Operator::new  (OpType::ShL   ))]
    #[token(">>" , |_| Operator::new  (OpType::ShRA  ))]
    #[token(">>>", |_| Operator::new  (OpType::ShRL  ))]
    #[token("<+" , |_| Operator::new  (OpType::RotL  ))]
    #[token("+>" , |_| Operator::new  (OpType::RotR  ))]
    #[token("<+'", |_| Operator::carry(OpType::RotL  ))]
    #[token("+>'", |_| Operator::carry(OpType::RotR  ))]
    #[token("++" , |_| Operator::new  (OpType::Inc   ))]
    #[token("--" , |_| Operator::new  (OpType::Dec   ))]
    #[token("+"  , |_| Operator::new  (OpType::Add   ))]
    #[token("-"  , |_| Operator::new  (OpType::Sub   ))]
    #[token("*"  , |_| Operator::new  (OpType::Mul   ))]
    #[token("/"  , |_| Operator::new  (OpType::Div   ))]
    #[token("%"  , |_| Operator::new  (OpType::Rem   ))]
    #[token("+'" , |_| Operator::carry(OpType::Add   ))]
    #[token("-'" , |_| Operator::carry(OpType::Sub   ))]
    #[token("*'" , |_| Operator::carry(OpType::Mul   ))]
    #[token("/'" , |_| Operator::carry(OpType::Div   ))]
    #[token("%'" , |_| Operator::carry(OpType::Rem   ))]
    Operator(Operator),

    /// Assign
    #[token("="   , |_| Assign::new())]
    #[token("&&=" , |_| Assign::op      (OpType::And   ))]
    #[token("||=" , |_| Assign::op      (OpType::Or    ))]
    #[token("~="  , |_| Assign::op      (OpType::BitInv))]
    #[token("&="  , |_| Assign::op      (OpType::BitAnd))]
    #[token("|="  , |_| Assign::op      (OpType::BitOr ))]
    #[token("^="  , |_| Assign::op      (OpType::BitXor))]
    #[token("<<=" , |_| Assign::op      (OpType::ShL   ))]
    #[token(">>=" , |_| Assign::op      (OpType::ShRA  ))]
    #[token(">>>=", |_| Assign::op      (OpType::ShRL  ))]
    #[token("<+=" , |_| Assign::op      (OpType::RotL  ))]
    #[token("+>=" , |_| Assign::op      (OpType::RotR  ))]
    #[token("<+'=", |_| Assign::op_carry(OpType::RotL  ))]
    #[token("+>'=", |_| Assign::op_carry(OpType::RotR  ))]
    #[token("+="  , |_| Assign::op      (OpType::Add   ))]
    #[token("-="  , |_| Assign::op      (OpType::Sub   ))]
    #[token("*="  , |_| Assign::op      (OpType::Mul   ))]
    #[token("/="  , |_| Assign::op      (OpType::Div   ))]
    #[token("%="  , |_| Assign::op      (OpType::Rem   ))]
    #[token("+'=" , |_| Assign::op_carry(OpType::Add   ))]
    #[token("-'=" , |_| Assign::op_carry(OpType::Sub   ))]
    #[token("*'=" , |_| Assign::op_carry(OpType::Mul   ))]
    #[token("/'=" , |_| Assign::op_carry(OpType::Div   ))]
    #[token("%'=" , |_| Assign::op_carry(OpType::Rem   ))]
    Assign(Assign),

    /// Range is only used in for-loop
    #[token(".." , |_| Range::excl())]
    #[token("..=", |_| Range::incl())]
    Range(Range),

    /// Address Operator
    #[token("@")]
    Address,

    /// Declare a variable/constant
    #[token("var"   , |_| VarDecl::Var   )]
    #[token("let"   , |_| VarDecl::Let   )]
    #[token("static", |_| VarDecl::Static)]
    #[token("const" , |_| VarDecl::Const )]
    VarDecl(VarDecl),

    /// Declare a variable/constant
    #[token("struct", |_| TypeDef::Struct)]
    #[token("enum"  , |_| TypeDef::Enum  )]
    #[token("union" , |_| TypeDef::Union )]
    TypeDef(TypeDef),

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

    /// Boolean literal
    #[token("false", |_| BoolLit(false))]
    #[token("true" , |_| BoolLit(true ))]
    BoolLit(BoolLit),

    /// Character literal
    #[regex(r"'[^']'", CharLit::parse)]
    #[regex(r"'\\.'", CharLit::parse_esc)]
    #[regex(r"'\\x[0-9A-Fa-f]{2}'", CharLit::parse_hex)]
    CharLit(CharLit),

    /// Integer literal
    #[regex(r"\d[0-9_]*", IntLit::parse)]
    #[regex(r"0b[01][01_]*", IntLit::parse)]
    #[regex(r"0o[0-7][0-7_]*", IntLit::parse)]
    #[regex(r"0x[0-9A-Fa-f][0-9A-Fa-f_]*", IntLit::parse)]
    IntLit(IntLit),

    /// Floating point literal
    #[regex(r"\d+\.\d*([eE][+-]?\d+)?", FloatLit::parse)]
    #[regex(r"\.\d+([eE][+-]?\d+)?", FloatLit::parse)]
    FloatLit(FloatLit),

    /// String literal
    #[regex(r#""([^"\\]|\\.)*""#, StrLit::parse)]
    StrLit(StrLit),

    /// Label
    #[regex(r"'[A-Za-z_]\w*", Label::parse)]
    Label(Label),

    /// Identifier
    #[regex(r"[A-Za-z_]\w*", Ident::parse)]
    Ident(Ident),
}

// Define how tokora pull tokens
impl tokora::Token<'_> for Token {
    type Kind = TokenKind;
    type Error = LexError;

    const SCAN_LOOKAHEAD: tokora::ScanLookahead = tokora::ScanLookahead::Unbounded;

    fn kind(&self) -> TokenKind {
        TokenKind::from(self)
    }
    fn is_trivia(&self) -> bool {
        false
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
        let i = lex.slice().parse::<u128>().unwrap();
        Self(i)
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

impl Ident {
    /// Parse identifier `ident`
    #[inline]
    fn parse<'s>(lex: &Lexer<'s, Token>) -> Self {
        let i = lex.slice().to_string();
        Self(i.into_boxed_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
