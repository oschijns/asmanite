//! Startup routine: runtime initialization of the static variables

use super::{
    error::TrsError,
    function::{FuncLowerer, Lowered},
    scope::Local,
    symbols::Globals,
};
use asmanite_ast::parser::expression::Expr;
use asmanite_common::{
    ident::Ident,
    path::Path,
    symbol_tree::{NodeId, ROOT},
};
use asmanite_ir::inter::{
    IrFunc, IrStorage,
    opcode::{OpCode, Operand},
};
use std::collections::HashMap;

/// Initializer of a static variable, lowered into the startup routine
pub struct StaticInit<'a> {
    /// Static variable to initialize
    pub(super) target: Path,

    /// Initial value
    pub(super) value: &'a Expr,

    /// Module from which names are resolved
    pub(super) module: NodeId,

    /// Local variables visible from the initializer (for statics declared in functions)
    pub(super) scope: HashMap<Ident, Local<'a>>,
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    /// Lowerer for the startup routine
    pub fn startup(globals: &'g Globals<'a>, root: NodeId) -> Self {
        let mut lowerer = Self::new(globals, root, Path::from(ROOT));
        lowerer.startup = true;
        lowerer
    }

    /// Startup routine: compute the initial value of a static variable at runtime
    pub fn lower_static_init(&mut self, init: StaticInit<'a>) {
        // A static declared in an inline function is found at each expansion
        if !self.initialized.insert(init.target.clone()) {
            return;
        }
        // The initializer resolves names from where the static is declared
        self.module = init.module;
        self.scopes = vec![init.scope];
        if let Err(error) = self.lower_into(Operand::mem(init.target), init.value) {
            self.errors.push(error);
        }
    }

    /// Finalize the startup routine.
    /// Return every error found in the initializers if it could not be lowered.
    pub fn finish_init(mut self) -> core::result::Result<Lowered<'a>, Vec<TrsError>> {
        // Statics declared in inline functions expanded by the startup routine
        while let Some(init) = self.static_inits.pop() {
            self.lower_static_init(init);
        }
        self.emit(OpCode::Return);
        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        Ok(Lowered {
            func: IrFunc {
                name: self.path,
                storage: IrStorage::Startup,
                params: Vec::new(),
                ret: None,
                locals: self.locals,
                body: self.body,
            },
            static_inits: Vec::new(),
            inline_data: self.inline_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::transform::{TrsError, TrsErrorKind, transform};

    use asmanite_common::{ident::Ident, path::Path, symbol_tree::ROOT};

    use crate::transform::test_utils::*;
    use asmanite_ir::inter::{
        IrData, IrStorage,
        opcode::{OpCode, Operand},
    };

    #[test]
    fn reserves_statics_and_initializes_them_at_startup() {
        let program = translate(
            "const LIMIT: u8 = 2 + 3;
             static DEVICE: u8 at 0x0010 = a = LIMIT;
             static BUFFER: [u8; LIMIT] in ram;
             mod sub { static FLAG: u8 in ram; }",
        );

        // Constants take no memory, statics are placed where the user said
        assert_eq!(
            program.data,
            vec![
                IrData {
                    name: Path::from("DEVICE"),
                    size: 1,
                    storage: IrStorage::Address(0x10),
                },
                IrData {
                    name: Path::from("BUFFER"),
                    size: 5,
                    storage: IrStorage::Bank(Ident::new("ram")),
                },
                IrData {
                    name: Path::from("sub::FLAG"),
                    size: 1,
                    storage: IrStorage::Bank(Ident::new("ram")),
                },
            ]
        );

        // The initial value of a static is computed at runtime, through a register
        assert_eq!(program.init.name, Path::from(ROOT));
        assert_eq!(program.init.storage, IrStorage::Startup);
        assert_eq!(
            program.init.body,
            vec![
                transfer(reg("a"), Operand::Imm(5)),
                transfer(mem("DEVICE"), reg("a")),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn initializes_function_statics_after_global_ones() {
        let program = translate(
            "fn f in code () {
                const OFFSET: u8 = 2;
                static COUNT: u8 in ram = a = OFFSET;
                static TOTAL: u8 in ram = x = COUNT;
             }
             static BASE: u8 at 0x10 = a = 1;",
        );

        // BASE is declared last, but global statics are initialized first
        assert_eq!(
            program.init.body,
            vec![
                transfer(reg("a"), Operand::Imm(1)),
                transfer(mem("BASE"), reg("a")),
                transfer(reg("a"), Operand::Imm(2)),
                transfer(mem("f::COUNT"), reg("a")),
                transfer(reg("x"), mem("f::COUNT")),
                transfer(mem("f::TOTAL"), reg("x")),
                OpCode::Return,
            ]
        );
        // The memory of the statics belongs to the function, no code is emitted there
        let func = function(&program, "f");
        assert_eq!(func.locals.len(), 2);
        assert_eq!(func.body, vec![label(0), OpCode::Return]);
    }

    #[test]
    fn rejects_runtime_values_in_static_initializers() {
        let errors = transform(&parse(
            "fn f in code (v: u8 in zp) { static S: u8 in ram = a = v; }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [TrsError { kind: TrsErrorKind::NotAtStartup { ident, .. }, .. }]
                if *ident == Ident::new("v")),
            "unexpected errors: {errors:?}"
        );
    }
}
