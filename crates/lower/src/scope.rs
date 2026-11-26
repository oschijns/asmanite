//! Local scopes: declaration and lookup of the names visible in a function

use super::{
    error::{LowError, LowErrorKind, LowResult},
    function::{FuncLowerer, child},
    startup::StaticInit,
};
use crate::symbol::{SymbolType, SymbolValue, no_locals};
use asmanite_ast::{
    lexer::atom::VarType,
    parser::{composite_type::TypeUsed, expression::Expr, memory::VarDecl},
};
use asmanite_common::{
    ident::Ident,
    number::Number,
    path::Path,
    span::{Span, Spanned},
};
use asmanite_ir::intermediate::{IrData, Operand};
use std::collections::HashMap;

/// Variable visible in a function body
#[derive(Debug, Clone)]
pub struct Local<'a> {
    /// Where the variable lives (an immediate value for constants)
    pub(crate) operand: Operand,

    /// Declared type, if any (used to compute element sizes)
    pub(crate) ty: Option<&'a TypeUsed>,

    /// What kind of variable it is
    pub(crate) kind: LocalKind,
}

/// Kind of local variable
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalKind {
    /// Compile-time constant (`const`, `let`)
    Const,

    /// Static variable, exists from the startup of the program
    Static,

    /// Variable or parameter, only exists while the function runs
    Runtime,
}

impl<'g, 'a> FuncLowerer<'g, 'a> {
    pub fn bind(
        &mut self,
        ident: Ident,
        operand: Operand,
        ty: Option<&'a TypeUsed>,
        kind: LocalKind,
    ) {
        let local = Local { operand, ty, kind };
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(ident, local);
        }
    }

    /// Find a local variable (only single identifiers can be locals)
    pub fn local(&self, path: &Path) -> Option<&Local<'a>> {
        if let [ident] = path.as_slice() {
            self.scopes.iter().rev().find_map(|scope| scope.get(ident))
        } else {
            None
        }
    }

    /// Every local variable visible from the current scope
    pub fn visible_locals(&self) -> HashMap<Ident, Local<'a>> {
        let mut visible = HashMap::new();
        for scope in &self.scopes {
            for (ident, local) in scope {
                visible.insert(ident.clone(), local.clone());
            }
        }
        visible
    }

    /// Unique IR name for a local variable (shadowed names get a suffix)
    pub fn local_path(&self, ident: &Ident) -> Path {
        let base = child(&self.path, ident.clone());
        let taken = |path: &Path| self.locals.iter().any(|data| data.name == *path);
        if taken(&base) {
            // '.' cannot appear in a user identifier
            (1..)
                .map(|n| child(&self.path, Ident::new(&format!("{ident}.{n}"))))
                .find(|path| !taken(path))
                .expect("there is always a free name")
        } else {
            base
        }
    }

    /// Declare a variable in the current scope
    pub fn declare_local(&mut self, var: &'a VarDecl, bank: Option<Ident>) -> LowResult<()> {
        match var.var_type {
            // Constants are replaced by their value
            VarType::Const | VarType::Let => {
                let value = match &var.assign_expr {
                    Some(expr) => self.require_const(expr)?,
                    None => return Err(LowError::new(var.span, LowErrorKind::NotConstant)),
                };
                self.bind(
                    var.ident.clone(),
                    Operand::Imm(value),
                    Some(&var.type_used),
                    LocalKind::Const,
                );
                Ok(())
            }
            VarType::Stack => Err(LowError::unsupported(var.span, "stack variables")),
            VarType::Var | VarType::Static => {
                let locals = |path: &Path| self.local_const(path);
                let storage = self
                    .globals
                    .var_storage(var, self.module, &locals, bank.as_ref())?;
                let size = self.globals.size_of(&var.type_used, self.module, &locals)?;

                // The initializer is evaluated before the variable is visible
                let name = self.local_path(&var.ident);
                let operand = Operand::mem(name.clone());
                let kind = match var.var_type {
                    VarType::Static => LocalKind::Static,
                    _ => LocalKind::Runtime,
                };
                match (&var.assign_expr, kind) {
                    // A static is initialized once, by the startup routine
                    (Some(value), LocalKind::Static) => self.static_inits.push(StaticInit::new(
                        name.clone(),
                        value,
                        self.module,
                        self.visible_locals(),
                    )),
                    // A var is initialized each time it is declared
                    (Some(value), _) => self.lower_into(operand.clone(), value)?,
                    (None, _) => {}
                }

                self.locals.push(IrData::new(name, size, storage));
                self.bind(var.ident.clone(), operand, Some(&var.type_used), kind);
                Ok(())
            }
        }
    }

    /// Operand of a variable
    pub fn lookup_var(&self, path: &Path, span: Span) -> LowResult<Operand> {
        if let Some(local) = self.local(path) {
            if self.startup && local.kind == LocalKind::Runtime {
                Err(LowError::not_at_startup(span, path[0].clone()))
            } else {
                Ok(local.operand.clone())
            }
        } else {
            let symbol = self.globals.resolve(self.module, path, span)?;
            match symbol.symbol_type {
                SymbolType::Var(var) if matches!(var.var_type, VarType::Const | VarType::Let) => {
                    let value = var
                        .assign_expr
                        .as_ref()
                        .ok_or(LowError::new(span, LowErrorKind::NotConstant))?;
                    self.globals
                        .eval_const(
                            value,
                            self.globals.module_of(symbol),
                            &no_locals,
                            &mut Vec::new(),
                        )
                        .map(Operand::Imm)
                }
                SymbolType::Var(_) => Ok(Operand::mem(symbol.path().clone())),
                _ => Err(LowError::new(
                    span,
                    LowErrorKind::WrongKind {
                        path: path.clone(),
                        expected: "variable",
                    },
                )),
            }
        }
    }

    /// Value of a local constant (see [`LocalConsts`])
    pub fn local_const(&self, path: &Path) -> SymbolValue {
        match self.local(path) {
            None => SymbolValue::None,
            Some(local) => match &local.operand {
                Operand::Imm(value) if local.kind == LocalKind::Const => {
                    SymbolValue::LocalConst(*value)
                }
                _ => SymbolValue::Local,
            },
        }
    }

    /// Try to evaluate an expression at compile time, using local constants
    pub fn try_const(&self, expr: &Expr) -> Option<Number> {
        let locals = |path: &Path| self.local_const(path);
        self.globals
            .eval_const(expr, self.module, &locals, &mut Vec::new())
            .ok()
    }

    /// Evaluate an expression which must be constant
    pub fn require_const(&self, expr: &Expr) -> LowResult<Number> {
        self.try_const(expr)
            .ok_or(LowError::new(expr.span(), LowErrorKind::NotConstant))
    }
}
