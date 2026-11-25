//! First draft of the AST → IR translation.
//!
//! The translation runs in three passes:
//!
//! 1. **Collection**: walk the module tree and register every declaration
//!    (module, variable, function, type) in a [`SymbolTree`].
//! 2. **Global data**: evaluate constants, reserve memory for static variables
//!    and lower their initializers into the startup routine ([`IrProgram::init`]).
//! 3. **Function lowering**: translate each function body into a flat list of
//!    [`OpCode`]s. Structured control flow (`if`, loops, `switch`) is turned into
//!    labels, jumps and conditional branches. Inline functions are expanded at
//!    each call site and produce no code of their own. The initializers of the statics
//!    declared in functions are appended to the startup routine, after the global
//!    ones, as they may depend on them.
//!
//! The language is a high-level assembly: the user decides where every variable
//! lives and which registers perform each operation. The translator follows that:
//! - Placements are never chosen: every `static`, `var` and non-inline function
//!   must be declared `in bank` or `at address`, or be part of an `in bank { ... }`
//!   block.
//! - There are no temporaries: every expression maps onto operands the user wrote.
//!   An expression is either an operand (constant, register, variable, array cell,
//!   assignment or call, whose value is their destination) or a single operation
//!   written into a destination (`a = x + 1`). Anything more complex must be split
//!   into several instructions by the user.
//! - Operations are emitted as written. Multi-byte operations are written byte by
//!   byte by the user (low bytes, then high bytes with the carry), the translator
//!   never splits them.
//! - Only `const` values are computed by the compiler. Everything else, including
//!   the initial value of a `static`, is computed at runtime by the target.
//!   Checking that each operation is encodable on the target CPU (e.g. storing an
//!   immediate value in memory requires a register on the 6502) is the job of the
//!   backend.
//!
//! Limitations of this draft (reported as [`TrsError::Unsupported`] or ignored):
//! - `use` statements are ignored: names resolve by walking up enclosing modules.
//! - Only primitive and array types have a size (no struct/union/enum layouts yet).
//! - Generics, `defer`, `inline for`, field accesses and pointers are not handled.

mod call;
mod condition;
mod constant;
mod emit;
mod error;
mod expression;
mod function;
mod inline;
mod scope;
mod startup;
mod statement;
mod storage;
mod symbols;

#[cfg(test)]
pub(crate) mod test_utils;

pub use error::{TrsError, TrsErrorKind};

use self::{
    function::FuncLowerer,
    startup::StaticInit,
    symbols::{Decl, Globals},
};
use asmanite_ast::parser::{function::FuncMarker, memory::MemStorage, module::Module};
use asmanite_ir::intermediate::{IrData, IrProgram};
use std::collections::HashMap;

// MARK: Entry point

/// Translate the root module of a project into the IR.
///
/// All `mod name;` declarations must already have been replaced by the module
/// they refer to. Every error encountered is reported, not only the first one.
pub fn transform(root: &Module) -> core::result::Result<IrProgram, Vec<TrsError>> {
    let mut errors = Vec::new();

    // Pass 1: register every declaration
    let mut globals = Globals::new();
    let root_node = globals.tree.root();
    globals.collect(root, root_node, &[], &mut errors);

    // Pass 2: constants and static variables.
    // The initializers of the statics are computed at runtime, by the startup routine.
    let mut data = Vec::new();
    let mut init = FuncLowerer::startup(&globals, root_node);
    for &node in &globals.order {
        let symbol = &globals.symbols[&node];
        if let Decl::Var(var) = symbol.decl {
            match globals.global_data(symbol, var) {
                Ok(Some(item)) => {
                    data.push(item);
                    if let Some(value) = &var.assign_expr {
                        init.lower_static_init(StaticInit {
                            target: symbol.path.clone(),
                            value,
                            module: symbol.module,
                            scope: HashMap::new(),
                        });
                    }
                }
                Ok(None) => {}
                Err(error) => errors.push(error),
            }
        }
    }

    // Pass 3: function bodies
    let mut functions = Vec::new();
    let mut local_inits = Vec::new();
    for &node in &globals.order {
        let symbol = &globals.symbols[&node];
        let Decl::Func(func) = symbol.decl else {
            continue;
        };
        // Inline functions only exist where they are expanded
        if func.marker == FuncMarker::Inline {
            if func.storage != MemStorage::Undefined {
                errors.push(TrsError::new(func.span, TrsErrorKind::InlinePlacement));
            }
            continue;
        }
        let lowerer = FuncLowerer::new(&globals, symbol.module, symbol.path.clone());
        match lowerer.lower(func, symbol.bank.as_ref()) {
            Ok(lowered) => {
                local_inits.extend(lowered.static_inits);
                push_new_data(&mut data, lowered.inline_data);
                functions.push(lowered.func);
            }
            Err(func_errors) => errors.extend(func_errors),
        }
    }

    // The statics declared in functions are initialized after the global ones
    for static_init in local_inits {
        init.lower_static_init(static_init);
    }
    let init = match init.finish_init() {
        Ok(lowered) => {
            push_new_data(&mut data, lowered.inline_data);
            Some(lowered.func)
        }
        Err(init_errors) => {
            errors.extend(init_errors);
            None
        }
    };

    // An error in an inline function is found again at each expansion
    let mut unique_errors: Vec<TrsError> = Vec::with_capacity(errors.len());
    for error in errors {
        if !unique_errors.contains(&error) {
            unique_errors.push(error);
        }
    }
    let errors = unique_errors;

    match init {
        Some(init) if errors.is_empty() => Ok(IrProgram {
            data,
            init,
            functions,
        }),
        _ => Err(errors),
    }
}

/// Reserve the memory cells of inline functions.
/// Each expansion declares the same cells, they are reserved only once.
fn push_new_data(data: &mut Vec<IrData>, cells: Vec<IrData>) {
    for cell in cells {
        if !data.iter().any(|known| known.name == cell.name) {
            data.push(cell);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transform::{TrsError, TrsErrorKind, transform};

    use crate::transform::test_utils::*;

    #[test]
    fn reports_every_error() {
        let errors = transform(&parse(
            "fn f in code () { break; goto 'nowhere; unknown = 1; }
             fn f in code () {}",
        ))
        .expect_err("translation should fail");

        assert!(errors.iter().any(|e| matches!(
            e,
            TrsError {
                kind: TrsErrorKind::Duplicate { .. },
                ..
            }
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            TrsError {
                kind: TrsErrorKind::OutsideLoop,
                ..
            }
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            TrsError {
                kind: TrsErrorKind::UnknownLabel { .. },
                ..
            }
        )));
        assert!(errors.iter().any(|e| matches!(
            e,
            TrsError {
                kind: TrsErrorKind::Unresolved { .. },
                ..
            }
        )));
    }
}
