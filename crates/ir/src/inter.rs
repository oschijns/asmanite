/// Define generic op-codes
pub mod opcode;

/// Absolute Address
pub type Address = u64;

/// Relative address
pub type Relative = i64;

/// Type used for immediate values
pub type Int = i64;

/// Program to transform into CPU target instructions
pub struct IrProgram {
    /// Functions to convert
    functions: Vec<IrFunc>,
}

/// Define a function which should be placed somewhere in the ROM
pub struct IrFunc {
    // TODO:
    // - define parameters
    // - instructions
    // - return values
}
