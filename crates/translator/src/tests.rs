//! End-to-end tests: asmanite source → IR → machine code

use crate::transform::test_utils::translate;
use asmanite_common::ident::Ident;
use asmanite_ir::{
    backend::{Backend, mos6502::Mos6502},
    link::{Bank, BankKind, MemoryMap},
};

/// 6502 with a zero page, some RAM and a 32 KiB ROM at $8000
fn mos6502() -> Mos6502 {
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
        scratch: Some(0xFF),
    }
}

#[test]
fn compiles_if_else_to_6502() {
    let program = translate(
        "fn pick in code () {
            if a < 10 { x = 1; } else { x = 2; }
        }",
    );
    let image = mos6502()
        .assemble(&program)
        .expect("program should assemble");

    let rom = &image.banks[0];
    assert_eq!(rom.name, Ident::new("code"));
    #[rustfmt::skip]
    assert_eq!(rom.bytes[..13], [
        // $8000: startup routine (no static to initialize)
        0x60,             // rts
        // $8001: pick
        0xC9, 0x0A,       // cmp #10
        0xB0, 0x05,       // bcs else ($800A)
        0xA2, 0x01,       // ldx #1
        0x4C, 0x0C, 0x80, // jmp end ($800C)
        0xA2, 0x02,       // else: ldx #2
        0x60,             // end:  rts
    ]);
    // the rest of the bank is unused
    assert!(rom.bytes[13..].iter().all(|&byte| byte == 0xFF));
}
