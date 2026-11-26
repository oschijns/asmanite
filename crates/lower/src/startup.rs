//! Startup routine: runtime initialization of the static variables

use super::{
    error::LowError,
    function::{FuncLowerer, FuncSharedCtx, Lowered},
    scope::Local,
};
use crate::symbol::{NodeId, ROOT, SymbolTree};
use asmanite_ast::parser::expression::Expr;
use asmanite_common::{ident::Ident, path::Path};
use asmanite_ir::intermediate::{IrData, IrFunc, IrStorage, OpCode, Operand};
use std::collections::{HashMap, HashSet};

/// Initializer of a static variable, lowered into the startup routine
pub struct StaticInit<'a> {
    /// Static variable to initialize
    target: Path,

    /// Initial value
    value: &'a Expr,

    /// Module from which names are resolved
    module: NodeId,

    /// Local variables visible from the initializer (for statics declared in functions)
    scope: HashMap<Ident, Local<'a>>,
}

impl<'a> StaticInit<'a> {
    /// Create a initializer
    #[inline]
    pub fn new(
        target: Path,
        value: &'a Expr,
        module: NodeId,
        scope: HashMap<Ident, Local<'a>>,
    ) -> Self {
        Self {
            target,
            value,
            module,
            scope,
        }
    }
}

/// Startup routine being lowered: one initializer after the other
pub struct Startup<'g, 'a> {
    /// Every symbol declared in the program
    globals: &'g SymbolTree<'a>,

    /// Output of the routine, shared by the initializers
    shared: FuncSharedCtx<'a>,

    /// Memory cells owned by the routine
    locals: Vec<IrData>,

    /// Statics already initialized
    initialized: HashSet<Path>,
}

impl<'g, 'a> Startup<'g, 'a> {
    /// Empty startup routine
    pub fn new(globals: &'g SymbolTree<'a>) -> Self {
        Self {
            globals,
            shared: FuncSharedCtx::default(),
            locals: Vec::new(),
            initialized: HashSet::new(),
        }
    }

    /// Compute the initial value of a static variable at runtime
    pub fn lower_static_init(&mut self, init: StaticInit<'a>) {
        // A static declared in an inline function is found at each expansion
        if !self.initialized.insert(init.target.clone()) {
            return;
        }
        // The initializer resolves names from where the static is declared
        let mut lowerer = FuncLowerer::new(
            self.globals,
            &mut self.shared,
            init.module,
            Path::from(ROOT),
        );
        lowerer.startup = true;
        lowerer.scopes.push(init.scope);
        if let Err(error) = lowerer.lower_into(Operand::mem(init.target), init.value) {
            lowerer.shared.errors.push(error);
        }
        let cells = lowerer.locals;
        self.locals.extend(cells);
    }

    /// Finalize the startup routine.
    /// Return every error found in the initializers if it could not be lowered.
    pub fn finish(mut self) -> Result<Lowered<'a>, Vec<LowError>> {
        // Statics declared in inline functions expanded by the startup routine
        while let Some(init) = self.shared.static_inits.pop() {
            self.lower_static_init(init);
        }
        self.shared.body.push(OpCode::Return);
        if !self.shared.errors.is_empty() {
            return Err(self.shared.errors);
        }
        Ok(Lowered {
            func: IrFunc {
                name: Path::from(ROOT),
                storage: IrStorage::Startup,
                params: Vec::new(),
                ret: None,
                locals: self.locals,
                body: self.shared.body,
            },
            static_inits: Vec::new(),
            inline_data: self.shared.inline_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::symbol::ROOT;
    use crate::{LowError, LowErrorKind, test_utils::*, translate};
    use asmanite_common::{ident::Ident, path::Path};
    use asmanite_ir::intermediate::{IrData, IrStorage, OpCode, Operand};

    #[test]
    fn reserves_statics_and_initializes_them_at_startup() {
        let program = run_translation(
            "const LIMIT: u8 = 2 + 3;
             static DEVICE: u8 at 0x0010 = a = LIMIT;
             static BUFFER: [u8; LIMIT] in ram;
             mod sub { static FLAG: u8 in ram; }",
        );

        // Constants take no memory, statics are placed where the user said
        assert_eq!(
            program.data,
            vec![
                IrData::new(Path::from("DEVICE"), 1, IrStorage::Addr(0x10)),
                IrData::new(Path::from("BUFFER"), 5, IrStorage::Bank(Ident::new("ram"))),
                IrData::new(
                    Path::from("sub::FLAG"),
                    1,
                    IrStorage::Bank(Ident::new("ram"))
                ),
            ]
        );

        // The initial value of a static is computed at runtime, through a register
        assert_eq!(program.init.name, Path::from(ROOT));
        assert_eq!(program.init.storage, IrStorage::Startup);
        assert_eq!(
            program.init.body,
            vec![
                transfer(reg("a"), Operand::Imm(5u64.into())),
                transfer(mem("DEVICE"), reg("a")),
                OpCode::Return,
            ]
        );
    }

    #[test]
    fn initializes_function_statics_after_global_ones() {
        let program = run_translation(
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
                transfer(reg("a"), Operand::Imm(1u64.into())),
                transfer(mem("BASE"), reg("a")),
                transfer(reg("a"), Operand::Imm(2u64.into())),
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
        let errors = translate(&parse(
            "fn f in code (v: u8 in zp) { static S: u8 in ram = a = v; }",
        ))
        .expect_err("translation should fail");

        assert!(
            matches!(&errors[..], [LowError { kind: LowErrorKind::NotAtStartup { ident, .. }, .. }]
                if *ident == Ident::new("v")),
            "unexpected errors: {errors:?}"
        );
    }
}
