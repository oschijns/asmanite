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
#[derive(Debug, Default, Clone, PartialEq)]
pub struct IrProgram {
    /// Memory cells to reserve (global variables, static variables)
    pub data: Vec<IrData>,

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

    /// Value of the memory cell when the program starts, if any
    pub init: Option<Int>,
}

/// Placement of a memory cell or a function
#[derive(Debug, Default, Clone, PartialEq)]
pub enum IrStorage {
    /// Let the backend decide where to place it
    #[default]
    Auto,

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

    /// Where to place the code of the function
    pub storage: IrStorage,

    /// Location of each parameter, in declaration order.
    /// The caller writes the arguments in these locations before the call.
    pub params: Vec<Operand>,

    /// Location of the return value, if any.
    /// The callee writes its result there before returning.
    pub ret: Option<Operand>,

    /// Memory cells owned by the function (memory parameters, return value, locals)
    pub locals: Vec<IrData>,

    /// Number of temporaries (`Operand::Temp`) used in the body
    pub temps: u32,

    /// Instructions of the function
    pub body: Vec<OpCode>,
}
