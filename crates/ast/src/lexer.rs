/// Define factorized atoms.
/// This helps simplifying lexing and parsing.
pub mod atom;

/// Tokens and lexer
pub mod token;

#[cfg(test)]
pub(crate) fn init_test_registers() {
    use asmanite_common::{
        primitive::def::U8,
        register::{REGISTERS, RegDef, RegSet},
    };

    REGISTERS.get_or_init(|| {
        RegSet::new(&[
            ("a", RegDef::new(U8)),
            ("x", RegDef::new(U8)),
            ("y", RegDef::new(U8)),
        ])
    });
}
