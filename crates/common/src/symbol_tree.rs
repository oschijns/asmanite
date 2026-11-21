use crate::{ident::Ident, path::Path};
use indextree::Arena;

/// Identifier of a node in the symbol tree
pub use indextree::NodeId;

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

    /// Get the root module of the tree
    #[inline]
    pub fn root(&self) -> NodeId {
        self.root
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
        if target.is_empty() {
            None
        } else {
            // Check if the first element of the path is "root"
            // thus returning to the root module first
            let mut slice = target.as_slice();
            if slice[0].0.as_ref() == ROOT {
                current = self.root;
                slice = &slice[1..];
            }

            // Iterate over the tree to find the target
            for elem in slice {
                current = current
                    .children(&self.arena)
                    .find(|child| self.arena[*child].get().ident == *elem)?;
            }
            Some(current)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Tree used by most tests:
    ///
    /// ```text
    /// root
    /// ├── LIMIT            (variable)
    /// ├── helper           (function)
    /// └── outer            (module)
    ///     ├── LIMIT        (variable, shadows root::LIMIT)
    ///     └── inner        (module)
    ///         └── process  (function)
    /// ```
    struct Fixture {
        tree: SymbolTree,
        root_limit: NodeId,
        helper: NodeId,
        outer: NodeId,
        outer_limit: NodeId,
        inner: NodeId,
        process: NodeId,
    }

    fn data(ident: &str, symbol_type: SymbolType) -> SymbolData {
        SymbolData::new(Ident::new(ident), symbol_type)
    }

    fn fixture() -> Fixture {
        let mut tree = SymbolTree::new();
        let root = tree.root();
        let root_limit = tree.insert(root, data("LIMIT", SymbolType::Variable));
        let helper = tree.insert(root, data("helper", SymbolType::Function));
        let outer = tree.insert(root, data("outer", SymbolType::Module));
        let outer_limit = tree.insert(outer, data("LIMIT", SymbolType::Variable));
        let inner = tree.insert(outer, data("inner", SymbolType::Module));
        let process = tree.insert(inner, data("process", SymbolType::Function));
        Fixture {
            tree,
            root_limit,
            helper,
            outer,
            outer_limit,
            inner,
            process,
        }
    }

    // MARK: Construction

    #[test]
    fn new_tree_only_contains_the_root_module() {
        let tree = SymbolTree::new();
        let root = tree.root();

        assert_eq!(tree.get(root), Some(&data(ROOT, SymbolType::Module)));
        assert_eq!(tree.find(&Path::from("anything")), None);
    }

    #[test]
    fn default_and_with_capacity_match_new() {
        for tree in [SymbolTree::default(), SymbolTree::with_capacity(16)] {
            assert_eq!(tree.get(tree.root()), Some(&data(ROOT, SymbolType::Module)));
            assert_eq!(tree.find(&Path::from("anything")), None);
        }
    }

    // MARK: Insertion and access

    #[test]
    fn inserted_symbols_keep_their_data() {
        let f = fixture();

        assert_eq!(
            f.tree.get(f.root_limit),
            Some(&data("LIMIT", SymbolType::Variable))
        );
        assert_eq!(
            f.tree.get(f.helper),
            Some(&data("helper", SymbolType::Function))
        );
        assert_eq!(
            f.tree.get(f.outer),
            Some(&data("outer", SymbolType::Module))
        );
        assert_eq!(
            f.tree.get(f.process),
            Some(&data("process", SymbolType::Function))
        );
    }

    #[test]
    fn same_name_in_different_modules_creates_distinct_symbols() {
        let f = fixture();

        assert_ne!(f.root_limit, f.outer_limit);
        assert_eq!(f.tree.get(f.root_limit), f.tree.get(f.outer_limit));
    }

    #[test]
    fn get_returns_none_for_a_node_of_another_tree() {
        let f = fixture();
        let small = SymbolTree::new();

        // `process` is the 7th node of the fixture, `small` only has one node
        assert_eq!(small.get(f.process), None);
    }

    // MARK: Parent

    #[test]
    fn parent_returns_the_enclosing_module() {
        let f = fixture();
        let root = f.tree.root();

        assert_eq!(f.tree.parent(f.process), f.inner);
        assert_eq!(f.tree.parent(f.inner), f.outer);
        assert_eq!(f.tree.parent(f.outer_limit), f.outer);
        assert_eq!(f.tree.parent(f.outer), root);
        assert_eq!(f.tree.parent(f.helper), root);
    }

    #[test]
    fn parent_of_the_root_is_the_root() {
        let f = fixture();
        let root = f.tree.root();

        assert_eq!(f.tree.parent(root), root);
    }

    // MARK: Find

    #[test]
    fn find_follows_an_absolute_path_from_the_root() {
        let f = fixture();

        assert_eq!(f.tree.find(&Path::from("helper")), Some(f.helper));
        assert_eq!(f.tree.find(&Path::from("outer")), Some(f.outer));
        assert_eq!(
            f.tree.find(&Path::from("outer::LIMIT")),
            Some(f.outer_limit)
        );
        assert_eq!(
            f.tree.find(&Path::from("outer::inner::process")),
            Some(f.process)
        );
    }

    #[test]
    fn find_fails_when_any_element_is_missing() {
        let f = fixture();

        assert_eq!(f.tree.find(&Path::from("missing")), None);
        assert_eq!(f.tree.find(&Path::from("outer::missing")), None);
        assert_eq!(f.tree.find(&Path::from("missing::inner::process")), None);
        // `process` is not directly in `outer`
        assert_eq!(f.tree.find(&Path::from("outer::process")), None);
        // paths cannot go through a non-module symbol's children (it has none)
        assert_eq!(f.tree.find(&Path::from("helper::process")), None);
    }

    #[test]
    fn find_rejects_an_empty_path() {
        let f = fixture();

        assert_eq!(f.tree.find(&Path::new(Vec::new())), None);
        assert_eq!(f.tree.find_from(f.inner, &Path::new(Vec::new())), None);
    }

    #[test]
    fn find_from_is_relative_to_the_starting_node() {
        let f = fixture();

        assert_eq!(
            f.tree.find_from(f.outer, &Path::from("LIMIT")),
            Some(f.outer_limit)
        );
        assert_eq!(
            f.tree.find_from(f.outer, &Path::from("inner::process")),
            Some(f.process)
        );
        assert_eq!(
            f.tree.find_from(f.inner, &Path::from("process")),
            Some(f.process)
        );
        // find_from never looks into the parents
        assert_eq!(f.tree.find_from(f.inner, &Path::from("helper")), None);
    }

    #[test]
    fn find_from_with_root_prefix_starts_from_the_root() {
        let f = fixture();

        assert_eq!(
            f.tree.find_from(f.inner, &Path::from("root::helper")),
            Some(f.helper)
        );
        assert_eq!(
            f.tree.find_from(f.inner, &Path::from("root::LIMIT")),
            Some(f.root_limit)
        );
        assert_eq!(
            f.tree
                .find_from(f.process, &Path::from("root::outer::inner::process")),
            Some(f.process)
        );
        assert_eq!(f.tree.find(&Path::from("root::outer")), Some(f.outer));
    }

    #[test]
    fn root_alone_designates_the_root_module() {
        let f = fixture();
        let root = f.tree.root();

        assert_eq!(f.tree.find(&Path::from(ROOT)), Some(root));
        assert_eq!(f.tree.find_from(f.inner, &Path::from(ROOT)), Some(root));
    }

    #[test]
    fn root_is_only_special_as_the_first_element() {
        let f = fixture();

        assert_eq!(f.tree.find(&Path::from("outer::root")), None);
        assert_eq!(f.tree.find(&Path::from("outer::root::helper")), None);
    }

    // MARK: Resolve

    #[test]
    fn resolve_finds_symbols_in_the_starting_module() {
        let f = fixture();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("process")),
            Some(f.process)
        );
        assert_eq!(
            f.tree
                .resolve_path_from(f.outer, &Path::from("inner::process")),
            Some(f.process)
        );
    }

    #[test]
    fn resolve_walks_up_the_enclosing_modules() {
        let f = fixture();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("helper")),
            Some(f.helper)
        );
        assert_eq!(
            f.tree
                .resolve_path_from(f.process, &Path::from("outer::inner")),
            Some(f.inner)
        );
    }

    #[test]
    fn resolve_prefers_the_closest_symbol() {
        let f = fixture();
        let root = f.tree.root();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("LIMIT")),
            Some(f.outer_limit)
        );
        assert_eq!(
            f.tree.resolve_path_from(root, &Path::from("LIMIT")),
            Some(f.root_limit)
        );
    }

    #[test]
    fn resolve_with_root_prefix_ignores_closer_symbols() {
        let f = fixture();

        assert_eq!(
            f.tree
                .resolve_path_from(f.inner, &Path::from("root::LIMIT")),
            Some(f.root_limit)
        );
        assert_eq!(
            f.tree
                .resolve_path_from(f.inner, &Path::from("root::outer::LIMIT")),
            Some(f.outer_limit)
        );
    }

    #[test]
    fn resolve_fails_for_unknown_symbols() {
        let f = fixture();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("missing")),
            None
        );
        assert_eq!(
            f.tree
                .resolve_path_from(f.inner, &Path::from("root::missing")),
            None
        );
        // a child module is not visible from its parent without its prefix
        assert_eq!(
            f.tree.resolve_path_from(f.outer, &Path::from("process")),
            None
        );
        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::new(Vec::new())),
            None
        );
    }
}
