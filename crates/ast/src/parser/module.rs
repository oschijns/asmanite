use crate::{
    lexer::{
        atom::delimiter,
        token::{IdentOrReg, Token},
    },
    parser::{composite_type::TypeDecl, function::FuncDecl, memory::VarDecl},
};
use asmanite_common::{ToBox, ident::Ident, operator::op, path::Path};
use chumsky::{
    IterParser as _, Parser, error::Rich, extra, input::ValueInput, primitive::just,
    recursive::recursive, select, span::SimpleSpan,
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
    /// Use statement
    Use(Box<UseNode>),

    /// Type declaration
    Type(Box<TypeDecl>),

    /// Variable declaration
    Var(Box<VarDecl>),

    /// Function declaration
    Func(Box<FuncDecl>),
}

/// A node in a `use` statement
#[derive(Debug, Clone, PartialEq)]
pub struct UseNode {
    /// Name of the node
    pub element: UseElem,

    /// Children of this node
    pub children: Option<Box<[UseNode]>>,
}

impl UseNode {
    /// Create a leaf node in a `use` statement
    #[inline]
    pub fn new(element: UseElem) -> Self {
        Self {
            element,
            children: None,
        }
    }

    /// Add children nodes to this one
    #[inline]
    pub fn with_children(&mut self, children: Vec<UseNode>) -> &Self {
        self.children = Some(children.into_boxed_slice());
        self
    }
}

impl ToBox for UseNode {}

/// An element in the path
#[derive(Debug, Clone, PartialEq)]
pub enum UseElem {
    /// A generic identifier
    Ident(Ident),

    /// Root of the project
    Root,

    /// This module
    Self_,

    /// All symbols '*'
    All,
}

/// Open brace token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close brace token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

/// Parse a sequence of identifiers and path separator into a path
pub fn parser_path<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, Path, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    // collect a sequence of identifiers separated by `::`
    select! {
        Token::Ident(IdentOrReg::Ident(ident)) => ident,
    }
    .separated_by(just(Token::PathSep))
    .at_least(1)
    .collect::<Vec<_>>()
    .map(Path::new)
}

/// Parse a `use` statement into its path tree.
///
/// Both `path{...}` and Rust-style `path::{...}` groups are accepted.
pub fn parser_use<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, UseNode, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::Use).ignore_then(
        recursive(|use_tree| {
            let group = use_tree
                .clone()
                .or(just(Token::Operator(op::MUL)).to(UseNode::new(UseElem::All)))
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE));

            let element = select! {
                Token::Ident(IdentOrReg::Ident(ident)) => UseElem::Ident(ident),
                Token::Root => UseElem::Root,
                Token::Self_ => UseElem::Self_,
            };

            let children = group
                .clone()
                .or(just(Token::PathSep).ignore_then(
                    group
                        .clone()
                        .or(just(Token::Operator(op::MUL)).to(vec![UseNode::new(UseElem::All)]))
                        .or(use_tree.clone().map(|child| vec![child])),
                ))
                .or_not();

            element.then(children).map(|(element, children)| {
                let mut node = UseNode::new(element);
                if let Some(children) = children {
                    node.with_children(children);
                }
                node
            })
        })
        .then_ignore(just(Token::Semicolon)),
    )
}

#[cfg(test)]
mod tests {
    use crate::lexer::init_test_registers;

    use super::*;
    use chumsky::input::{Input, Stream};
    use logos::Logos;

    fn parse_use(source: &str) -> Result<UseNode, ()> {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        parser_use().parse(input).into_result().map_err(|_| ())
    }

    fn node(element: UseElem, children: Vec<UseNode>) -> UseNode {
        UseNode {
            element,
            children: (!children.is_empty()).then(|| children.into_boxed_slice()),
        }
    }

    fn ident(name: &str, children: Vec<UseNode>) -> UseNode {
        node(UseElem::Ident(Ident::new(name)), children)
    }

    #[test]
    fn parses_grouped_and_wildcard_use_statements() {
        assert_eq!(
            parse_use("use path::to::my{MyType, my_func, MY_VAR};").unwrap(),
            ident(
                "path",
                vec![ident(
                    "to",
                    vec![ident(
                        "my",
                        vec![
                            ident("MyType", vec![]),
                            ident("my_func", vec![]),
                            ident("MY_VAR", vec![]),
                        ],
                    )],
                )],
            )
        );

        assert_eq!(
            parse_use("use root::other{self, OTHER};").unwrap(),
            node(
                UseElem::Root,
                vec![ident(
                    "other",
                    vec![node(UseElem::Self_, vec![]), ident("OTHER", vec![]),],
                )],
            )
        );

        assert_eq!(
            parse_use("use root::last::*;").unwrap(),
            node(
                UseElem::Root,
                vec![ident("last", vec![node(UseElem::All, vec![])],)],
            )
        );
    }

    #[test]
    fn parses_rust_style_nested_groups() {
        assert_eq!(
            parse_use("use root::foo::{bar::Type, other::*};").unwrap(),
            node(
                UseElem::Root,
                vec![ident(
                    "foo",
                    vec![
                        ident("bar", vec![ident("Type", vec![])]),
                        ident("other", vec![node(UseElem::All, vec![])],),
                    ],
                )],
            )
        );
    }
}
