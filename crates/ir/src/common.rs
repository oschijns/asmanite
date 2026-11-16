use asmanite_common::{ToBox, operator::Operator, path::Path, register::RegIdent};

/// Type used for immediate values
pub type Int = i64;

// MARK: OP-CODE

/// Define a generic instruction
#[derive(Debug, Clone, PartialEq)]
pub enum OpCode {
    /// Transfer data between two operands
    Transfer(Box<OpTransfer>),

    /// Apply operation to operand
    Unary(Box<OpUnary>),

    /// Apply operation between two operand
    Binary(Box<OpBinary>),
}

/// Transfer data from one memory operand to an other
#[derive(Debug, Clone, PartialEq)]
pub struct OpTransfer {
    /// Move data from source
    pub src: Operand,

    /// Move data to destination
    pub dst: Operand,
}

impl ToBox for OpTransfer {}

/// Unary operation on an operand
#[derive(Debug, Clone, PartialEq)]
pub struct OpUnary {
    /// Operation
    pub op: Operator,

    /// Operand
    pub data: Operand,
}

impl ToBox for OpUnary {}

/// Binary operation on two operands
#[derive(Debug, Clone, PartialEq)]
pub struct OpBinary {
    /// Operation
    pub op: Operator,

    /// Operand
    pub left: Operand,

    /// Operand
    pub right: Operand,
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

    /// Register used for indexing
    pub index: Option<RegIdent>,
}

impl ToBox for MemCell {}
