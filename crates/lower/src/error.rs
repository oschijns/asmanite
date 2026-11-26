//! Errors reported while translating the AST into the IR

use asmanite_common::{ident::Ident, path::Path, span::Span};

/// Result of lowering an AST node into IR instructions
pub type LowResult<T> = Result<T, LowError>;

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

    /// Create a new unresolved error
    #[inline]
    pub fn unresolved(span: Span, path: Path) -> Self {
        Self {
            span,
            kind: LowErrorKind::Unresolved { path },
        }
    }

    /// Create a new duplicate error
    #[inline]
    pub fn duplicate(span: Span, ident: Ident) -> Self {
        Self {
            span,
            kind: LowErrorKind::Duplicate { ident },
        }
    }

    /// Create a new unknown label error
    #[inline]
    pub fn unknown_label(span: Span, label: Ident) -> Self {
        Self {
            span,
            kind: LowErrorKind::UnknownLabel { label },
        }
    }

    /// Create a new wrong kind error
    #[inline]
    pub fn wrong_kind(span: Span, path: Path, expected: &'static str) -> Self {
        Self {
            span,
            kind: LowErrorKind::WrongKind { path, expected },
        }
    }

    /// Create a new cyclic constant error
    #[inline]
    pub fn cyclic_const(span: Span, path: Path) -> Self {
        Self {
            span,
            kind: LowErrorKind::CyclicConstant { path },
        }
    }

    /// Create a new argument count error
    #[inline]
    pub fn arg_count(span: Span, expected: usize, found: usize) -> Self {
        Self {
            span,
            kind: LowErrorKind::ArgCount { expected, found },
        }
    }

    /// Create a new missing storage error
    #[inline]
    pub fn missing_storage(span: Span, ident: Ident) -> Self {
        Self {
            span,
            kind: LowErrorKind::MissingStorage { ident },
        }
    }

    /// Create a new missing return value error
    #[inline]
    pub fn no_return_value(span: Span, path: Path) -> Self {
        Self {
            span,
            kind: LowErrorKind::NoReturnValue { path },
        }
    }

    /// Create a new not at startup error
    #[inline]
    pub fn not_at_startup(span: Span, ident: Ident) -> Self {
        Self {
            span,
            kind: LowErrorKind::NotAtStartup { ident },
        }
    }

    /// Create a new not in module error
    #[inline]
    pub fn not_in_mod(span: Span, var_type: &'static str) -> Self {
        Self {
            span,
            kind: LowErrorKind::NotInModule { var_type },
        }
    }

    /// Create a new recusrive inline error
    #[inline]
    pub fn rec_inline(span: Span, path: Path) -> Self {
        Self {
            span,
            kind: LowErrorKind::RecursiveInline { path },
        }
    }

    /// Create a new unsupported feature error
    #[inline]
    pub fn unsupported(span: Span, what: &'static str) -> Self {
        Self {
            span,
            kind: LowErrorKind::Unsupported { what },
        }
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
