//! Symbols of the program: where they are declared relative to their modules,
//! how they are registered from the AST (pass 1 of the translation)
//! and evaluation of the compile-time constants they declare.

use crate::error::{LowError, LowErrorKind, LowResult};
use asmanite_ast::{
    lexer::atom::VarType,
    parser::{
        composite_type::TypeDecl,
        expression::Expr,
        function::FuncDecl,
        memory::{BankElement, VarDecl},
        module::{ModElement, Module},
    },
};
use asmanite_common::{
    ident::Ident,
    number::Number,
    path::Path,
    span::{Span, Spanned},
};
use asmanite_ir::Addr;
use indextree::Arena;

/// Identifier of a node in the symbol tree
pub use indextree::NodeId;

/// Name of the root module which is reserved
pub const ROOT: &str = "root";

// MARK: TREE

/// Every symbol declared in the program,
/// kept where it is defined relative to its module.
pub struct SymbolTree<'a> {
    /// Storage for the module-tree
    arena: Arena<SymbolData<'a>>,

    /// The root module
    root: NodeId,
}

/// Data associated with a symbol of the tree.
/// Its path is used as its name in the IR, and the names in its declaration
/// resolve from its module.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolData<'a> {
    /// Identifier of the element
    pub ident: Ident,

    /// Type of object the symbol denote, with its declaration
    pub symbol_type: SymbolType<'a>,

    /// Absolute path of the symbol (empty for the root module)
    pub path: Path,

    /// Module in which the symbol is declared (`None` for the root module)
    pub module: Option<NodeId>,

    /// Bank of the enclosing `in bank { ... }` block, if any
    pub bank: Option<Ident>,
}

impl<'a> SymbolData<'a> {
    /// Identifier of the symbol
    #[inline]
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    /// Type of object the symbol denote
    #[inline]
    pub fn symbol_type(&self) -> SymbolType<'a> {
        self.symbol_type
    }

    /// Absolute path of the symbol, without the `root` prefix
    #[inline]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Module in which the symbol is declared (`None` for the root module)
    #[inline]
    pub fn module(&self) -> Option<NodeId> {
        self.module
    }
}

/// Specify what the symbol denotes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolType<'a> {
    /// Symbol is a module
    Module,

    /// Symbol is a type
    Type,

    /// Symbol is a variable
    Var(&'a VarDecl),

    /// Symbol is a function
    Func(&'a FuncDecl),
}

impl<'a> Default for SymbolTree<'a> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SymbolTree<'a> {
    /// Construct an empty symbol tree
    #[inline]
    pub fn new() -> Self {
        Self::with_arena(Arena::new())
    }

    /// Construct an empty symbol tree with the given capacity
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_arena(Arena::with_capacity(capacity))
    }

    /// Create the root module in the arena
    fn with_arena(mut arena: Arena<SymbolData<'a>>) -> Self {
        let root = arena.new_node(SymbolData {
            ident: Ident::new(ROOT),
            symbol_type: SymbolType::Module,
            path: Path::new(Vec::new()),
            module: None,
            bank: None,
        });
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
                current = self.find_child(current, elem)?;
            }
            Some(current)
        }
    }

    /// Find a direct child of a node
    fn find_child(&self, parent: NodeId, ident: &Ident) -> Option<NodeId> {
        parent
            .children(&self.arena)
            .find(|child| self.arena[*child].get().ident == *ident)
    }

    /// Get the parent of a given node
    #[inline]
    pub fn parent(&self, node: NodeId) -> NodeId {
        node.parent(&self.arena).unwrap_or(self.root)
    }

    /// Insert a symbol in a module.
    /// Fails with the existing symbol if the module already declares that identifier.
    pub fn insert(
        &mut self,
        parent: NodeId,
        ident: Ident,
        symbol_type: SymbolType<'a>,
        bank: Option<Ident>,
    ) -> Result<NodeId, NodeId> {
        if let Some(existing) = self.find_child(parent, &ident) {
            return Err(existing);
        }
        let mut path = self.arena[parent].get().path.as_slice().to_vec();
        path.push(ident.clone());
        let symbol = SymbolData {
            ident,
            symbol_type,
            path: Path::new(path),
            module: Some(parent),
            bank,
        };
        Ok(parent.append_value(symbol, &mut self.arena))
    }

    /// Get the data of a given node
    #[inline]
    pub fn get(&self, node: NodeId) -> Option<&SymbolData<'a>> {
        self.arena.get(node).map(|n| n.get())
    }

    /// Every symbol of the tree (except the root module), in insertion order
    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &SymbolData<'a>)> {
        self.arena
            .iter()
            .filter_map(|node| Some((self.arena.get_node_id(node)?, node.get())))
            .filter(|(node, _)| *node != self.root)
    }

    /// Every symbol of the program, in declaration order
    pub fn symbols(&self) -> impl Iterator<Item = &SymbolData<'a>> {
        self.iter().map(|(_, symbol)| symbol)
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

    /// Find the symbol a path refers to, starting from a module
    pub fn resolve(&self, module: NodeId, path: &Path, span: Span) -> LowResult<&SymbolData<'a>> {
        self.resolve_path_from(module, path)
            .and_then(|node| self.get(node))
            .ok_or_else(|| LowError::unresolved(span, path.clone()))
    }

    /// Module in which a symbol is declared (the root module is in itself)
    pub fn module_of(&self, symbol: &SymbolData<'a>) -> NodeId {
        symbol.module().unwrap_or(self.root)
    }
}

// MARK: COLLECT

impl<'a> SymbolTree<'a> {
    /// Pass 1: register the elements of a module and of its sub-modules
    pub fn collect(&mut self, module: &'a Module, node: NodeId, errors: &mut Vec<LowError>) {
        for element in module.elements.iter() {
            let declared = match element {
                ModElement::Mod(sub) => self
                    .declare(node, &sub.ident, SymbolType::Module, None, sub.span)
                    .map(|child| self.collect(sub, child, errors)),
                ModElement::ModDecl(decl) => Err(LowError::unsupported(
                    decl.span,
                    "loading a module from another file (`mod name;`)",
                )),
                // TODO: resolve imports into per-module alias tables
                ModElement::Use(_) => Ok(()),
                ModElement::Type(ty) => {
                    let name = match ty {
                        TypeDecl::Composite(c) => c.name.as_ref(),
                        TypeDecl::Enum(e) => e.name.as_ref(),
                        TypeDecl::Alias(a) => Some(&a.name),
                    };
                    match name {
                        Some(name) => self
                            .declare(node, name, SymbolType::Type, None, ty.span())
                            .map(drop),
                        None => Ok(()),
                    }
                }
                ModElement::Var(var) => self
                    .declare(node, &var.ident, SymbolType::Var(var), None, var.span)
                    .map(drop),
                ModElement::Func(func) => self
                    .declare(node, &func.ident, SymbolType::Func(func), None, func.span)
                    .map(drop),
                // The elements of a bank block belong to the enclosing module
                ModElement::Bank(bank) => {
                    let in_bank = Some(bank.ident.clone());
                    for element in bank.elements.iter() {
                        let (ident, symbol_type, span) = match element {
                            BankElement::Var(var) => (&var.ident, SymbolType::Var(var), var.span),
                            BankElement::Func(func) => {
                                (&func.ident, SymbolType::Func(func), func.span)
                            }
                        };
                        let declared =
                            self.declare(node, ident, symbol_type, in_bank.clone(), span);
                        if let Err(error) = declared {
                            errors.push(error);
                        }
                    }
                    Ok(())
                }
            };
            if let Err(error) = declared {
                errors.push(error);
            }
        }
    }

    /// Insert a symbol in the tree, rejecting duplicates in the same module
    pub fn declare(
        &mut self,
        parent: NodeId,
        ident: &Ident,
        symbol_type: SymbolType<'a>,
        bank: Option<Ident>,
        span: Span,
    ) -> LowResult<NodeId> {
        self.insert(parent, ident.clone(), symbol_type, bank)
            .map_err(|_| LowError::duplicate(span, ident.clone()))
    }
}

// MARK: CONSTANTS

/// Evaluation of constant expression
#[derive(Debug, Default, Clone, Copy)]
pub enum SymbolValue {
    /// Not a local
    #[default]
    None,

    /// Local symbol but not a constant
    Local,

    /// Local constant
    LocalConst(Number),
}

/// Lookup of local constants while evaluating a constant expression
/// (see [`SymbolValue`])
pub type LocalConsts<'l> = &'l dyn Fn(&Path) -> SymbolValue;

/// Used when there is no local scope (global declarations)
pub fn no_locals(_: &Path) -> SymbolValue {
    SymbolValue::None
}

impl<'a> SymbolTree<'a> {
    /// Evaluate an expression at compile time
    pub fn eval_const(
        &self,
        expr: &Expr,
        module: NodeId,
        locals: LocalConsts,
        visiting: &mut Vec<NodeId>,
    ) -> LowResult<Number> {
        let not_const = LowError::new(expr.span(), LowErrorKind::NotConstant);

        match expr {
            Expr::Lit(lit) => Number::from_literal(&lit.atom).ok_or(not_const),
            Expr::Group(group) => self.eval_const(&group.atom, module, locals, visiting),
            // TODO: truncate the value to the size of the primitive
            Expr::Cast(cast) => self.eval_const(&cast.expr, module, locals, visiting),
            Expr::Var(var) => match locals(&var.atom) {
                SymbolValue::LocalConst(value) => Ok(value),
                SymbolValue::Local => Err(not_const),
                SymbolValue::None => {
                    let node = self
                        .resolve_path_from(module, &var.atom)
                        .ok_or_else(|| LowError::unresolved(var.span, var.atom.clone()))?;
                    let symbol = self.get(node).expect("resolved nodes exist");

                    if let SymbolType::Var(decl) = symbol.symbol_type
                        && matches!(decl.var_type, VarType::Const | VarType::Let)
                        && let Some(value) = &decl.assign_expr
                    {
                        if visiting.contains(&node) {
                            Err(LowError::cyclic_const(var.span, symbol.path.clone()))
                        } else {
                            visiting.push(node);
                            let result = self.eval_const(
                                value,
                                self.module_of(symbol),
                                &no_locals,
                                visiting,
                            );
                            visiting.pop();
                            result
                        }
                    } else {
                        Err(not_const)
                    }
                }
            },
            Expr::Unary(unary) => {
                let value = self.eval_const(&unary.expr, module, locals, visiting)?;
                unary.op.op_type.eval_unary_num(value).ok_or(not_const)
            }
            Expr::Binary(binary) if !binary.op.carry => {
                let lhs = self.eval_const(&binary.lhs, module, locals, visiting)?;
                let rhs = self.eval_const(&binary.rhs, module, locals, visiting)?;
                binary.op.op_type.eval_binary_num(lhs, rhs).ok_or(not_const)
            }
            _ => Err(not_const),
        }
    }

    /// Evaluate a fixed address
    pub fn address(&self, expr: &Expr, module: NodeId, locals: LocalConsts) -> LowResult<Addr> {
        let value = self.eval_const(expr, module, locals, &mut Vec::new())?;
        Addr::try_from(value).map_err(|_| LowError::new(expr.span(), LowErrorKind::NotConstant))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tree used by most tests:
    ///
    /// ```text
    /// root
    /// ├── LIMIT            (type)
    /// ├── helper           (type, in bank `rom`)
    /// └── outer            (module)
    ///     ├── LIMIT        (type, shadows root::LIMIT)
    ///     └── inner        (module)
    ///         └── process  (type)
    /// ```
    struct Fixture {
        tree: SymbolTree<'static>,
        root_limit: NodeId,
        helper: NodeId,
        outer: NodeId,
        outer_limit: NodeId,
        inner: NodeId,
        process: NodeId,
    }

    fn insert(
        tree: &mut SymbolTree<'static>,
        parent: NodeId,
        ident: &str,
        symbol_type: SymbolType<'static>,
        bank: Option<&str>,
    ) -> NodeId {
        tree.insert(parent, Ident::new(ident), symbol_type, bank.map(Ident::new))
            .expect("no duplicate in the fixture")
    }

    fn fixture() -> Fixture {
        use SymbolType::{Module, Type};

        let mut tree = SymbolTree::new();
        let t = &mut tree;
        let root = t.root();
        let root_limit = insert(t, root, "LIMIT", Type, None);
        let helper = insert(t, root, "helper", Type, Some("rom"));
        let outer = insert(t, root, "outer", Module, None);
        let outer_limit = insert(t, outer, "LIMIT", Type, None);
        let inner = insert(t, outer, "inner", Module, None);
        let process = insert(t, inner, "process", Type, None);
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
        let root = tree.get(tree.root()).unwrap();

        assert_eq!(root.ident(), &Ident::new(ROOT));
        assert_eq!(root.symbol_type(), SymbolType::Module);
        assert_eq!(root.path(), &Path::new(Vec::new()));
        assert_eq!(root.module(), None);
        assert_eq!(tree.find(&Path::from("anything")), None);
        assert_eq!(tree.iter().count(), 0);
    }

    #[test]
    fn default_and_with_capacity_match_new() {
        for tree in [SymbolTree::default(), SymbolTree::with_capacity(16)] {
            let root = tree.get(tree.root()).unwrap();
            assert_eq!(root.ident(), &Ident::new(ROOT));
            assert_eq!(tree.find(&Path::from("anything")), None);
        }
    }

    // MARK: Insertion and access

    #[test]
    fn inserted_symbols_keep_their_data() {
        let f = fixture();
        let symbol = |node| f.tree.get(node).unwrap();

        assert_eq!(symbol(f.root_limit).ident(), &Ident::new("LIMIT"));
        assert_eq!(symbol(f.root_limit).symbol_type(), SymbolType::Type);
        assert_eq!(symbol(f.root_limit).bank, None);
        assert_eq!(symbol(f.helper).bank, Some(Ident::new("rom")));
        assert_eq!(symbol(f.outer).symbol_type(), SymbolType::Module);
        assert_eq!(symbol(f.process).ident(), &Ident::new("process"));
    }

    #[test]
    fn same_name_in_different_modules_creates_distinct_symbols() {
        let f = fixture();

        assert_ne!(f.root_limit, f.outer_limit);
        assert_eq!(
            f.tree.get(f.root_limit).map(SymbolData::ident),
            f.tree.get(f.outer_limit).map(SymbolData::ident)
        );
    }

    #[test]
    fn insert_rejects_duplicates_in_the_same_module() {
        let mut f = fixture();
        let root = f.tree.root();

        let duplicate = f
            .tree
            .insert(f.outer, Ident::new("LIMIT"), SymbolType::Module, None);
        assert_eq!(duplicate, Err(f.outer_limit));
        let duplicate = f
            .tree
            .insert(root, Ident::new("outer"), SymbolType::Type, None);
        assert_eq!(duplicate, Err(f.outer));
        // the existing symbols are left untouched
        let limit = f.tree.get(f.outer_limit).unwrap();
        assert_eq!(limit.symbol_type(), SymbolType::Type);
        assert_eq!(f.tree.iter().count(), 6);
    }

    #[test]
    fn symbols_know_their_absolute_path_and_module() {
        let f = fixture();
        let root = f.tree.root();
        let symbol = |node| f.tree.get(node).unwrap();

        assert_eq!(symbol(f.helper).path(), &Path::from("helper"));
        assert_eq!(symbol(f.helper).module(), Some(root));
        assert_eq!(symbol(f.outer_limit).path(), &Path::from("outer::LIMIT"));
        assert_eq!(symbol(f.outer_limit).module(), Some(f.outer));
        assert_eq!(
            symbol(f.process).path(),
            &Path::from("outer::inner::process")
        );
        assert_eq!(f.tree.module_of(symbol(f.process)), f.inner);
        // the root module is in itself
        assert_eq!(f.tree.module_of(symbol(root)), root);
    }

    #[test]
    fn iter_visits_every_symbol_in_insertion_order() {
        let f = fixture();

        let nodes: Vec<_> = f.tree.iter().map(|(node, _)| node).collect();
        assert_eq!(
            nodes,
            [
                f.root_limit,
                f.helper,
                f.outer,
                f.outer_limit,
                f.inner,
                f.process
            ]
        );
        let paths: Vec<_> = f.tree.symbols().map(|s| s.path().to_string()).collect();
        assert_eq!(
            paths,
            [
                "LIMIT",
                "helper",
                "outer",
                "outer::LIMIT",
                "outer::inner",
                "outer::inner::process"
            ]
        );
    }

    #[test]
    fn get_returns_none_for_a_node_of_another_tree() {
        let f = fixture();
        let small = SymbolTree::new();

        // `process` is the 7th node of the fixture, `small` only has one node
        assert!(small.get(f.process).is_none());
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
        // the parent of the root is the root
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
        // find_from never looks into the parents
        assert_eq!(f.tree.find_from(f.inner, &Path::from("helper")), None);
    }

    #[test]
    fn root_prefix_starts_from_the_root() {
        let f = fixture();
        let root = f.tree.root();

        assert_eq!(
            f.tree.find_from(f.inner, &Path::from("root::helper")),
            Some(f.helper)
        );
        assert_eq!(f.tree.find(&Path::from(ROOT)), Some(root));
        assert_eq!(f.tree.find_from(f.inner, &Path::from(ROOT)), Some(root));
        // `root` is only special as the first element
        assert_eq!(f.tree.find(&Path::from("outer::root")), None);
    }

    // MARK: Resolve

    #[test]
    fn resolve_walks_up_the_enclosing_modules() {
        let f = fixture();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("process")),
            Some(f.process)
        );
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

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("LIMIT")),
            Some(f.outer_limit)
        );
        assert_eq!(
            f.tree
                .resolve_path_from(f.inner, &Path::from("root::LIMIT")),
            Some(f.root_limit)
        );
    }

    #[test]
    fn resolve_fails_for_unknown_symbols() {
        let f = fixture();
        let span = Span::invalid();

        assert_eq!(
            f.tree.resolve_path_from(f.inner, &Path::from("missing")),
            None
        );
        // a child module is not visible from its parent without its prefix
        assert_eq!(
            f.tree.resolve_path_from(f.outer, &Path::from("process")),
            None
        );
        let error = f.tree.resolve(f.inner, &Path::from("missing"), span);
        assert_eq!(
            error.map(SymbolData::path).unwrap_err().kind,
            LowErrorKind::Unresolved {
                path: Path::from("missing")
            }
        );
    }
}
