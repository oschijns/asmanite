//! Placement and size of the memory cells

use super::error::{LowError, LowErrorKind, Result, unsupported};
use crate::symbol::{LocalConsts, NodeId, SymbolData, SymbolTree, no_locals};
use asmanite_ast::{
    lexer::atom::VarType,
    parser::{
        composite_type::TypeUsed,
        memory::{MemStorage, ParamStorage, VarDecl},
    },
};
use asmanite_common::{
    ident::Ident,
    span::{Span, Spanned},
};
use asmanite_ir::intermediate::{IrData, IrStorage};

impl<'a> SymbolTree<'a> {
    /// Pass 2: memory to reserve for a variable declared in a module.
    /// Only `const` and `static` can be declared there.
    /// Constants do not need memory: they are replaced by their value.
    pub fn global_data(&self, symbol: &SymbolData<'a>, var: &'a VarDecl) -> Result<Option<IrData>> {
        let module = self.module_of(symbol);
        match var.var_type {
            VarType::Const => {
                // Evaluate it now to report errors even if the constant is never used
                let value = var
                    .assign_expr
                    .as_ref()
                    .ok_or(LowError::new(var.span, LowErrorKind::NotConstant))?;
                self.eval_const(value, module, &no_locals, &mut Vec::new())?;
                Ok(None)
            }
            VarType::Static => Ok(Some(IrData::new(
                symbol.path().clone(),
                self.size_of(&var.type_used, module, &no_locals)?,
                self.var_storage(var, module, &no_locals, symbol.bank.as_ref())?,
            ))),
            VarType::Var | VarType::Let | VarType::Stack => Err(LowError::new(
                var.span,
                LowErrorKind::NotInModule {
                    var_type: var.var_type.into(),
                },
            )),
        }
    }

    /// Size in bytes of a type
    pub fn size_of(&self, ty: &TypeUsed, module: NodeId, locals: LocalConsts) -> Result<u32> {
        match ty {
            TypeUsed::Prim(prim) => Ok(u32::from(prim.atom.size)),
            TypeUsed::Array(array) => {
                let cell = self.size_of(&array.cell_type, module, locals)?;
                let count = self.eval_const(&array.size, module, locals, &mut Vec::new())?;
                u32::try_from(count)
                    .ok()
                    .and_then(|count| count.checked_mul(cell))
                    .ok_or(LowError::new(array.size.span(), LowErrorKind::NotConstant))
            }
            // TODO: struct/union/enum layouts, named types, tuples, pointers
            _ => unsupported("this kind of type", ty.span()),
        }
    }

    /// Placement of a variable or a function, `None` if not specified
    pub fn mem_storage(
        &self,
        storage: &MemStorage,
        module: NodeId,
        locals: LocalConsts,
    ) -> Result<Option<IrStorage>> {
        match storage {
            MemStorage::Undefined => Ok(None),
            MemStorage::In(bank) => Ok(Some(IrStorage::Bank(bank.clone()))),
            MemStorage::At(addr) => self
                .address(addr, module, locals)
                .map(|addr| Some(IrStorage::Addr(addr))),
        }
    }

    /// Placement of a variable, which must be explicit:
    /// given in its declaration, or by the enclosing bank block.
    pub fn var_storage(
        &self,
        var: &VarDecl,
        module: NodeId,
        locals: LocalConsts,
        bank: Option<&Ident>,
    ) -> Result<IrStorage> {
        self.mem_storage(&var.storage, module, locals)?
            .or_else(|| bank.map(|bank| IrStorage::Bank(bank.clone())))
            .ok_or_else(|| {
                LowError::new(
                    var.span,
                    LowErrorKind::MissingStorage {
                        ident: var.ident.clone(),
                    },
                )
            })
    }

    /// Placement of a parameter or a return value
    pub fn param_storage(
        &self,
        storage: &ParamStorage,
        module: NodeId,
        span: Span,
    ) -> Result<IrStorage> {
        match storage {
            ParamStorage::In(bank) => Ok(IrStorage::Bank(bank.clone())),
            ParamStorage::At(addr) => self.address(addr, module, &no_locals).map(IrStorage::Addr),
            ParamStorage::Stack => Ok(IrStorage::Stack),
            ParamStorage::CompTime => unsupported("compile-time parameters", span),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{LowError, LowErrorKind, test_utils::*, translate};
    use asmanite_common::ident::Ident;
    use asmanite_ir::intermediate::IrStorage;

    #[test]
    fn requires_explicit_placement_of_variables() {
        for source in [
            "static FLAG: u8;",
            "mod sub { static FLAG: u8; }",
            "fn f in code () { var local: u8; }",
        ] {
            let errors = translate(&parse(source)).expect_err("translation should fail");
            assert!(
                matches!(&errors[..], [LowError { kind: LowErrorKind::MissingStorage { ident, .. }, .. }]
                    if *ident == Ident::new("FLAG") || *ident == Ident::new("local")),
                "unexpected errors for {source:?}: {errors:?}"
            );
        }

        // An in-bank block gives a placement to its variables
        let program = run_translation("fn f in code () { in ram { var local: u8; } }");
        assert_eq!(
            function(&program, "f").locals[0].storage,
            IrStorage::Bank(Ident::new("ram"))
        );
    }

    #[test]
    fn rejects_runtime_variables_in_modules() {
        for (source, var_type) in [
            ("var counter: u8 in ram;", "var"),
            ("let value: u8 = 1;", "let"),
            ("stack value: u8;", "stack"),
        ] {
            let errors = translate(&parse(source)).expect_err("translation should fail");
            assert!(
                matches!(&errors[..], [LowError { kind: LowErrorKind::NotInModule { var_type: found, .. }, .. }]
                    if *found == var_type),
                "unexpected errors for {source:?}: {errors:?}"
            );
        }
    }
}
