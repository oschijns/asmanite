use crate::{ident::Ident, path::Path};
use indextree::{Arena, NodeId};

/// Name of the root module which is reserved
pub const ROOT: &str = "root";

/// Keep track where the symbols are defined in the project relative to their modules.
pub struct SymbolTree {
    /// Storage for the module-tree
    arena: Arena<SymbolData>,

    /// The root module
    root: NodeId,
}

/// Extra data associated with the symbol
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolData {
    /// Identifier of the element
    ident: Ident,

    /// Type of object the symbol denote
    symbol_type: SymbolType,
}

impl SymbolData {
    /// Create a new node in the symbol tree
    #[inline]
    pub fn new(ident: Ident, symbol_type: SymbolType) -> Self {
        Self { ident, symbol_type }
    }
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

        // Create the root node
        let root = arena.new_node(SymbolData::new(Ident::new(ROOT), SymbolType::Module));
        Self { arena, root }
    }

    /// Construct an empty symbol tree with the given capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        let mut arena = Arena::with_capacity(capacity);

        // Create the root node
        let root = arena.new_node(SymbolData::new(Ident::new(ROOT), SymbolType::Module));
        Self { arena, root }
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
                .find(|child| self.arena[*child].get().ident == *elem)?;
        }
        Some(current)
    }

    /// Get the parent of a given node
    #[inline]
    pub fn parent(&self, node: NodeId) -> NodeId {
        node.parent(&self.arena).unwrap_or(self.root)
    }

    /// Insert an element in the tree
    #[inline]
    pub fn insert(&mut self, parent: NodeId, data: SymbolData) -> NodeId {
        parent.append_value(data, &mut self.arena)
    }

    /// Get the data of a given node
    #[inline]
    pub fn get(&self, node: NodeId) -> Option<&SymbolData> {
        self.arena.get(node).map(|n| n.get())
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
