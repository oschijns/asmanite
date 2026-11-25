//! MOS 6502 backend (NMOS, official opcodes only)
//!
//! Supported: 8-bit values, registers `a`, `x`, `y`, variables (with constant
//! offset and `x`/`y` indexing), transfers, `+ - & | ^`, shifts and rotations
//! by a constant, `++`/`--`, negation, bitwise inversion, unsigned comparisons,
//! status-flag branches, jumps (direct and indirect), calls and returns.
//!
//! Not supported yet: values wider than 8 bits, signed comparisons,
//! multiplication / division (they need a runtime routine), variables on the stack.

/// Instruction set and encoding
pub mod isa;

/// Instruction selection
mod select;

use crate::{
    backend::{Backend, BackendError},
    intermediate::IrProgram,
    link::{self, Image, Layout, MemoryMap, Section},
};
use asmanite_common::{primitive::def::U8, register::RegDef};
use isa::Op;

/// Registers of the 6502, to declare with [`registers_init`](asmanite_common::register::registers_init)
pub const REGISTERS: [(&str, RegDef); 3] = [
    ("a", RegDef { primitive: U8 }),
    ("x", RegDef { primitive: U8 }),
    ("y", RegDef { primitive: U8 }),
];

/// Configuration of the 6502 backend
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mos6502 {
    /// Memory layout of the target system
    pub memory: MemoryMap,

    /// Zero-page byte reserved for the backend.
    /// Some sequences need it (comparing memory with memory, moving between
    /// `x` and `y`, operations with `x` or `y` as right operand, ...).
    /// Without it, these operations are reported as errors.
    pub scratch: Option<u8>,
}

impl Mos6502 {
    /// Place the variables and select the instructions, without encoding them
    pub fn select(&self, program: &IrProgram) -> Result<(Layout, Vec<Section<Op>>), BackendError> {
        let functions = || core::iter::once(&program.init).chain(&program.functions);
        let data = program
            .data
            .iter()
            .chain(functions().flat_map(|func| &func.locals));
        let layout = link::allocate(&self.memory, data)?;

        let mut synthetic = 0;
        let sections = functions()
            .enumerate()
            .map(|(id, func)| {
                select::select(
                    func,
                    id as u32,
                    &layout.symbols,
                    self.scratch,
                    &mut synthetic,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((layout, sections))
    }

    /// Assembly listing of the program (for debugging)
    pub fn listing(&self, program: &IrProgram) -> Result<String, BackendError> {
        let (_, sections) = self.select(program)?;
        Ok(link::listing(&sections))
    }
}

impl Backend for Mos6502 {
    fn assemble(&self, program: &IrProgram) -> Result<Image, BackendError> {
        let (layout, sections) = self.select(program)?;
        link::link(&self.memory, layout, sections)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        isa::{Mnemonic, Mode, opcode},
        *,
    };
    use crate::{
        intermediate::{
            Condition, IrData, IrFunc, IrStorage, JumpTarget, Label, MemCell, OpBinary, OpBranch,
            OpCode, OpCompare, OpTransfer, OpUnary, Operand,
        },
        link::{Bank, BankKind, Symbol},
    };
    use asmanite_common::{
        ToBox,
        ident::Ident,
        operator::{Operator, op},
        path::Path,
        register::RegIdent,
        status::StatusFlag,
    };
    use std::collections::HashSet;

    const SCRATCH: u8 = 0xFF;

    fn backend() -> Mos6502 {
        Mos6502 {
            memory: MemoryMap {
                banks: vec![
                    Bank::new("zp", 0x0000, 0x00F0, BankKind::Ram),
                    Bank::new("ram", 0x0200, 0x0600, BankKind::Ram),
                    Bank::new("code", 0x8000, 0x8000, BankKind::Rom),
                ],
                startup: Ident::new("code"),
                fill: 0xFF,
            },
            scratch: Some(SCRATCH),
        }
    }

    fn reg(name: &str) -> Operand {
        Operand::Reg(RegIdent::new(name))
    }

    fn mem(name: &str) -> Operand {
        Operand::mem(Path::from(name))
    }

    fn indexed(name: &str, offset: i64, index: &str) -> Operand {
        Operand::Mem(
            MemCell {
                var: Path::from(name),
                offset,
                index: Some(RegIdent::new(index)),
            }
            .to_box(),
        )
    }

    fn var(name: &str, size: u32, bank: &str) -> IrData {
        IrData {
            name: Path::from(name),
            size,
            storage: IrStorage::Bank(Ident::new(bank)),
        }
    }

    fn transfer(dst: Operand, src: Operand) -> OpCode {
        OpCode::Transfer(OpTransfer { dst, src }.to_box())
    }

    fn binary(op: Operator, dst: Operand, lhs: Operand, rhs: Operand) -> OpCode {
        OpCode::Binary(OpBinary { op, dst, lhs, rhs }.to_box())
    }

    fn label(id: u32) -> OpCode {
        OpCode::Label(Label::Local(id))
    }

    fn target(id: u32) -> JumpTarget {
        JumpTarget::Label(Label::Local(id))
    }

    fn compare(op: Operator, lhs: Operand, rhs: Operand, to: u32) -> OpCode {
        OpCode::Branch(
            OpBranch {
                cond: Condition::Compare(OpCompare { op, lhs, rhs }.to_box()),
                target: target(to),
            }
            .to_box(),
        )
    }

    fn func(name: &str, body: Vec<OpCode>) -> IrFunc {
        IrFunc {
            name: Path::from(name),
            storage: IrStorage::Bank(Ident::new("code")),
            params: Vec::new(),
            ret: None,
            locals: Vec::new(),
            body,
        }
    }

    /// Program with an empty startup routine (one `rts` at $8000)
    fn program(data: Vec<IrData>, functions: Vec<IrFunc>) -> IrProgram {
        let mut init = func("init", vec![OpCode::Return]);
        init.storage = IrStorage::Startup;
        IrProgram {
            data,
            init,
            functions,
        }
    }

    /// Code generated for the first function after the startup routine
    fn code(data: Vec<IrData>, body: Vec<OpCode>) -> Vec<u8> {
        let image = backend()
            .assemble(&program(data, vec![func("f", body)]))
            .expect("program should assemble");
        let symbols = &image.symbols;
        let start = symbols.get(&Symbol::Func(Path::from("f"))).unwrap() as usize;
        let rom = &image.banks[0];
        assert_eq!(rom.name, Ident::new("code"));
        // the function is the last thing in the bank: stop at the fill bytes
        let bytes = &rom.bytes[start - 0x8000..];
        let end = bytes.iter().rposition(|&b| b != 0xFF).map_or(0, |i| i + 1);
        bytes[..end].to_vec()
    }

    #[test]
    fn has_the_151_official_opcodes() {
        let mut opcodes = HashSet::new();
        for mnemonic in Mnemonic::ALL {
            for mode in Mode::ALL {
                if let Some(code) = opcode(mnemonic, mode) {
                    assert!(opcodes.insert(code), "duplicate opcode {code:02X}");
                }
            }
        }
        assert_eq!(opcodes.len(), 151);
    }

    #[test]
    fn transfers_with_zero_page_and_absolute_modes() {
        let data = vec![var("small", 1, "zp"), var("big", 4, "ram")];
        let body = vec![
            transfer(reg("a"), Operand::Imm(5u64.into())),
            transfer(mem("small"), reg("a")),
            transfer(reg("x"), mem("big")),
            transfer(indexed("big", 1, "x"), reg("a")),
            transfer(reg("y"), reg("a")),
            OpCode::Return,
        ];
        #[rustfmt::skip]
        assert_eq!(code(data, body), vec![
            0xA9, 0x05,       // lda #5
            0x85, 0x00,       // sta small
            0xAE, 0x00, 0x02, // ldx big
            0x9D, 0x01, 0x02, // sta big+1,x
            0xA8,             // tay
            0x60,             // rts
        ]);
    }

    #[test]
    fn preserves_the_accumulator_when_it_is_not_the_destination() {
        let data = vec![var("total", 1, "zp"), var("other", 1, "ram")];
        let body = vec![
            // total = 3
            transfer(mem("total"), Operand::Imm(3u64.into())),
            // other = x + 3
            binary(op::ADD, mem("other"), reg("x"), Operand::Imm(3u64.into())),
        ];
        #[rustfmt::skip]
        assert_eq!(code(data, body), vec![
            0x48, 0xA9, 0x03, 0x85, 0x00, 0x68,             // pha / lda #3 / sta total / pla
            0x48, 0x8A, 0x18, 0x69, 0x03, 0x8D, 0x00, 0x02, // pha / txa / clc / adc #3 / sta other
            0x68,                                           // pla
        ]);
    }

    #[test]
    fn selects_arithmetic() {
        let data = vec![var("total", 1, "zp")];
        let body = vec![
            // a += x, through the scratch byte
            binary(op::ADD, reg("a"), reg("a"), reg("x")),
            // total += 1
            binary(
                op::ADD,
                mem("total"),
                mem("total"),
                Operand::Imm(1u64.into()),
            ),
            // a -' total (with carry)
            binary(op::SUB_C, reg("a"), reg("a"), mem("total")),
            // total <<= 2, in place
            binary(
                op::SH_L,
                mem("total"),
                mem("total"),
                Operand::Imm(2u64.into()),
            ),
            // x = -x
            OpCode::Unary(
                OpUnary {
                    op: op::SUB,
                    dst: reg("x"),
                    src: reg("x"),
                }
                .to_box(),
            ),
        ];
        #[rustfmt::skip]
        assert_eq!(code(data, body), vec![
            0x86, SCRATCH, 0x18, 0x65, SCRATCH, // stx scratch / clc / adc scratch
            0xE6, 0x00,                         // inc total
            0xE5, 0x00,                         // sbc total
            0x06, 0x00, 0x06, 0x00,             // asl total / asl total
            0x48, 0x8A, 0x49, 0xFF, 0x18, 0x69, 0x01, 0xAA, 0x68, // pha / txa / eor / clc / adc / tax / pla
        ]);
    }

    #[test]
    fn selects_comparisons() {
        let body = vec![
            label(0),
            OpCode::Unary(
                OpUnary {
                    op: op::INC,
                    dst: reg("x"),
                    src: reg("x"),
                }
                .to_box(),
            ),
            // x < 10: loop
            compare(op::LT, reg("x"), Operand::Imm(10u64.into()), 0),
            // 20 > a, i.e. a < 20
            compare(op::GT, Operand::Imm(20u64.into()), reg("a"), 0),
            // a > 7, i.e. a >= 8
            compare(op::GT, reg("a"), Operand::Imm(7u64.into()), 0),
            // a > y: two branches
            compare(op::GT, reg("a"), reg("y"), 0),
        ];
        #[rustfmt::skip]
        assert_eq!(code(vec![], body), vec![
            0xE8,                   // $8001 inx
            0xE0, 0x0A, 0x90, 0xFB, // cpx #10 / bcc $8001
            0xC9, 0x14, 0x90, 0xF7, // cmp #20 / bcc $8001
            0xC9, 0x08, 0xB0, 0xF3, // cmp #8  / bcs $8001
            0x84, SCRATCH, 0xC5, SCRATCH, // sty scratch / cmp scratch
            0xF0, 0x02, 0xB0, 0xEB, // beq skip / bcs $8001
        ]);
    }

    #[test]
    fn relaxes_branches_out_of_range() {
        let data = vec![var("big", 1, "ram")];
        let far = || (0..50).map(|_| transfer(mem("big"), reg("a"))); // 150 bytes
        let zero = |to| {
            OpCode::Branch(
                OpBranch {
                    cond: Condition::Status {
                        flag: StatusFlag::Zero,
                        inverse: false,
                    },
                    target: target(to),
                }
                .to_box(),
            )
        };
        let body: Vec<_> = core::iter::once(zero(1))
            .chain(far())
            .chain([label(1), label(0)])
            .chain(far())
            .chain([zero(0)])
            .collect();
        let code = code(data, body);

        // forward: bne +3 / jmp $809C
        assert_eq!(code[..5], [0xD0, 0x03, 0x4C, 0x9C, 0x80]);
        // backward: bne +3 / jmp $809C
        assert_eq!(code[code.len() - 5..], [0xD0, 0x03, 0x4C, 0x9C, 0x80]);
        assert_eq!(code.len(), 5 + 150 + 150 + 5);
    }

    #[test]
    fn links_calls_between_functions() {
        let program = program(
            vec![],
            vec![
                func(
                    "main",
                    vec![OpCode::Call(Path::from("sub")), OpCode::Return],
                ),
                func("sub", vec![OpCode::Return]),
            ],
        );
        let image = backend().assemble(&program).unwrap();
        // init: rts / main: jsr sub, rts / sub: rts
        assert_eq!(
            image.banks[0].bytes[..6],
            [0x60, 0x20, 0x05, 0x80, 0x60, 0x60]
        );
    }

    #[test]
    fn reports_unsupported_operations() {
        let program = program(
            vec![],
            vec![func(
                "f",
                vec![binary(op::MUL, reg("a"), reg("a"), reg("x"))],
            )],
        );
        assert!(matches!(
            backend().assemble(&program),
            Err(BackendError::Unsupported { .. })
        ));

        let mut without_scratch = backend();
        without_scratch.scratch = None;
        let program = self::program(
            vec![],
            vec![func(
                "f",
                vec![binary(op::ADD, reg("a"), reg("a"), reg("x"))],
            )],
        );
        assert!(matches!(
            without_scratch.assemble(&program),
            Err(BackendError::NeedsScratch { .. })
        ));
    }

    #[test]
    fn prints_a_listing() {
        let program = program(
            vec![var("v", 1, "zp")],
            vec![func(
                "f",
                vec![transfer(reg("a"), mem("v")), OpCode::Return],
            )],
        );
        let listing = backend().listing(&program).unwrap();
        assert_eq!(listing, "init:\n    rts\nf:\n    lda $00\n    rts\n");
    }
}
