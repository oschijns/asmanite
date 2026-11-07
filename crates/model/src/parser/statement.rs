use crate::{lexer::define::*, parser::expression::Expr};

// MARK: AST

/// Variable declaration
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    /// Type of variable declaration
    pub decl_type: VarDeclType,

    /// Identifier of the variable
    pub ident: Ident,

    // TODO: for now we are onyl supporting primitives
    /// Type of the variable
    pub var_type: Primitive,

    /// Storage location for the variable
    pub storage: VarStorage,

    /// Assign a value to the variable when declaring it.
    /// Note: Only regular assignments (`=`) are allowed here.
    pub assign_expr: Option<Box<Expr>>,
}

/// Specify where to store the variable
#[derive(Debug, Default, Clone, PartialEq)]
pub enum VarStorage {
    /// If unspecified, the variable will fallback to the parent's scope bank.
    #[default]
    Undefined,

    /// Find an available memory location in the given bank.
    In(Ident),

    /// Fixed address for the variable.
    /// Provided address must be solvable at compile-time.
    At(Box<Expr>),
}

// MARK: Parsers
