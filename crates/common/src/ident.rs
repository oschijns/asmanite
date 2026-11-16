use core::fmt;

/// Identifier for register, variable, function, type
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ident(pub Box<str>);

impl Ident {
    /// Instantiate a new identifier given its name
    #[inline]
    pub fn new(ident: &str) -> Self {
        Self(ident.to_owned().into_boxed_str())
    }
}

/// Create a new register identifier from a raw string
impl From<&str> for Ident {
    #[inline]
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

/// Implement Display on identifier
impl fmt::Display for Ident {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
