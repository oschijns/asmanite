//! Instruction set of the NMOS 6502 (official opcodes only) and its encoding

use crate::backend::{
    BackendError,
    link::{Addr, Instruction, Symbol, SymbolTable},
};
use bytes::{BufMut, BytesMut};
use core::fmt;

type Result<T> = core::result::Result<T, BackendError>;

/// Opcode of `JMP absolute`, used by long branches
const JMP_ABSOLUTE: u8 = 0x4C;

/// Instruction names
#[rustfmt::skip]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mnemonic {
    Adc, And, Asl, Bcc, Bcs, Beq, Bit, Bmi, Bne, Bpl, Brk, Bvc, Bvs, Clc,
    Cld, Cli, Clv, Cmp, Cpx, Cpy, Dec, Dex, Dey, Eor, Inc, Inx, Iny, Jmp,
    Jsr, Lda, Ldx, Ldy, Lsr, Nop, Ora, Pha, Php, Pla, Plp, Rol, Ror, Rti,
    Rts, Sbc, Sec, Sed, Sei, Sta, Stx, Sty, Tax, Tay, Tsx, Txa, Txs, Tya,
}

impl Mnemonic {
    /// Every mnemonic
    #[rustfmt::skip]
    pub const ALL: [Self; 56] = {
        use Mnemonic::*;
        [
            Adc, And, Asl, Bcc, Bcs, Beq, Bit, Bmi, Bne, Bpl, Brk, Bvc, Bvs, Clc,
            Cld, Cli, Clv, Cmp, Cpx, Cpy, Dec, Dex, Dey, Eor, Inc, Inx, Iny, Jmp,
            Jsr, Lda, Ldx, Ldy, Lsr, Nop, Ora, Pha, Php, Pla, Plp, Rol, Ror, Rti,
            Rts, Sbc, Sec, Sed, Sei, Sta, Stx, Sty, Tax, Tay, Tsx, Txa, Txs, Tya,
        ]
    };

    /// Branch taken on the opposite condition
    pub fn inverse_branch(self) -> Option<Self> {
        use Mnemonic::*;
        let inverse = match self {
            Bcc => Bcs,
            Bcs => Bcc,
            Beq => Bne,
            Bne => Beq,
            Bmi => Bpl,
            Bpl => Bmi,
            Bvc => Bvs,
            Bvs => Bvc,
            _ => return None,
        };
        Some(inverse)
    }
}

impl fmt::Display for Mnemonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{self:?}").to_lowercase())
    }
}

/// Addressing modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    /// `rts`
    Implied,
    /// `asl a`
    Accumulator,
    /// `lda #$nn`
    Immediate,
    /// `lda $nn`
    ZeroPage,
    /// `lda $nn,x`
    ZeroPageX,
    /// `ldx $nn,y`
    ZeroPageY,
    /// `lda $nnnn`
    Absolute,
    /// `lda $nnnn,x`
    AbsoluteX,
    /// `lda $nnnn,y`
    AbsoluteY,
    /// `jmp ($nnnn)`
    Indirect,
    /// `lda ($nn,x)`
    IndirectX,
    /// `lda ($nn),y`
    IndirectY,
    /// `bne label`
    Relative,
}

impl Mode {
    /// Every addressing mode
    pub const ALL: [Self; 13] = {
        use Mode::*;
        [
            Implied,
            Accumulator,
            Immediate,
            ZeroPage,
            ZeroPageX,
            ZeroPageY,
            Absolute,
            AbsoluteX,
            AbsoluteY,
            Indirect,
            IndirectX,
            IndirectY,
            Relative,
        ]
    };

    /// Number of bytes following the opcode
    #[rustfmt::skip]
    pub const fn operand_len(self) -> usize {
        use Mode::*;
        match self {
            Implied   | Accumulator => 0,
            Immediate | ZeroPage  | ZeroPageX | ZeroPageY | IndirectX | IndirectY | Relative => 1,
            Absolute  | AbsoluteX | AbsoluteY | Indirect => 2,
        }
    }
}

/// Opcode of an instruction, if the addressing mode exists for it
pub fn opcode(mnemonic: Mnemonic, mode: Mode) -> Option<u8> {
    use Mnemonic::*;
    use Mode::*;

    /// ORA, AND, EOR, ADC, STA, LDA, CMP, SBC share the same layout:
    /// the addressing mode is encoded in bits 2-4.
    #[rustfmt::skip]
    fn group1(mnemonic: Mnemonic, mode: Mode) -> Option<u8> {
        let base = match mnemonic {
            Ora => 0x01, And => 0x21, Eor => 0x41, Adc => 0x61,
            Sta => 0x81, Lda => 0xA1, Cmp => 0xC1, Sbc => 0xE1,
            _ => return None,
        };
        let offset = match mode {
            IndirectX                     => 0x00,
            ZeroPage                      => 0x04,
            Immediate if mnemonic != Sta  => 0x08,
            Absolute                      => 0x0C,
            IndirectY                     => 0x10,
            ZeroPageX                     => 0x14,
            AbsoluteY                     => 0x18,
            AbsoluteX                     => 0x1C,
            _ => return None,
        };
        Some(base + offset)
    }

    /// Other op-codes
    #[rustfmt::skip]
    fn group2(mnemonic: Mnemonic, mode: Mode) -> Option<u8> {
        let code = match (mnemonic, mode) {
            // Read-modify-write
            (Asl, Accumulator) => 0x0A, (Asl, ZeroPage ) => 0x06, (Asl, ZeroPageX) => 0x16, (Asl, Absolute ) => 0x0E, (Asl, AbsoluteX) => 0x1E,
            (Rol, Accumulator) => 0x2A, (Rol, ZeroPage ) => 0x26, (Rol, ZeroPageX) => 0x36, (Rol, Absolute ) => 0x2E, (Rol, AbsoluteX) => 0x3E,
            (Lsr, Accumulator) => 0x4A, (Lsr, ZeroPage ) => 0x46, (Lsr, ZeroPageX) => 0x56, (Lsr, Absolute ) => 0x4E, (Lsr, AbsoluteX) => 0x5E,
            (Ror, Accumulator) => 0x6A, (Ror, ZeroPage ) => 0x66, (Ror, ZeroPageX) => 0x76, (Ror, Absolute ) => 0x6E, (Ror, AbsoluteX) => 0x7E,
            (Dec, ZeroPage   ) => 0xC6, (Dec, ZeroPageX) => 0xD6, (Dec, Absolute ) => 0xCE, (Dec, AbsoluteX) => 0xDE,
            (Inc, ZeroPage   ) => 0xE6, (Inc, ZeroPageX) => 0xF6, (Inc, Absolute ) => 0xEE, (Inc, AbsoluteX) => 0xFE,

            // Index registers
            (Ldx, Immediate) => 0xA2, (Ldx, ZeroPage ) => 0xA6, (Ldx, ZeroPageY) => 0xB6, (Ldx, Absolute) => 0xAE, (Ldx, AbsoluteY) => 0xBE,
            (Ldy, Immediate) => 0xA0, (Ldy, ZeroPage ) => 0xA4, (Ldy, ZeroPageX) => 0xB4, (Ldy, Absolute) => 0xAC, (Ldy, AbsoluteX) => 0xBC,
            (Stx, ZeroPage ) => 0x86, (Stx, ZeroPageY) => 0x96, (Stx, Absolute ) => 0x8E,
            (Sty, ZeroPage ) => 0x84, (Sty, ZeroPageX) => 0x94, (Sty, Absolute ) => 0x8C,
            (Cpx, Immediate) => 0xE0, (Cpx, ZeroPage ) => 0xE4, (Cpx, Absolute ) => 0xEC,
            (Cpy, Immediate) => 0xC0, (Cpy, ZeroPage ) => 0xC4, (Cpy, Absolute ) => 0xCC,
            (Bit, ZeroPage ) => 0x24, (Bit, Absolute ) => 0x2C,

            // Control flow
            (Jmp, Absolute) => 0x4C, (Jmp, Indirect) => 0x6C, (Jsr, Absolute) => 0x20,
            (Bpl, Relative) => 0x10, (Bmi, Relative) => 0x30, (Bvc, Relative) => 0x50, (Bvs, Relative) => 0x70,
            (Bcc, Relative) => 0x90, (Bcs, Relative) => 0xB0, (Bne, Relative) => 0xD0, (Beq, Relative) => 0xF0,

            // Implied
            (Brk, Implied) => 0x00, (Php, Implied) => 0x08, (Clc, Implied) => 0x18, (Plp, Implied) => 0x28,
            (Sec, Implied) => 0x38, (Rti, Implied) => 0x40, (Pha, Implied) => 0x48, (Cli, Implied) => 0x58,
            (Rts, Implied) => 0x60, (Pla, Implied) => 0x68, (Sei, Implied) => 0x78, (Dey, Implied) => 0x88,
            (Txa, Implied) => 0x8A, (Tya, Implied) => 0x98, (Txs, Implied) => 0x9A, (Tay, Implied) => 0xA8,
            (Tax, Implied) => 0xAA, (Clv, Implied) => 0xB8, (Tsx, Implied) => 0xBA, (Iny, Implied) => 0xC8,
            (Dex, Implied) => 0xCA, (Cld, Implied) => 0xD8, (Inx, Implied) => 0xE8, (Nop, Implied) => 0xEA,
            (Sed, Implied) => 0xF8,

            _ => return None,
        };
        Some(code)
    }

    group1(mnemonic, mode).or_else(|| group2(mnemonic, mode))
}

// MARK: Operands

/// Destination of a jump: a symbol resolved at link time, or a fixed address
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    Symbol(Symbol),
    Addr(u16),
}

impl Target {
    /// Address, if already known
    fn get(&self, symbols: &SymbolTable) -> Option<Addr> {
        match self {
            Self::Symbol(symbol) => symbols.get(symbol),
            Self::Addr(addr) => Some(Addr::from(*addr)),
        }
    }

    /// 16-bit address
    fn resolve(&self, symbols: &SymbolTable) -> Result<u16> {
        match self {
            Self::Addr(addr) => Ok(*addr),
            Self::Symbol(symbol) => {
                let addr = symbols.resolve(symbol)?;
                u16::try_from(addr).map_err(|_| BackendError::AddressRange {
                    symbol: symbol.clone(),
                    addr,
                })
            }
        }
    }
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Symbol(symbol) => symbol.fmt(f),
            Self::Addr(addr) => write!(f, "${addr:04X}"),
        }
    }
}

/// Bytes following the opcode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Arg {
    /// No operand
    None,

    /// Immediate value or zero-page address
    Byte(u8),

    /// Absolute address
    Word(Target),
}

/// A single 6502 instruction
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inst {
    pub mnemonic: Mnemonic,
    pub mode: Mode,
    pub arg: Arg,
}

impl Inst {
    /// New instruction, if the mode exists for the mnemonic and the argument matches the mode
    pub fn new(mnemonic: Mnemonic, mode: Mode, arg: Arg) -> Option<Self> {
        opcode(mnemonic, mode)?;
        let len = match arg {
            Arg::None => 0,
            Arg::Byte(_) => 1,
            Arg::Word(_) => 2,
        };
        (len == mode.operand_len() && mode != Mode::Relative).then_some(Self {
            mnemonic,
            mode,
            arg,
        })
    }
}

impl fmt::Display for Inst {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Mode::*;
        write!(f, "{}", self.mnemonic)?;
        match (&self.arg, self.mode) {
            (Arg::None, Accumulator ) => write!(f, " a"),
            (Arg::None, _           ) => Ok(()),
            (Arg::Byte(b), Immediate) => write!(f, " #${b:02X}"),
            (Arg::Byte(b), ZeroPageX) => write!(f, " ${b:02X},x"),
            (Arg::Byte(b), ZeroPageY) => write!(f, " ${b:02X},y"),
            (Arg::Byte(b), IndirectX) => write!(f, " (${b:02X},x)"),
            (Arg::Byte(b), IndirectY) => write!(f, " (${b:02X}),y"),
            (Arg::Byte(b), _        ) => write!(f, " ${b:02X}"),
            (Arg::Word(t), AbsoluteX) => write!(f, " {t},x"),
            (Arg::Word(t), AbsoluteY) => write!(f, " {t},y"),
            (Arg::Word(t), Indirect ) => write!(f, " ({t})"),
            (Arg::Word(t), _        ) => write!(f, " {t}"),
        }
    }
}

// MARK: Layout

/// Element of a 6502 section
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    /// Instruction with a fixed size
    Inst(Inst),

    /// Conditional branch.
    /// If the target is too far for a relative branch (-128..=127),
    /// it becomes the opposite branch skipping a `JMP target`.
    Branch {
        mnemonic: Mnemonic,
        target: Target,
        long: bool,
    },
}

/// Can a branch at `pc` reach `target` ?
fn branch_reaches(pc: Addr, target: Addr) -> bool {
    let displacement = target as i64 - (pc as i64 + 2);
    (-128..=127).contains(&displacement)
}

impl Instruction for Op {
    fn size(&mut self, pc: Addr, symbols: &SymbolTable) -> usize {
        match self {
            Self::Inst(inst) => 1 + inst.mode.operand_len(),
            Self::Branch { target, long, .. } => {
                // Unknown targets are assumed close: they are checked again once known
                if !*long && target.get(symbols).is_some_and(|t| !branch_reaches(pc, t)) {
                    *long = true;
                }
                if *long { 5 } else { 2 }
            }
        }
    }

    fn encode(&self, pc: Addr, symbols: &SymbolTable, out: &mut BytesMut) -> Result<()> {
        let missing = |mnemonic, mode| {
            BackendError::Internal(format!("no opcode for `{mnemonic}` in mode {mode:?}"))
        };
        match self {
            Self::Inst(inst) => {
                let code = opcode(inst.mnemonic, inst.mode)
                    .ok_or_else(|| missing(inst.mnemonic, inst.mode))?;
                out.put_u8(code);
                match &inst.arg {
                    Arg::None => {}
                    Arg::Byte(byte) => out.put_u8(*byte),
                    Arg::Word(target) => out.put_u16_le(target.resolve(symbols)?),
                }
            }
            Self::Branch {
                mnemonic,
                target,
                long,
            } => {
                let addr = target.resolve(symbols)?;
                if *long {
                    let inverse = mnemonic
                        .inverse_branch()
                        .ok_or_else(|| missing(*mnemonic, Mode::Relative))?;
                    let code = opcode(inverse, Mode::Relative)
                        .ok_or_else(|| missing(inverse, Mode::Relative))?;
                    // skip the 3 bytes of the JMP
                    out.put_slice(&[code, 3, JMP_ABSOLUTE]);
                    out.put_u16_le(addr);
                } else {
                    let code = opcode(*mnemonic, Mode::Relative)
                        .ok_or_else(|| missing(*mnemonic, Mode::Relative))?;
                    if !branch_reaches(pc, Addr::from(addr)) {
                        return Err(BackendError::Internal(format!(
                            "short branch to `{target}` out of range"
                        )));
                    }
                    let displacement = i64::from(addr) - (pc as i64 + 2);
                    out.put_u8(code);
                    out.put_i8(displacement as i8);
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inst(inst) => inst.fmt(f),
            Self::Branch {
                mnemonic,
                target,
                long: false,
            } => write!(f, "{mnemonic} {target}"),
            Self::Branch {
                mnemonic,
                target,
                long: true,
            } => match mnemonic.inverse_branch() {
                Some(inverse) => write!(f, "{inverse} *+5 / jmp {target}"),
                None => write!(f, "{mnemonic} {target} (invalid)"),
            },
        }
    }
}
