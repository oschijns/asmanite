use core::fmt;
use std::fmt::Write as _;

// MARK: ASSIGN

#[rustfmt::skip]
pub mod assign {
    use super::{Assign, OpType};

    pub const ASSIGN : Assign = Assign::new();
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
pub mod op {
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

/// Display the operator
impl fmt::Display for OpType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.into())
    }
}

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
}
