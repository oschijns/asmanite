//! Errors reported while translating the AST into the IR

use asmanite_common::{ident::Ident, path::Path, span::Span};

/// Error raised while translating the AST into the IR
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
#[error("{kind}")]
pub struct LowError {
    /// Span of the error
    pub span: Span,

    /// Kind of error encountered
    pub kind: LowErrorKind,
}

impl LowError {
    /// Create a new error
    #[inline]
    pub fn new(span: Span, kind: LowErrorKind) -> Self {
        Self { span, kind }
    }
}

/// Error raised while translating the AST into the IR
#[derive(thiserror::Error, Debug, Clone, PartialEq)]
pub enum LowErrorKind {
    #[error("`{ident}` is defined multiple times in the same module")]
    Duplicate { ident: Ident },

    #[error("cannot find `{path}`")]
    Unresolved { path: Path },

    #[error("`{path}` is not a {expected}")]
    WrongKind { path: Path, expected: &'static str },

    #[error("expression is not a compile-time constant")]
    NotConstant,

    #[error("constant `{path}` depends on itself")]
    CyclicConstant { path: Path },

    #[error("unknown label `{label}`")]
    UnknownLabel { label: Ident },

    #[error("`break` or `continue` outside of a loop")]
    OutsideLoop,

    #[error("this expression cannot be assigned to")]
    NotAssignable,

    #[error("expected {expected} arguments, found {found}")]
    ArgCount { expected: usize, found: usize },

    #[error("function `{path}` does not return a value")]
    NoReturnValue { path: Path },

    #[error("`{ident}` must be placed in a bank (`in bank`) or at an address (`at address`)")]
    MissingStorage { ident: Ident },

    #[error("`{var_type}` variables can only be declared inside a function")]
    NotInModule { var_type: &'static str },

    #[error("this expression does not map onto an operand, split it into several instructions")]
    NotAnOperand,

    #[error("this expression has no effect")]
    NoEffect,

    #[error(
        "`{ident}` does not exist at startup: a static can only be initialized from constants, statics and registers"
    )]
    NotAtStartup { ident: Ident },

    #[error("an inline function is expanded at each call site, it cannot be placed in memory")]
    InlinePlacement,

    #[error("inline function `{path}` cannot be expanded inside itself")]
    RecursiveInline { path: Path },

    #[error("{what} is not supported yet")]
    Unsupported { what: &'static str },
}

pub type Result<T> = core::result::Result<T, LowError>;

/// Shorthand to report an unsupported feature
#[inline]
pub fn unsupported<T>(what: &'static str, span: Span) -> Result<T> {
    Err(LowError::new(span, LowErrorKind::Unsupported { what }))
}

/// Filter out repeated errors which result from inline function expansion
pub fn filter_unique_errors(errors: Vec<LowError>) -> Box<[LowError]> {
    let mut out = Vec::with_capacity(errors.len());
    for err in errors {
        if !out.contains(&err) {
            out.push(err);
        }
    }
    out.into_boxed_slice()
}
