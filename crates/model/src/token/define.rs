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
    pub fn new(prim_type: PrimType, size: u8) -> Self {
        Self { prim_type, size }
    }

    /// Define a primitive with a size in bits
    #[inline]
    pub fn new_bits(prim_type: PrimType, bitsize: u8) -> Self {
        Self {
            prim_type,
            size: bitsize >> 8,
        }
    }
}

/// Type of primitive values
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimType {
    /// Unsigned integer
    UInt,

    /// Signed integer
    Int,

    /// Floating point number
    Float,
}

/// Assignment instruction
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Assign {
    /// Optional operator while doing the assignment
    pub op: Option<Operator>,
}

impl Assign {
    /// Assignment
    #[inline]
    pub fn new() -> Self {
        Self { op: None }
    }

    /// New operator
    #[inline]
    pub fn op(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::new(op_type)),
        }
    }

    /// New operator with carry
    #[inline]
    pub fn op_carry(op_type: OpType) -> Self {
        Self {
            op: Some(Operator::carry(op_type)),
        }
    }
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
    pub fn new(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: false,
        }
    }

    /// New operator with carry
    #[inline]
    pub fn carry(op_type: OpType) -> Self {
        Self {
            op_type,
            carry: true,
        }
    }
}

/// Type of operator
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpType {
    /// Equality '=='
    Eq,

    /// Difference '!='
    Diff,

    /// Lesser than '<'
    Lt,

    /// Greater than '>'
    Gt,

    /// Lesser or equal than '<='
    LEq,

    /// Greater or equal than '>='
    GEq,

    /// Logical NOT '!'
    Not,

    /// Logical AND '&&'
    And,

    /// Logical OR '||'
    Or,

    /// Bitwise INV '~'
    BitInv,

    /// Bitwise AND '&'
    BitAnd,

    /// Bitwise OR '|'
    BitOr,

    /// Bitwise XOR '^'
    BitXor,

    /// Shift Left '<<'
    ShL,

    /// Shift Right (arithmetic) '>>'
    ShRA,

    /// Shift Right (logical) '>>>'
    ShRL,

    /// Rotate bits Left '<+'
    RotL,

    /// Rotate bits Right '+>'
    RotR,

    /// Increment '++'
    Inc,

    /// Decrement '--'
    Dec,

    /// Addition '+'
    Add,

    /// Subtraction '-'
    Sub,

    /// Multiplication '*'
    Mul,

    /// Division '/'
    Div,

    /// Remainder '%'
    Rem,
}

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
    pub fn open(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: false,
        }
    }

    /// Create a new close delimiter
    #[inline]
    pub fn close(delim_type: DelimType) -> Self {
        Self {
            delim_type,
            is_close: true,
        }
    }
}

/// Type of delimiter
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimType {
    /// Parenthesis '(', ')'
    Paren,

    /// Brace '{', '}'
    Brace,

    /// Bracket '[', ']'
    Bracket,
}

/// Range '..' or '..='
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    inclusive: bool,
}

impl Range {
    /// Exclusive range
    #[inline]
    pub fn excl() -> Self {
        Self { inclusive: false }
    }

    /// Inclusive range
    #[inline]
    pub fn incl() -> Self {
        Self { inclusive: true }
    }
}

/// Declare a variable/constant
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarDecl {
    /// Variable
    Var,

    /// Let
    Let,

    /// Static variable
    Static,

    /// Constant
    Const,
}

/// Boolean literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoolLit(pub bool);

/// Character literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharLit(pub char);

/// Integer literal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntLit(pub u128);

/// Floating point literal
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatLit(pub f64);

/// String literal
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrLit(pub Box<str>);

/// Identifier for register, variable, function, type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ident(pub Box<str>);

/// Label identifier
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label(pub Box<str>);
