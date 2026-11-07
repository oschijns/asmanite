use crate::{lexer::define::*, parser::path::Path};

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

// MARK: Expression

/// Expression AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal
    Lit(Literal),

    /// Path to variable identifier
    Var(Path),

    /// Unary operation
    Unary(UnaryOp),

    /// Binary operation
    Binary(BinaryOp),

    /// Assign operation
    Assign(AssignOp),

    /// Function call
    Call(FunctionCall),

    /// Cast an expression into another type primitive
    Cast(TypeCast),

    /// Parenthesized expression
    Group(Box<Self>),
}

/// Unary operation
#[derive(Debug, Clone, PartialEq)]
pub struct UnaryOp {
    /// Operator applied to the expression
    pub op: Operator,

    /// Operand
    pub expr: Box<Expr>,
}

impl UnaryOp {
    #[inline]
    pub fn new(op: Operator, expr: Expr) -> Self {
        Self {
            op,
            expr: Box::new(expr),
        }
    }
}

/// Binary operation
#[derive(Debug, Clone, PartialEq)]
pub struct BinaryOp {
    /// Operator between the operands
    pub op: Operator,

    /// Left operand
    pub lhs: Box<Expr>,

    /// Right operand
    pub rhs: Box<Expr>,
}

impl BinaryOp {
    #[inline]
    pub fn new(op: Operator, lhs: Expr, rhs: Expr) -> Self {
        Self {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

/// Assign operation
#[derive(Debug, Clone, PartialEq)]
pub struct AssignOp {
    /// Operator between the operands
    pub assign: Assign,

    /// Left operand
    pub lhs: Box<Expr>,

    /// Right operand
    pub rhs: Box<Expr>,
}

impl AssignOp {
    #[inline]
    pub fn new(assign: Assign, lhs: Expr, rhs: Expr) -> Self {
        Self {
            assign,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }
}

/// Function call
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    /// Function being called
    pub callee: Path,

    /// Call arguments
    pub args: Box<[Expr]>,
}

impl FunctionCall {
    #[inline]
    pub fn new(callee: Path, args: Vec<Expr>) -> Self {
        Self {
            callee,
            args: args.into_boxed_slice(),
        }
    }
}

/// Type cast using the `as` keyword
#[derive(Debug, Clone, PartialEq)]
pub struct TypeCast {
    /// Expression to cast
    pub expr: Box<Expr>,

    /// Primitive type to cast into
    pub prim: Primitive,
}

impl TypeCast {
    #[inline]
    pub fn new(expr: Expr, prim: Primitive) -> Self {
        Self {
            expr: Box::new(expr),
            prim,
        }
    }
}
