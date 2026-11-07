use crate::{lexer::atom::Ident, parser::path::Path};
use indextree::{Arena, NodeId};
use std::collections::HashMap;

/// Name of the root module which is reserved
const ROOT: &str = "crate";

/// Keep track where the symbols are defined in the project relative to their modules.
pub struct SymbolTree {
    /// Storage for the module-tree
    arena: Arena<Ident>,

    /// Extra data for the elements
    /// Specify if the symbol denote a module, variable, function, type, etc..
    symbol_data: HashMap<NodeId, SymbolData>,

    /// The root module
    root: NodeId,
}

/// Extra data associated with the symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolData {
    /// Public or private marker
    public: bool,

    /// Type of object the symbol denote
    symbol_type: SymbolType,
}

/// Specify what the variable denotes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Module (mod)
    Module,

    /// Variable (var, static, const)
    Variable,

    /// Funtion (fn)
    Function,

    /// Type (type, struct, enum, union)
    Type,
}

impl Default for SymbolTree {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTree {
    /// Construct an empty symbol tree
    #[inline]
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let mut symbol_data = HashMap::new();

        // Create the root node
        let root = arena.new_node(Ident::new(ROOT));
        symbol_data.insert(root, SymbolData::new(true, SymbolType::Module));

        Self {
            arena,
            symbol_data,
            root,
        }
    }

    /// Construct an empty symbol tree with the given capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arena = Arena::with_capacity(capacity);
        let mut symbol_data = HashMap::with_capacity(capacity);

        // Create the root node
        let root = arena.new_node(Ident::new(ROOT));
        symbol_data.insert(root, SymbolData::new(true, SymbolType::Module));

        Self {
            arena,
            symbol_data,
            root,
        }
    }

    /// Find a node given an absolute path
    #[inline]
    pub fn find(&self, target: &Path) -> Option<NodeId> {
        self.find_from(self.root, target)
    }

    /// Find a node from a starting point and a relative path
    pub fn find_from(&self, start: NodeId, target: &Path) -> Option<NodeId> {
        let mut current = start;

        // try finding a node by following a path
        for elem in target.as_slice() {
            current = current
                .children(&self.arena)
                .find(|child| self.arena[*child].get() == elem)?;
        }
        Some(current)
    }

    /// Given s starting point and a path to a target symbol, find the corresponding node id
    pub fn resolve_path_from(&self, start: NodeId, target: &Path) -> Option<NodeId> {
        let mut current = Some(start);
        while let Some(node) = current {
            if let Some(found) = self.find_from(node, target) {
                return Some(found);
            }
            current = node.parent(&self.arena);
        }
        None
    }
}

impl SymbolData {
    /// Create data related to a symbol
    #[inline]
    pub const fn new(public: bool, symbol_type: SymbolType) -> Self {
        Self {
            public,
            symbol_type,
        }
    }
}
