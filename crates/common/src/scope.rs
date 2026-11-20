/// Scope ID for the root of a source file
pub const ROOT: ScopeId = 0;

/// Identifier for code block/scope
pub type ScopeId = u32;

/// Data structure that holds a stack keeping track of the ID of each code block
#[derive(Debug, Default)]
pub struct ScopeStack {
    /// stack
    stack: Vec<ScopeId>,

    /// next scope ID to assign
    counter: ScopeId,
}

impl ScopeStack {
    /// Create a new scope stack
    #[inline]
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            counter: 0,
        }
    }

    /// Create a new scope stack with a given capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            stack: Vec::with_capacity(capacity),
            counter: 0,
        }
    }

    /// Call when opening a new scope
    pub fn on_open(&mut self) -> ScopeId {
        let id = self.counter;
        self.counter += 1;
        self.stack.push(id);
        id
    }

    /// Call when closing a scope
    #[inline]
    pub fn on_close(&mut self) -> ScopeId {
        self.stack.pop().unwrap_or(ROOT)
    }

    /// Get the scope ID of the current block
    #[inline]
    pub fn current(&self) -> ScopeId {
        *self.stack.last().unwrap_or(&ROOT)
    }
}
