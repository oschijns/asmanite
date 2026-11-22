//! Pass 1: register every declaration of the program in the symbol tree

use super::error::{Result, TrsError, TrsErrorKind, unsupported};
use asmanite_ast::parser::{
    composite_type::TypeDecl,
    function::FuncDecl,
    memory::{BankElement, VarDecl},
    module::{ModElement, Module},
};
use asmanite_common::{
    ident::Ident,
    path::Path,
    span::{Span, Spanned},
    symbol_tree::{NodeId, SymbolData, SymbolTree, SymbolType},
};
use std::collections::HashMap;

/// Declaration a symbol refers to
#[derive(Debug, Clone, Copy)]
pub enum Decl<'a> {
    Module,
    Type,
    Var(&'a VarDecl),
    Func(&'a FuncDecl),
}

/// A symbol declared in a module
#[derive(Debug)]
pub struct Symbol<'a> {
    /// Absolute path of the symbol, used as its name in the IR
    pub(super) path: Path,

    /// Module in which the symbol is declared (names in it resolve from there)
    pub(super) module: NodeId,

    /// Bank of the enclosing `in bank { ... }` block, if any
    pub(super) bank: Option<Ident>,

    /// The declaration itself
    pub(super) decl: Decl<'a>,
}

/// Every symbol declared in the program
pub struct Globals<'a> {
    pub(super) tree: SymbolTree,
    pub(super) symbols: HashMap<NodeId, Symbol<'a>>,

    /// Declaration order, to keep the output deterministic
    pub(super) order: Vec<NodeId>,
}

impl<'a> Globals<'a> {
    pub fn new() -> Self {
        Self {
            tree: SymbolTree::new(),
            symbols: HashMap::new(),
            order: Vec::new(),
        }
    }

    /// Pass 1: register the elements of a module and of its sub-modules
    pub fn collect(
        &mut self,
        module: &'a Module,
        node: NodeId,
        prefix: &[Ident],
        errors: &mut Vec<TrsError>,
    ) {
        for element in module.elements.iter() {
            let declared = match element {
                ModElement::Mod(sub) => self
                    .declare(
                        node,
                        prefix,
                        &sub.ident,
                        SymbolType::Module,
                        Decl::Module,
                        sub.span,
                    )
                    .map(|child| {
                        let mut sub_prefix = prefix.to_vec();
                        sub_prefix.push(sub.ident.clone());
                        self.collect(sub, child, &sub_prefix, errors);
                    }),
                ModElement::ModDecl(decl) => unsupported(
                    "loading a module from another file (`mod name;`)",
                    decl.span,
                ),
                // TODO: resolve imports into per-module alias tables
                ModElement::Use(_) => Ok(()),
                ModElement::Type(ty) => {
                    let name = match ty {
                        TypeDecl::Composite(c) => c.name.as_ref(),
                        TypeDecl::Enum(e) => e.name.as_ref(),
                        TypeDecl::Alias(a) => Some(&a.name),
                    };
                    match name {
                        Some(name) => self
                            .declare(node, prefix, name, SymbolType::Type, Decl::Type, ty.span())
                            .map(drop),
                        None => Ok(()),
                    }
                }
                ModElement::Var(var) => self
                    .declare(
                        node,
                        prefix,
                        &var.ident,
                        SymbolType::Variable,
                        Decl::Var(var),
                        var.span,
                    )
                    .map(drop),
                ModElement::Func(func) => self
                    .declare(
                        node,
                        prefix,
                        &func.ident,
                        SymbolType::Function,
                        Decl::Func(func),
                        func.span,
                    )
                    .map(drop),
                // The elements of a bank block belong to the enclosing module
                ModElement::Bank(bank) => {
                    for element in bank.elements.iter() {
                        let declared = match element {
                            BankElement::Var(var) => self.declare(
                                node,
                                prefix,
                                &var.ident,
                                SymbolType::Variable,
                                Decl::Var(var),
                                var.span,
                            ),
                            BankElement::Func(func) => self.declare(
                                node,
                                prefix,
                                &func.ident,
                                SymbolType::Function,
                                Decl::Func(func),
                                func.span,
                            ),
                        };
                        match declared {
                            Ok(child) => {
                                if let Some(symbol) = self.symbols.get_mut(&child) {
                                    symbol.bank = Some(bank.ident.clone());
                                }
                            }
                            Err(error) => errors.push(error),
                        }
                    }
                    Ok(())
                }
            };
            if let Err(error) = declared {
                errors.push(error);
            }
        }
    }

    /// Insert a symbol in the tree, rejecting duplicates in the same module
    pub fn declare(
        &mut self,
        parent: NodeId,
        prefix: &[Ident],
        ident: &Ident,
        symbol_type: SymbolType,
        decl: Decl<'a>,
        span: Span,
    ) -> Result<NodeId> {
        if self
            .tree
            .find_from(parent, &Path::new(vec![ident.clone()]))
            .is_some()
        {
            return Err(TrsError::new(
                span,
                TrsErrorKind::Duplicate {
                    ident: ident.clone(),
                },
            ));
        }

        let node = self
            .tree
            .insert(parent, SymbolData::new(ident.clone(), symbol_type));
        let mut path = prefix.to_vec();
        path.push(ident.clone());
        self.symbols.insert(
            node,
            Symbol {
                path: Path::new(path),
                module: parent,
                bank: None,
                decl,
            },
        );
        self.order.push(node);
        Ok(node)
    }

    /// Find the symbol a path refers to, starting from a module
    pub fn resolve(&self, module: NodeId, path: &Path, span: Span) -> Result<&Symbol<'a>> {
        self.tree
            .resolve_path_from(module, path)
            .and_then(|node| self.symbols.get(&node))
            .ok_or_else(|| TrsError::new(span, TrsErrorKind::Unresolved { path: path.clone() }))
    }
}
