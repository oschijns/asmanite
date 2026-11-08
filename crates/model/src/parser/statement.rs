use crate::{lexer::atom::*, parser::expression::Expr};

// MARK: AST

/// Statement AST
/// Instruction that can appear in a block of code
#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    /// Variable declaration
    Var(VarDecl),

    /// If-else chain
    If(IfElse),
}

/// Parameter declaration
/// Handle variable declaration for function parameters and for-loop iterator
/// The `var` keyword is omitted and a register can be used instead
#[derive(Debug, Clone, PartialEq)]
pub enum ParamDecl {
    /// Declare a variable with a memory location
    Var(VarDecl),

    /// Use a register directly
    Register {
        /// Register to use
        register: Ident,

        /// Initialize the register with a value
        init_expr: Option<Box<Expr>>,
    },
}

/// Variable declaration.
/// This structure can be used in various context:
/// - Outside of functions for constants and static variables
/// - Inside of functions' bodies
/// - As function parameters (`decl_type` is then omitted)
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

/// If-else branching chain
#[derive(Debug, Clone, PartialEq)]
pub struct IfElse {
    /// Branches in the if-else chain
    /// Atleast one must be present and only the last one may omit a condition
    pub branches: Box<[Branching]>,
}

/// Switch statement
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    /// Variable or register to check
    pub variable: Box<Expr>,

    /// Cases to compare with
    /// Atleast one must be present and only the last one may omit a condition
    pub cases: Box<[Branching]>,
}

/// Branching element
#[derive(Debug, Clone, PartialEq)]
pub struct Branching {
    /// Condition / case
    /// If none provided, default to true
    pub condition: Option<Box<Expr>>,

    /// Block of code to execute if the condition is true
    pub block: Box<[Statement]>,
}

/// Loop
#[derive(Debug, Clone, PartialEq)]
pub struct Loop {
    /// The type of loop
    pub loop_type: LoopType,

    /// Block of code to execute as long as the condition is true
    pub block: Box<[Statement]>,
}

/// Define the type of loop
#[derive(Debug, Clone, PartialEq)]
pub enum LoopType {
    /// Common loop without any conditions
    Loop,

    /// While loop
    While(WhileLoopHeader),

    /// For loop
    /// A variable is initialized and incremented / decremented while iterating over the loop
    For(ForLoopHeader),
}

/// Header of a while loop
#[derive(Debug, Clone, PartialEq)]
pub struct WhileLoopHeader {
    /// Condition to stop iterating
    pub condition: Box<Expr>,

    /// Specify if the condition is checked only after a first iteration
    pub do_while: bool,
}

/// Header of a for loop
#[derive(Debug, Clone, PartialEq)]
pub struct ForLoopHeader {
    /// Initialize the iterator
    pub init: ParamDecl,

    /// Condition to stop iterating
    pub condition: Box<Expr>,

    /// Increment or decrement the iterator by the given step
    pub step: i64,
}

// MARK: Parsers

// MARK: Notes

// TODO:
// if-else chains
// loops (loop, for, while, do-while)
// switch
// return       Return from a function
// break        Exit a loop or switch
// continue     Skip to the next loop iteration
// fallthrough  Fall to next switch case
// goto         Jump to a label
// 'label       Define jump destinations
// defer
// asm

/*
    consider macro block for inserting raw assembly or calling a scripting language ?

    #[macro]
    {%
        ...
    %}

    #[asm]
    {{
        ...
    }}

    #[macro]
    (*
        ...
    *)

    #[asm]
    #{
        ...
    }#
*/
