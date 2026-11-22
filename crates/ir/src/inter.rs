/// Define generic op-codes
pub mod opcode;

use asmanite_common::{ident::Ident, path::Path};
use opcode::{OpCode, Operand};

/// Absolute Address
pub type Address = u64;

/// Relative address
pub type Relative = i64;

/// Type used for immediate values
pub type Int = i64;

/// Program to transform into CPU target instructions
#[derive(Debug, Clone, PartialEq)]
pub struct IrProgram {
    /// Memory cells to reserve that are not owned by a function
    /// (static variables, memory of the inline functions)
    pub data: Vec<IrData>,

    /// Code run by the target system at startup, before anything else.
    /// It computes the initial value of the static variables.
    pub init: IrFunc,

    /// Functions to convert
    pub functions: Vec<IrFunc>,
}

/// Memory reserved for a variable
#[derive(Debug, Clone, PartialEq)]
pub struct IrData {
    /// Absolute path of the variable
    pub name: Path,

    /// Size of the variable in bytes
    pub size: u32,

    /// Where to place the variable
    pub storage: IrStorage,
}

/// Placement of a memory cell or a function.
/// The placement is always explicit: the compiler never chooses it.
#[derive(Debug, Clone, PartialEq)]
pub enum IrStorage {
    /// Startup routine (initialize static variables)
    Startup,

    /// Place it somewhere in the given memory bank
    Bank(Ident),

    /// Place it at a fixed address
    Address(Address),

    /// Place it on the stack
    Stack,
}

/// Define a function which should be placed somewhere in the ROM
#[derive(Debug, Clone, PartialEq)]
pub struct IrFunc {
    /// Absolute path of the function
    pub name: Path,

    /// Where to place the code of the function.
    /// Inline functions do not appear in the IR: they are expanded at each call site.
    pub storage: IrStorage,

    /// Location of each parameter, in declaration order.
    /// The caller writes the arguments in these locations before the call.
    pub params: Vec<Operand>,

    /// Location of the return value, if any.
    /// The callee writes its result there before returning.
    pub ret: Option<Operand>,

    /// Memory cells owned by the function (memory parameters, return value, locals)
    pub locals: Vec<IrData>,

    /// Instructions of the function
    pub body: Vec<OpCode>,
}
