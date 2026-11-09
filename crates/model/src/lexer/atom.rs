use chumsky::pratt;
use std::fmt::{self, Write};

/// Implement Display trait
macro_rules! impl_display {
    ($type:ty) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
    ($type:ty, $fmt:literal) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, $fmt, self.0)
            }
        }
    };
    ($type:ty, str) => {
        impl fmt::Display for $type {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.into())
            }
        }
    };
}

/// Define the precedence of an operator.
/// Higher values mean the operator have a higher priority than others.
pub type Precedence = u16;

/// Define the associativity of the operation
#[repr(u8)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Associativity {
    /// No associativity (parenthesis are required)
    #[default]
    None,

    /// Left to right associativity
    Left,

    /// Right to left associativity
    Right,
}

impl Associativity {
    #[rustfmt::skip]
    pub fn to_chumsky(self, precedence: Precedence) -> pratt::Associativity {
        match self {
            Self::None  => pratt::none (precedence),
            Self::Left  => pratt::left (precedence),
            Self::Right => pratt::right(precedence),
        }
    }
}

/// Associativity of type cast operation
pub const ASSOC_TYPE_CAST: Associativity = Associativity::Left;

/// Precendence of type cast operation
pub const PREC_TYPE_CAST: Precedence = 39;

// MARK: PRIMITIVE

#[rustfmt::skip]
pub mod primitive {
    use super::{Primitive, PrimType};

    pub const U8 : Primitive = Primitive::new_bits(PrimType::UInt ,  8);
    pub const U16: Primitive = Primitive::new_bits(PrimType::UInt , 16);
    pub const U24: Primitive = Primitive::new_bits(PrimType::UInt , 24);
    pub const U32: Primitive = Primitive::new_bits(PrimType::UInt , 32);
    pub const U64: Primitive = Primitive::new_bits(PrimType::UInt , 64);
    pub const I8 : Primitive = Primitive::new_bits(PrimType::Int  ,  8);
    pub const I16: Primitive = Primitive::new_bits(PrimType::Int  , 16);
    pub const I24: Primitive = Primitive::new_bits(PrimType::Int  , 24);
    pub const I32: Primitive = Primitive::new_bits(PrimType::Int  , 32);
    pub const I64: Primitive = Primitive::new_bits(PrimType::Int  , 64);
    pub const F16: Primitive = Primitive::new_bits(PrimType::Float, 16);
    pub const F32: Primitive = Primitive::new_bits(PrimType::Float, 32);
    pub const F64: Primitive = Primitive::new_bits(PrimType::Float, 64);
}

/// Define a primitive type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Primitive {
    /// Type of the primitive uint, int or float
    pub prim_type: PrimType,

    /// Size of the primitive in bytes
    pub size: u8,
}

impl Primitive {
    /// Define a primitive
    #[inline]
    pub const fn new(prim_type: PrimType, size: u8) -> Self {
        Self { prim_type, size }
    }

    /// Define a primitive with a size in bits
    #[inline]
    pub const fn new_bits(prim_type: PrimType, bitsize: u8) -> Self {
        Self {
            prim_type,
            size: bitsize >> 3,
        }
    }
}

impl fmt::Display for Primitive {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.prim_type {
            PrimType::UInt  => 'u',
            PrimType::Int   => 'i',
            PrimType::Float => 'f',
        };
        write!(f, "{}{}", prefix, self.size * 8)
    }
}

/// Type of primitive values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum PrimType {
    /// Unsigned integer
    #[strum(serialize = "uint")]
    UInt,

    /// Signed integer
    #[strum(serialize = "int")]
    Int,

    /// Floating point number
    #[strum(serialize = "float")]
    Float,
}
impl_display!(PrimType, str);

// MARK: ASSIGN

#[rustfmt::skip]
pub mod assign {
    use super::{Assign, OpType};

    pub const AND    : Assign = Assign::op      (OpType::And   );
    pub const OR     : Assign = Assign::op      (OpType::Or    );
    pub const BIT_AND: Assign = Assign::op      (OpType::BitAnd);
    pub const BIT_OR : Assign = Assign::op      (OpType::BitOr );
    pub const BIT_XOR: Assign = Assign::op      (OpType::BitXor);
    pub const SH_L   : Assign = Assign::op      (OpType::ShL   );
    pub const SH_RA  : Assign = Assign::op      (OpType::ShRA  );
    pub const SH_RL  : Assign = Assign::op      (OpType::ShRL  );
    pub const ROT_L  : Assign = Assign::op      (OpType::RotL  );
    pub const ROT_R  : Assign = Assign::op      (OpType::RotR  );
    pub const ROT_L_C: Assign = Assign::op_carry(OpType::RotL  );
    pub const ROT_R_C: Assign = Assign::op_carry(OpType::RotR  );
    pub const ADD    : Assign = Assign::op      (OpType::Add   );
    pub const SUB    : Assign = Assign::op      (OpType::Sub   );
    pub const MUL    : Assign = Assign::op      (OpType::Mul   );
    pub const DIV    : Assign = Assign::op      (OpType::Div   );
    pub const REM    : Assign = Assign::op      (OpType::Rem   );
    pub const ADD_C  : Assign = Assign::op_carry(OpType::Add   );
    pub const SUB_C  : Assign = Assign::op_carry(OpType::Sub   );
    pub const MUL_C  : Assign = Assign::op_carry(OpType::Mul   );
    pub const DIV_C  : Assign = Assign::op_carry(OpType::Div   );
    pub const REM_C  : Assign = Assign::op_carry(OpType::Rem   );
}

/// Assignment instruction
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Assign {
    /// Optional operator while doing the assignment
    pub op: Option<Operator>,
}

/// Associativity of the assignement expression is always right to left
pub const ASSOC_ASSIGN: Associativity = Associativity::Right;

/// Precendence of assignement expression is always the same
pub const PREC_ASSIGN: Precedence = 10;

impl Assign {
    /// Assignment
    #[inline]
    pub const fn new() -> Self {
        Self { op: None }
    }

    /// New operator
    #[inline]
    pub const fn op(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::new(op_type)),
        }
    }

    /// New operator with carry
    #[inline]
    pub const fn op_carry(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::carry(op_type)),
        }
    }
}

impl fmt::Display for Assign {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(op) = self.op {
            op.fmt(f)?;
        }
        f.write_char('=')
    }
}

// MARK: OPERATOR

#[rustfmt::skip]
pub mod operator {
    use super::{Operator, OpType};

    pub const EQ     : Operator = Operator::new  (OpType::Eq    );
    pub const DIFF   : Operator = Operator::new  (OpType::Diff  );
    pub const LT     : Operator = Operator::new  (OpType::Lt    );
    pub const GT     : Operator = Operator::new  (OpType::Gt    );
    pub const L_EQ   : Operator = Operator::new  (OpType::LEq   );
    pub const G_EQ   : Operator = Operator::new  (OpType::GEq   );
    pub const NOT    : Operator = Operator::new  (OpType::Not   );
    pub const AND    : Operator = Operator::new  (OpType::And   );
    pub const OR     : Operator = Operator::new  (OpType::Or    );
    pub const BIT_INV: Operator = Operator::new  (OpType::BitInv);
    pub const BIT_AND: Operator = Operator::new  (OpType::BitAnd);
    pub const BIT_OR : Operator = Operator::new  (OpType::BitOr );
    pub const BIT_XOR: Operator = Operator::new  (OpType::BitXor);
    pub const SH_L   : Operator = Operator::new  (OpType::ShL   );
    pub const SH_RA  : Operator = Operator::new  (OpType::ShRA  );
    pub const SH_RL  : Operator = Operator::new  (OpType::ShRL  );
    pub const ROT_L  : Operator = Operator::new  (OpType::RotL  );
    pub const ROT_R  : Operator = Operator::new  (OpType::RotR  );
    pub const ROT_L_C: Operator = Operator::carry(OpType::RotL  );
    pub const ROT_R_C: Operator = Operator::carry(OpType::RotR  );
    pub const INC    : Operator = Operator::new  (OpType::Inc   );
    pub const DEC    : Operator = Operator::new  (OpType::Dec   );
    pub const ADD    : Operator = Operator::new  (OpType::Add   );
    pub const SUB    : Operator = Operator::new  (OpType::Sub   );
    pub const MUL    : Operator = Operator::new  (OpType::Mul   );
    pub const DIV    : Operator = Operator::new  (OpType::Div   );
    pub const REM    : Operator = Operator::new  (OpType::Rem   );
    pub const ADD_C  : Operator = Operator::carry(OpType::Add   );
    pub const SUB_C  : Operator = Operator::carry(OpType::Sub   );
    pub const MUL_C  : Operator = Operator::carry(OpType::Mul   );
    pub const DIV_C  : Operator = Operator::carry(OpType::Div   );
    pub const REM_C  : Operator = Operator::carry(OpType::Rem   );
}

/// Operator with optional carry flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operator {
    /// The type of operator
    pub op_type: OpType,

    /// Should we use the carry flag or clear it ?
    pub carry: bool,
}

impl Operator {
    /// New operator
    #[inline]
    pub const fn new(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: false,
        }
    }

    /// New operator with carry
    #[inline]
    pub const fn carry(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: true,
        }
    }
}

impl fmt::Display for Operator {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.op_type.into())?;
        if self.carry {
            f.write_char('\'')?;
        }
        Ok(())
    }
}

/// Type of operator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::EnumIter, strum::IntoStaticStr)]
pub enum OpType {
    /// Equality '=='
    #[strum(serialize = "==")]
    Eq,

    /// Difference '!='
    #[strum(serialize = "!=")]
    Diff,

    /// Lesser than '<'
    #[strum(serialize = "<")]
    Lt,

    /// Greater than '>'
    #[strum(serialize = ">")]
    Gt,

    /// Lesser or equal than '<='
    #[strum(serialize = "<=")]
    LEq,

    /// Greater or equal than '>='
    #[strum(serialize = ">=")]
    GEq,

    /// Logical NOT '!'
    #[strum(serialize = "!")]
    Not,

    /// Logical AND '&&'
    #[strum(serialize = "&&")]
    And,

    /// Logical OR '||'
    #[strum(serialize = "||")]
    Or,

    /// Bitwise INV '~'
    #[strum(serialize = "~")]
    BitInv,

    /// Bitwise AND '&'
    #[strum(serialize = "&")]
    BitAnd,

    /// Bitwise OR '|'
    #[strum(serialize = "|")]
    BitOr,

    /// Bitwise XOR '^'
    #[strum(serialize = "^")]
    BitXor,

    /// Shift Left '<<'
    #[strum(serialize = "<<")]
    ShL,

    /// Shift Right (arithmetic) '>>'
    #[strum(serialize = ">>")]
    ShRA,

    /// Shift Right (logical) '>>>'
    #[strum(serialize = ">>>")]
    ShRL,

    /// Rotate bits Left '<+'
    #[strum(serialize = "<+")]
    RotL,

    /// Rotate bits Right '+>'
    #[strum(serialize = "+>")]
    RotR,

    /// Increment '++'
    #[strum(serialize = "++")]
    Inc,

    /// Decrement '--'
    #[strum(serialize = "--")]
    Dec,

    /// Addition '+'
    #[strum(serialize = "+")]
    Add,

    /// Subtraction '-'
    #[strum(serialize = "-")]
    Sub,

    /// Multiplication '*'
    #[strum(serialize = "*")]
    Mul,

    /// Division '/'
    #[strum(serialize = "/")]
    Div,

    /// Remainder '%'
    #[strum(serialize = "%")]
    Rem,
}
impl_display!(OpType, str);

/// Precendence of unary operators
pub const PREC_UNARY: Precedence = 40;

impl OpType {
    /// Can the operator by used as a binary operator?
    #[rustfmt::skip]
    pub const fn is_binary(self) -> bool {
        matches!(self,
            Self::Eq     | // ==
            Self::Diff   | // !=
            Self::Lt     | // <
            Self::Gt     | // >
            Self::LEq    | // <=
            Self::GEq    | // >=
            Self::And    | // &&
            Self::Or     | // ||
            Self::BitAnd | // &
            Self::BitOr  | // |
            Self::BitXor | // ^
            Self::ShL    | // <<
            Self::ShRA   | // >>
            Self::ShRL   | // >>>
            Self::RotL   | // <+
            Self::RotR   | // +>
            Self::Add    | // +
            Self::Sub    | // -
            Self::Mul    | // *
            Self::Div    | // /
            Self::Rem      // %
        )
    }

    /// Can the operator by used as a unary operator?
    #[rustfmt::skip]
    pub const fn is_unary(self) -> bool {
        matches!(self,
            OpType::Not    | // !
            OpType::BitInv | // ~
            OpType::BitAnd | // &
            OpType::Inc    | // ++
            OpType::Dec    | // --
            OpType::Add    | // +
            OpType::Sub    | // -
            OpType::Mul      // *
        )
    }

    /// Provide the precedence of the binary operators.
    #[rustfmt::skip]
    pub const fn precedence_binary(self) -> Precedence {
        match self {
            Self::Mul    |      // *
            Self::Div    |      // /
            Self::Rem    => 29, // %
            Self::Add    |      // +
            Self::Sub    => 28, // -
            Self::ShL    |      // <<
            Self::ShRA   |      // >>
            Self::ShRL   |      // >>>
            Self::RotL   |      // <+
            Self::RotR   => 27, // +>
            Self::BitAnd => 26, // &
            Self::BitXor => 25, // ^
            Self::BitOr  => 24, // |
            Self::Eq     |      // ==
            Self::Diff   |      // !=
            Self::Lt     |      // <
            Self::Gt     |      // >
            Self::LEq    |      // <=
            Self::GEq    => 23, // >=
            Self::And    => 22, // &&
            Self::Or     => 21, // ||
            _ => Precedence::MIN,
        }
    }

    /// Provide the precedence of the unary operators.
    #[rustfmt::skip]
    pub const fn precedence_unary(self) -> Precedence {
        match self {
            Self::Not    | // !
            Self::BitInv | // ~
            Self::Inc    | // ++
            Self::Dec    | // --
            Self::Add    | // +
            Self::Sub    | // -
            Self::Mul    | // *
            Self::BitAnd   // &
              => PREC_UNARY,
            _ => Precedence::MIN,
        }
    }

    /// Provide the associativity of the operators.
    #[rustfmt::skip]
    pub const fn associativity_binary(self) -> Associativity {
        if matches!(self,
            Self::Eq   | // ==
            Self::Diff | // !=
            Self::Lt   | // <
            Self::Gt   | // >
            Self::LEq  | // <=
            Self::GEq    // >=
        ) {
            Associativity::None
        } else {
            Associativity::Left
        }
    }

    /// Provide the associativity of the operators.
    #[inline]
    pub const fn associativity_unary(self) -> Associativity {
        Associativity::None
    }

    /// Get a bitmask for the given operator, this allows for grouping
    /// operators with similar properties (associativity and precedence).
    #[inline]
    pub const fn bitmask(self) -> u64 {
        1u64 << (self as u64)
    }
}

// MARK: DELIMITER

#[rustfmt::skip]
pub mod delimiter {
    use super::{Delimiter, DelimType};

    pub const PAREN_OPEN   : Delimiter = Delimiter::open (DelimType::Paren  );
    pub const PAREN_CLOSE  : Delimiter = Delimiter::close(DelimType::Paren  );
    pub const BRACKET_OPEN : Delimiter = Delimiter::open (DelimType::Bracket);
    pub const BRACKET_CLOSE: Delimiter = Delimiter::close(DelimType::Bracket);
    pub const BRACE_OPEN   : Delimiter = Delimiter::open (DelimType::Brace  );
    pub const BRACE_CLOSE  : Delimiter = Delimiter::close(DelimType::Brace  );
}

/// Precendence of expression enclosed in parenthesis
pub const PREC_PAREN: Precedence = 100;

/// Define a delimiter token '(', ')', '[', ']', '{', '}'
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delimiter {
    /// Type of delimiter
    pub delim_type: DelimType,

    /// Is it an open or close delimiter
    pub is_close: bool,
}

impl Delimiter {
    /// Create a new open delimiter
    #[inline]
    pub const fn open(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: false,
        }
    }

    /// Create a new close delimiter
    #[inline]
    pub const fn close(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: true,
        }
    }
}

impl fmt::Display for Delimiter {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = if self.is_close {
            match self.delim_type {
                DelimType::Paren   => ')',
                DelimType::Brace   => '}',
                DelimType::Bracket => ']',
            }
        } else {
            match self.delim_type {
                DelimType::Paren   => '(',
                DelimType::Brace   => '{',
                DelimType::Bracket => '[',
            }
        };
        f.write_char(d)
    }
}

/// Type of delimiter
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum DelimType {
    /// Parenthesis '(', ')'
    #[strum(serialize = "()")]
    Paren,

    /// Brace '{', '}'
    #[strum(serialize = "{}")]
    Brace,

    /// Bracket '[', ']'
    #[strum(serialize = "[]")]
    Bracket,
}
impl_display!(DelimType, str);

// MARK: RANGE

/// Range '..' or '..='
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    inclusive: bool,
}

impl Range {
    /// Exclusive range
    #[inline]
    pub const fn excl() -> Self {
        Self { inclusive: false }
    }

    /// Inclusive range
    #[inline]
    pub const fn incl() -> Self {
        Self { inclusive: true }
    }
}

impl fmt::Display for Range {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = if self.inclusive { "..=" } else { ".." };
        f.write_str(s)
    }
}

/// Declare a variable/constant
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum VarType {
    /// Variable
    #[strum(serialize = "var")]
    Var,

    /// Let
    #[strum(serialize = "let")]
    Let,

    /// Static variable
    #[strum(serialize = "static")]
    Static,

    /// Constant
    #[strum(serialize = "const")]
    Const,
}
impl_display!(VarType, str);

/// Type definition keyword 'struct', 'enum', 'union'
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::IntoStaticStr)]
pub enum TypeDef {
    /// Structure
    #[strum(serialize = "struct")]
    Struct,

    /// Enumeration
    #[strum(serialize = "enum")]
    Enum,

    /// Memory Union
    #[strum(serialize = "union")]
    Union,
}
impl_display!(TypeDef, str);

// MARK: LITERAL

/// Literal
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// Boolean
    Bool(BoolLit),

    /// Character
    Char(CharLit),

    /// Integer
    Int(IntLit),

    /// Floating point number
    Float(FloatLit),

    /// String
    Str(StrLit),
}

/// Boolean literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoolLit(pub bool);
impl_display!(BoolLit);

/// Character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLit(pub char);
impl_display!(CharLit, "'{}'");

/// Integer literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntLit(pub u128);
impl_display!(IntLit);

/// Floating point literal
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatLit(pub f64);
impl_display!(FloatLit);

/// String literal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit(pub Box<str>);
impl_display!(StrLit, r#""{}""#);

/// Label identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub Box<str>);
impl_display!(Label, "'{}");

// MARK: IDENTIFIER

/// Identifier for register, variable, function, type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub Box<str>);
impl_display!(Ident);

impl Ident {
    /// Instantiate a new identifier given its name
    #[inline]
    pub fn new(ident: &str) -> Self {
        Self(ident.to_owned().into_boxed_str())
    }
}
