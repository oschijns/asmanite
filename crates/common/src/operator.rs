use crate::number::{Number, Promoted};
use core::fmt::{self, Write as _};
use num_traits::{
    CheckedDiv, CheckedRem, PrimInt, WrappingAdd, WrappingMul, WrappingNeg, WrappingShl,
    WrappingShr, WrappingSub,
};

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
            Self::Not    | // !
            Self::BitInv | // ~
            Self::BitAnd | // &
            Self::Inc    | // ++
            Self::Dec    | // --
            Self::Add    | // +
            Self::Sub    | // -
            Self::Mul      // *
        )
    }

    /// Is the operator a comparison ?
    pub fn is_comparison(self) -> bool {
        matches!(
            self,
            Self::Eq | Self::Diff | Self::Lt | Self::Gt | Self::LEq | Self::GEq
        )
    }

    /// Inverse a comparison operator
    #[rustfmt::skip]
    pub fn inverse_comparison(self) -> Option<Self> {
        let inv = match self {
            Self::Eq   => Self::Diff,
            Self::Diff => Self::Eq,
            Self::Lt   => Self::GEq,
            Self::Gt   => Self::LEq,
            Self::LEq  => Self::Gt,
            Self::GEq  => Self::Lt,
            Self::And  => Self::Or,
            Self::Or   => Self::And,
            _ => { return None; }
        };
        Some(inv)
    }

    /// Evaluate the comparison operator (for compile-time expressions)
    #[rustfmt::skip]
    pub fn eval_compare<I>(self, lhs: I, rhs: I) -> Option<bool>
    where
        I: PartialOrd<I>,
    {
        let result = match self {
            Self::Eq   => lhs == rhs,
            Self::Diff => lhs != rhs,
            Self::Lt   => lhs <  rhs,
            Self::Gt   => lhs >  rhs,
            Self::LEq  => lhs <= rhs,
            Self::GEq  => lhs >= rhs,
            _ => { return None; }
        };
        Some(result)
    }

    /// Evaluate the boolean operator (for compile-time expressions)
    #[rustfmt::skip]
    pub fn eval_boolean(self, lhs: bool, rhs: bool) -> Option<bool> {
        let result = match self {
            Self::And => lhs && rhs,
            Self::Or  => lhs || rhs,
            _ => { return None; }
        };
        Some(result)
    }

    /// Evaluate the binary operator (for compile-time expressions).
    /// Returns `None` on a division by zero.
    #[rustfmt::skip]
    pub fn eval_binary<I>(self, lhs: I, rhs: I) -> Option<I>
    where
        I: PrimInt + WrappingAdd + WrappingSub + WrappingMul + CheckedDiv + CheckedRem,
    {
        let result = match self {
            Self::BitAnd => lhs & rhs,
            Self::BitOr  => lhs | rhs,
            Self::BitXor => lhs ^ rhs,
            Self::Add    => lhs.wrapping_add(&rhs),
            Self::Sub    => lhs.wrapping_sub(&rhs),
            Self::Mul    => lhs.wrapping_mul(&rhs),
            Self::Div    => lhs.checked_div (&rhs)?,
            Self::Rem    => lhs.checked_rem (&rhs)?,
            _ => { return None; }
        };
        Some(result)
    }

    /// Evaluate the bitshift and rotation operators (for compile-time expressions).
    ///
    /// The width of `I` is the width of the rotated value:
    /// `0x81_u8 <+ 1 == 0x03` but `0x81_u16 <+ 1 == 0x0102`.
    /// Like the wrapping shifts, the amount is taken modulo the width of `I`.
    #[rustfmt::skip]
    pub fn eval_bitshift<I>(self, lhs: I, rhs: u32) -> Option<I>
    where
        I: PrimInt + WrappingShl + WrappingShr,
    {
        let bits = I::zero().count_zeros();
        let result = match self {
            Self::ShL  => lhs.wrapping_shl(rhs),
            Self::ShRA => lhs.signed_shr  (rhs % bits),
            Self::ShRL => lhs.unsigned_shr(rhs % bits),
            Self::RotL => lhs.rotate_left (rhs % bits),
            Self::RotR => lhs.rotate_right(rhs % bits),
            _ => { return None; }
        };
        Some(result)
    }

    /// Evaluate the unary operator (for compile-time expressions)
    #[rustfmt::skip]
    pub fn eval_unnary<I>(self, op: I) -> Option<I>
    where
        I: PrimInt + WrappingNeg + WrappingAdd + WrappingSub,
    {
        let result = match self {
            Self::Add    => op,
            Self::BitInv => !op,
            Self::Sub    => op.wrapping_neg(),
            Self::Inc    => op.wrapping_add(&I::one()),
            Self::Dec    => op.wrapping_sub(&I::one()),
            _ => { return None; }
        };
        Some(result)
    }
}

/// Evaluation of operators on [`Number`]s (for compile-time expressions)
impl OpType {
    /// Evaluate the arithmetic operator on floating point numbers.
    /// Returns `None` for other operators and divisions by zero.
    #[rustfmt::skip]
    pub fn eval_float(self, lhs: f64, rhs: f64) -> Option<f64> {
        let result = match self {
            Self::Add => lhs + rhs,
            Self::Sub => lhs - rhs,
            Self::Mul => lhs * rhs,
            Self::Div if rhs != 0.0 => lhs / rhs,
            Self::Rem if rhs != 0.0 => lhs % rhs,
            _ => { return None; }
        };
        Some(result)
    }

    /// Evaluate the binary operator on numbers.
    /// Comparisons and boolean operators give `0` or `1`.
    pub fn eval_binary_num(self, lhs: Number, rhs: Number) -> Option<Number> {
        let promoted = lhs.promote(rhs);
        let compare = match promoted {
            Promoted::UInt(lhs, rhs) => self.eval_compare(lhs, rhs),
            Promoted::Int(lhs, rhs) => self.eval_compare(lhs, rhs),
            Promoted::Float(lhs, rhs) => self.eval_compare(lhs, rhs),
        };
        if let Some(result) = compare.or_else(|| self.eval_boolean(lhs.is_true(), rhs.is_true())) {
            return Some(Number::from(result));
        }
        match self {
            // TODO: rotations depend on the width of the value,
            // which requires tracking the primitive type of the constants
            Self::RotL | Self::RotR => None,
            Self::ShL | Self::ShRA | Self::ShRL => {
                let amount = match rhs {
                    Number::Float(_) => return None,
                    _ => u32::try_from(rhs).ok()?,
                };
                match lhs {
                    Number::UInt(lhs) => self.eval_bitshift(lhs, amount).map(Number::from),
                    Number::Int(lhs) => self.eval_bitshift(lhs, amount).map(Number::from),
                    Number::Float(_) => None,
                }
            }
            _ => match promoted {
                // `1 - 2` is negative: switch to signed integers
                Promoted::UInt(lhs, rhs) if self == Self::Sub && lhs < rhs => {
                    Some(Number::from((lhs as i64).wrapping_sub(rhs as i64)))
                }
                Promoted::UInt(lhs, rhs) => self.eval_binary(lhs, rhs).map(Number::from),
                Promoted::Int(lhs, rhs) => self.eval_binary(lhs, rhs).map(Number::from),
                Promoted::Float(lhs, rhs) => self.eval_float(lhs, rhs).map(Number::from),
            },
        }
    }

    /// Evaluate the unary operator on a number.
    /// Increments and decrements modify a place: they are not evaluated.
    pub fn eval_unary_num(self, value: Number) -> Option<Number> {
        match (self, value) {
            (Self::Not, _) => Some(Number::from(!value.is_true())),
            (Self::Inc | Self::Dec, _) => None,
            // Negating an unsigned integer gives a signed one
            (Self::Sub, Number::UInt(uint)) => Some(Number::from((uint as i64).wrapping_neg())),
            (_, Number::UInt(uint)) => self.eval_unnary(uint).map(Number::from),
            (_, Number::Int(int)) => self.eval_unnary(int).map(Number::from),
            (Self::Add, Number::Float(_)) => Some(value),
            (Self::Sub, Number::Float(float)) => Some(Number::from(-float)),
            (_, Number::Float(_)) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OpType;
    use crate::number::Number;

    #[test]
    fn rotates_within_the_width_of_the_type() {
        assert_eq!(OpType::RotL.eval_bitshift(0x81_u8, 1), Some(0x03));
        assert_eq!(OpType::RotR.eval_bitshift(0x81_u8, 1), Some(0xC0));
        assert_eq!(OpType::RotL.eval_bitshift(0x8001_u16, 4), Some(0x0018));
        assert_eq!(OpType::RotR.eval_bitshift(0x8001_u16, 4), Some(0x1800));
        assert_eq!(
            OpType::RotL.eval_bitshift(0x8000_0001_u32, 1),
            Some(0x0000_0003)
        );
        assert_eq!(
            OpType::RotR.eval_bitshift(0x8000_0001_u32, 1),
            Some(0xC000_0000)
        );
        assert_eq!(OpType::RotL.eval_bitshift(1_u64 << 63, 1), Some(1));
        assert_eq!(OpType::RotR.eval_bitshift(1_u64, 1), Some(1 << 63));
        assert_eq!(
            OpType::RotL.eval_bitshift(1_usize << (usize::BITS - 1), 1),
            Some(1)
        );
        assert_eq!(
            OpType::RotR.eval_bitshift(1_usize, 1),
            Some(1 << (usize::BITS - 1))
        );
    }

    #[test]
    fn rotation_amount_wraps_around_the_width() {
        assert_eq!(OpType::RotL.eval_bitshift(0x81_u8, 8), Some(0x81));
        assert_eq!(OpType::RotL.eval_bitshift(0x81_u8, 9), Some(0x03));
        assert_eq!(OpType::RotR.eval_bitshift(0x8001_u16, 20), Some(0x1800));
        assert_eq!(OpType::RotL.eval_bitshift(0x1234_u16, 0), Some(0x1234));
    }

    #[test]
    fn rotations_are_reversible() {
        for amount in 0..70 {
            let left = OpType::RotL.eval_bitshift(0xDEAD_BEEF_u32, amount).unwrap();
            assert_eq!(OpType::RotR.eval_bitshift(left, amount), Some(0xDEAD_BEEF));
        }
    }

    #[test]
    fn distinguishes_arithmetic_and_logical_shifts() {
        assert_eq!(OpType::ShRA.eval_bitshift(-128_i8, 1), Some(-64));
        assert_eq!(OpType::ShRL.eval_bitshift(-128_i8, 1), Some(64));
        assert_eq!(OpType::ShRL.eval_bitshift(0x80_u8, 7), Some(1));
    }

    #[test]
    fn rejects_non_shift_operators() {
        assert_eq!(OpType::Add.eval_bitshift(1_u8, 1), None);
        assert_eq!(OpType::Div.eval_binary(1_u8, 0), None);
    }

    #[test]
    fn evaluates_numbers() {
        let (u, i, f) = (Number::UInt, Number::Int, Number::Float);
        assert_eq!(OpType::Add.eval_binary_num(u(1), u(2)), Some(u(3)));
        assert_eq!(OpType::Sub.eval_binary_num(u(1), u(2)), Some(i(-1)));
        assert_eq!(OpType::Mul.eval_binary_num(i(-2), u(3)), Some(i(-6)));
        assert_eq!(OpType::Div.eval_binary_num(f(1.0), u(2)), Some(f(0.5)));
        assert_eq!(OpType::Div.eval_binary_num(u(1), u(0)), None);
        assert_eq!(OpType::BitAnd.eval_binary_num(f(1.0), u(1)), None);
        assert_eq!(OpType::ShL.eval_binary_num(u(1), u(4)), Some(u(16)));
        assert_eq!(OpType::ShL.eval_binary_num(u(1), f(4.0)), None);
        assert_eq!(OpType::RotL.eval_binary_num(u(1), u(1)), None);
    }

    #[test]
    fn comparisons_and_booleans_give_zero_or_one() {
        let (u, i, f) = (Number::UInt, Number::Int, Number::Float);
        assert_eq!(OpType::Lt.eval_binary_num(i(-1), u(0)), Some(u(1)));
        assert_eq!(OpType::Eq.eval_binary_num(f(2.0), u(2)), Some(u(1)));
        assert_eq!(OpType::And.eval_binary_num(u(2), u(0)), Some(u(0)));
        assert_eq!(OpType::Or.eval_binary_num(u(0), i(-3)), Some(u(1)));
    }

    #[test]
    fn evaluates_unary_numbers() {
        let (u, i, f) = (Number::UInt, Number::Int, Number::Float);
        assert_eq!(OpType::Sub.eval_unary_num(u(5)), Some(i(-5)));
        assert_eq!(OpType::Sub.eval_unary_num(f(1.5)), Some(f(-1.5)));
        assert_eq!(OpType::Not.eval_unary_num(u(0)), Some(u(1)));
        assert_eq!(OpType::BitInv.eval_unary_num(u(0)), Some(u(u64::MAX)));
        assert_eq!(OpType::BitInv.eval_unary_num(f(0.0)), None);
        assert_eq!(OpType::Inc.eval_unary_num(u(0)), None);
    }
}
