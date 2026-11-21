use crate::inter::{Address, Int, Relative};
use asmanite_common::{
    ToBox, operator::Operator, path::Path, register::RegIdent, scope::ScopeId, status::StatusFlag,
};

/// Define a generic instruction
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Branching operation
    Branch(Box<OpBranch>),

    /// Transfer data between two operands
    Transfer(Box<OpTransfer>),

    /// Apply operation to operand
    Unary(Box<OpUnary>),

    /// Apply operation between two operand
    Binary(Box<OpBinary>),
}

/// Define a branching instruction
#[derive(Debug, Clone, PartialEq)]
pub struct OpBranch {
    /// Condition to branch on
    pub cond: Condition,

    /// Target label to branch to
    pub target: Path,
}

/// Condition in a branching operation
#[derive(Debug, Clone, PartialEq)]
pub enum Condition {
    /// Condition on a CPU status flag
    Status {
        /// Flag to check
        flag: StatusFlag,

        /// Should the condition be inversed ?
        inverse: bool,
    },

    /// Condition based on an expression
    Expr(Box<OpBinary>),
}

/// Define the target of a jump or branch instruction
#[derive(Debug, Clone, PartialEq)]
pub enum JumpTarget {
    /// Reference to a named label.
    Scope(ScopeId),

    /// Label
    Label(Path),

    /// Reference to a known absolute address.
    Address(Address),

    /// Relative displacement from the current instruction.
    Relative(Relative),

    /// Address held in a register or memory location.
    Indirect(Box<Operand>),
}

/// Transfer data from one memory operand to an other
#[derive(Debug, Clone, PartialEq)]
pub struct OpTransfer {
    /// Move data to destination
    pub dst: Operand,

    /// Move data from source
    pub src: Operand,
}

impl ToBox for OpTransfer {}

/// Unary operation on an operand
#[derive(Debug, Clone, PartialEq)]
pub struct OpUnary {
    /// Operation
    pub op: Operator,

    /// Destination operand
    pub dst: Operand,

    /// Operand
    pub src: Operand,
}

impl ToBox for OpUnary {}

/// Binary operation on two operands
#[derive(Debug, Clone, PartialEq)]
pub struct OpBinary {
    /// Operation
    pub op: Operator,

    /// Destination operand
    pub dst: Operand,

    /// Operand
    pub lhs: Operand,

    /// Operand
    pub rhs: Operand,
}

impl ToBox for OpBinary {}

// MARK: Memory

/// Define a type of memory location
#[derive(Debug, Clone, PartialEq)]
pub enum Operand {
    /// Immediate value
    Imm(Int),

    /// CPU register
    Reg(RegIdent),

    /// Path to a variable stored in RAM
    Mem(Box<MemCell>),

    /// Stack
    Stack,
}

/// Define memory access in RAM
#[derive(Debug, Clone, PartialEq)]
pub struct MemCell {
    /// Name of the variable
    pub var: Path,

    /// Is there a constant offset from the variable ?
    pub offset: i64,

    /// Register used for runtime indexing
    pub index: Option<RegIdent>,
}

impl ToBox for MemCell {}
