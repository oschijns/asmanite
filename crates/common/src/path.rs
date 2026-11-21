use crate::ident::Ident;
use core::{fmt, ops};

/// Path to a variable, type or function
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(pub Box<[Ident]>);

impl Path {
    /// Create a path instance from a slice of strings
    #[inline]
    pub fn new(elements: Vec<Ident>) -> Self {
        Self(elements.into_boxed_slice())
    }

    /// Create a path instance from a slice of strings
    pub fn from_slice(elements: &[&str]) -> Self {
        Self(elements.iter().map(|e| Ident::new(e)).collect::<Box<[_]>>())
    }

    /// Is the path empty ?
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the path as a slice
    #[inline]
    pub fn as_slice(&self) -> &[Ident] {
        &self.0
    }

    /// Construct a new pat hfrom to slices of identifiers
    #[allow(unused)]
    pub(crate) fn concat(path_a: &[Ident], path_b: &[Ident]) -> Self {
        let mut buffer = Vec::with_capacity(path_a.len() + path_b.len());
        for e in path_a.iter() {
            buffer.push(e.clone());
        }
        for e in path_b {
            buffer.push(e.clone());
        }
        Self(buffer.into_boxed_slice())
    }
}

/// Simplify constructing a path from a raw string for testing purpose
impl From<&str> for Path {
    fn from(value: &str) -> Self {
        Self(value.split("::").map(Ident::new).collect::<Box<[_]>>())
    }
}

impl ops::Index<usize> for Path {
    type Output = Ident;

    /// Get an identifier in this path
    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.0[index]
    }
}

impl ops::Index<ops::Range<usize>> for Path {
    type Output = [Ident];

    /// Get a sub-slice over this path
    #[inline]
    fn index(&self, index: ops::Range<usize>) -> &Self::Output {
        &self.0[index]
    }
}

impl ops::Add for Path {
    type Output = Path;

    /// Concatenate two paths into one
    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        let mut buffer = Vec::with_capacity(self.0.len() + rhs.0.len());
        for e in self.0 {
            buffer.push(e);
        }
        for e in rhs.0 {
            buffer.push(e);
        }
        Self(buffer.into_boxed_slice())
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            // A path cannot be empty
            Err(fmt::Error)
        } else {
            // Display all path elements with "::" in between
            write!(f, "{}", self.0[0])?;
            for elem in self.0[1..].iter() {
                write!(f, "::{}", elem)?;
            }
            Ok(())
        }
    }
}
