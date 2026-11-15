use crate::{
    ToBox,
    lexer::{
        atom::*,
        register::RegIdent,
        token::{IdentOrReg, Token},
    },
    parser::{
        composite_type::{TypeDecl, TypeUsed, parser_type_used},
        expression::{Expr, parser_expr},
        function::{FuncDecl, parser_function},
        memory::VarDecl,
    },
};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, primitive::just, select,
    span::SimpleSpan,
};

// MARK: AST

/// Define a module in the program
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Module identifier
    pub ident: Ident,

    /// Variables, functions and types declared in the module.
    pub elements: Box<[ModElement]>,
}

impl Module {
    #[inline]
    pub fn new(ident: Ident, elements: Vec<ModElement>) -> Self {
        Self {
            ident,
            elements: elements.into_boxed_slice(),
        }
    }
}

impl ToBox for Module {}

/// Element in a module
/// Can be either a variable, function or type
#[derive(Debug, Clone, PartialEq)]
pub enum ModElement {
    /// Variable declaration
    Var(Box<VarDecl>),

    /// Function declaration
    Func(Box<FuncDecl>),

    /// Type declaration
    Type(Box<TypeDecl>),
}
