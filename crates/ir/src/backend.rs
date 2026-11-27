use crate::Addr;
use crate::intermediate::IrProgram;
use crate::link::{Image, Symbol};
use asmanite_common::number::Number;
use asmanite_common::{ident::Ident, path::Path};

/// Convert an IR program into a binary image for a given CPU target
pub trait Backend {
    /// Select instructions, lay them out in memory and encode them
    fn assemble(&self, program: &IrProgram) -> Result<Image, BackendError>;
}

/// Error raised while converting the IR into machine code
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum BackendError {
    #[error("unknown memory bank `{0}`")]
    UnknownBank(Ident),

    #[error("memory bank `{bank}` overflows by {excess} bytes")]
    BankOverflow { bank: Ident, excess: Addr },

    #[error("`{0}` is defined more than once")]
    Duplicate(Symbol),

    #[error("`{0}` is not defined")]
    Undefined(Symbol),

    #[error("the code of `{section}` (at ${addr:04X}) is not in a ROM bank")]
    NotInRom { section: Symbol, addr: Addr },

    #[error("the code of `{0}` and `{1}` overlap")]
    Overlap(Symbol, Symbol),

    #[error("the layout of the code did not converge")]
    NoConvergence,

    #[error("address ${addr:X} of `{symbol}` is out of range for this target")]
    AddressRange { symbol: Symbol, addr: Addr },

    #[error("in `{at}`: {what} is not supported by this target")]
    Unsupported { at: Path, what: String },

    #[error("in `{at}`: {value} does not fit in {what}")]
    OutOfRange {
        at: Path,
        what: &'static str,
        value: Number,
    },

    #[error("internal backend error: {0}")]
    Internal(String),
}
