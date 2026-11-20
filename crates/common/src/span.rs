use crate::ToBox;
use core::{fmt, ops::Add};

/// Invalid span for unit test purposes
pub const UNSPANNED: Span = Span::invalid();

/// Span over a source code
#[derive(Debug, Clone, Copy, Eq)]
pub struct Span {
    /// Start of the span
    pub start: u32,

    // End of the span
    pub end: u32,
}

impl Span {
    /// Create a new span
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    /// Create an invalid span
    #[inline]
    pub const fn invalid() -> Self {
        Self {
            start: u32::MAX,
            end: u32::MAX,
        }
    }

    /// Invalid span
    #[inline]
    pub fn is_invalid(self) -> bool {
        self.start == u32::MAX && self.end == u32::MAX
    }

    /// Is the span empty ?
    #[inline]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }

    /// Get the length of the span
    #[inline]
    pub fn len(self) -> usize {
        (self.end - self.start) as usize
    }
}

impl Add for Span {
    type Output = Span;

    /// Combine two span into a larger one
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            start: self.start.min(rhs.start),
            end: self.end.max(rhs.end),
        }
    }
}

/// Ignore span comparison in unit-tests
pub static mut IGNORE_SPAN: bool = false;

impl PartialEq for Span {
    /// Compare two spans
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        let result = self.start == other.start && self.end == other.end;
        unsafe { IGNORE_SPAN || result }
    }
}

/// Implement this trait on AST nodes to access their spawn
pub trait Spanned {
    /// Get the span of an element
    fn span(&self) -> Span;
}

impl<T: Spanned> Spanned for Box<T> {
    /// Trivially get the span of a boxed type
    #[inline]
    fn span(&self) -> Span {
        self.as_ref().span()
    }
}

impl<T: Spanned> Spanned for [T] {
    /// We assume that arrays imply the first element appeared first
    /// in the source code, while the last element appeared at last.
    #[inline]
    fn span(&self) -> Span {
        if let [first, .., last] = self {
            first.span() + last.span()
        } else {
            Span::invalid()
        }
    }
}

impl<T: Spanned> Spanned for Vec<T> {
    /// We assume that arrays imply the first element appeared first
    /// in the source code, while the last element appeared at last.
    #[inline]
    fn span(&self) -> Span {
        self.as_slice().span()
    }
}

/// Wrapper adding span to a given type
#[derive(Debug, Clone, Copy, Eq)]
pub struct WithSpan<T> {
    /// Atom to attach a span to
    pub atom: T,

    /// Spanof the atom
    pub span: Span,
}

impl<T> WithSpan<T> {
    /// Create a new instance
    #[inline]
    pub fn new(span: Span, atom: T) -> Self {
        Self { atom, span }
    }

    /// Create a new instance without span (for unit-testing)
    #[inline]
    pub fn unspanned(atom: T) -> Self {
        Self {
            atom,
            span: Span::invalid(),
        }
    }
}

impl<T> Spanned for WithSpan<T> {
    /// Get the span
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

impl<T: PartialEq> PartialEq for WithSpan<T> {
    /// When checking for equality, ignore the span
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.atom == other.atom
    }
}

impl<T: fmt::Display> fmt::Display for WithSpan<T> {
    /// Display the atom without the span
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.atom.fmt(f)
    }
}

impl<T> ToBox for WithSpan<T> {}
