use super::convert_span;
use crate::{
    lexer::{
        atom::delimiter,
        token::{
            IdentOrReg,
            Token::{self},
        },
    },
    parser::{SubParser, composite_type::TypeDecl, function::FuncDecl, memory::VarDecl},
};
use asmanite_common::{
    ToBox,
    ident::Ident,
    operator::op,
    path::Path,
    span::{Span, Spanned, WithSpan},
};
use chumsky::{
    IterParser as _, Parser,
    error::Rich,
    extra,
    input::ValueInput,
    primitive::{end, just},
    recursive::recursive,
    select,
    span::SimpleSpan,
};

// MARK: AST

/// Define a module in the program
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    /// Span of the module
    pub span: Span,

    /// Module identifier
    pub ident: Ident,

    /// Variables, functions and types declared in the module.
    pub elements: Box<[ModElement]>,
}

impl Module {
    #[inline]
    pub fn new(span: Span, ident: Ident, elements: Vec<ModElement>) -> Self {
        Self {
            span,
            ident,
            elements: elements.into_boxed_slice(),
        }
    }
}

impl ToBox for Module {}

impl Spanned for Module {
    #[inline]
    fn span(&self) -> Span {
        self.span
    }
}

/// Element in a module
/// Can be either a variable, function or type
#[derive(Debug, Clone, PartialEq)]
pub enum ModElement {
    /// Module declaration referencing another file
    ModDecl(WithSpan<Ident>),

    /// Sub module in this module
    Mod(Box<Module>),

    /// Use statement
    Use(Box<WithSpan<UseNode>>),

    /// Type declaration
    Type(TypeDecl),

    /// Variable declaration
    Var(Box<VarDecl>),

    /// Function declaration
    Func(Box<FuncDecl>),
}

impl Spanned for ModElement {
    #[rustfmt::skip]
    fn span(&self) -> Span {
        match self {
            Self::ModDecl(md  ) => md  .span,
            Self::Mod    (md  ) => md  .span,
            Self::Use    (u   ) => u   .span,
            Self::Type   (ty  ) => ty  .span(),
            Self::Var    (var ) => var .span,
            Self::Func   (func) => func.span,
        }
    }
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

// MARK: Parsers

/// Open brace token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close brace token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

/// Parser for an inline module block.
///
/// This parser consumes `mod name { ... }` and can be embedded in a module.
pub fn parser_module<'tok, 'src: 'tok, I>(
    parse_mod_elem: SubParser<'tok, I, ModElement>,
) -> impl Parser<'tok, I, Module, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::Mod)
        .ignore_then(select! {
            Token::Ident(IdentOrReg::Ident(ident)) => ident,
        })
        .then(
            parse_mod_elem
                .repeated()
                .collect::<Vec<_>>()
                .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE))
                .or(just(Token::Semicolon).to(Vec::new())),
        )
        .map_with(|(ident, elements), extra| {
            Module::new(convert_span(extra.span()), ident, elements)
        })
}

/// Parser for the contents of a module source file.
///
/// Unlike [`parser_module`], this parser does not consume `mod name { ... }`.
/// It consumes all module elements and requires the end of the input.
pub fn parser_module_file_based<'tok, 'src: 'tok, I>(
    ident: Ident,
    parse_mod_elem: SubParser<'tok, I, ModElement>,
) -> impl Parser<'tok, I, Module, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    parse_mod_elem
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map_with(move |elements, extra| {
            Module::new(convert_span(extra.span()), ident.clone(), elements)
        })
}

/// Parser for an element of a module
pub fn parser_module_element<'tok, 'src: 'tok, I>(
    parse_type: SubParser<'tok, I, TypeDecl>,
    parse_var: SubParser<'tok, I, VarDecl>,
    parse_func: SubParser<'tok, I, FuncDecl>,
) -> impl Parser<'tok, I, ModElement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|element| {
        // `mod name { ... }` defines an inline module,
        // `mod name;` declares a module defined in another file.
        let module = just(Token::Mod)
            .ignore_then(select! {
                Token::Ident(IdentOrReg::Ident(ident)) => ident,
            })
            .then(
                element
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .delimited_by(just(BRACE_OPEN), just(BRACE_CLOSE))
                    .map(Some)
                    .or(just(Token::Semicolon).to(None)),
            )
            .map_with(|(ident, elements), extra| {
                let span = convert_span(extra.span());
                match elements {
                    Some(elements) => ModElement::Mod(Module::new(span, ident, elements).to_box()),
                    None => ModElement::ModDecl(WithSpan::new(span, ident)),
                }
            });

        module
            .or(parser_use().map(|u| ModElement::Use(u.to_box())).boxed())
            .or(parse_type.map(ModElement::Type).boxed())
            .or(parse_var.map(|var| ModElement::Var(var.to_box())).boxed())
            .or(parse_func
                .map(|func| ModElement::Func(func.to_box()))
                .boxed())
    })
}

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
-> impl Parser<'tok, I, WithSpan<UseNode>, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::Use)
        .ignore_then(
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
        .map_with(|node, extra| WithSpan::new(convert_span(extra.span()), node))
}

#[cfg(test)]
mod tests {
    use crate::lexer::init_test_registers;
    use crate::parser::Parsers;

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

        parser_use()
            .map(|node| node.atom)
            .parse(input)
            .into_result()
            .map_err(|_| ())
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

    fn parse_module(source: &str) -> Result<Module, ()> {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        Parsers::new()
            .module()
            .parse(input)
            .into_result()
            .map_err(|_| ())
    }

    fn parse_module_file(source: &str) -> Result<Module, ()> {
        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("test source should lex"), span.into()));
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        Parsers::new()
            .module_file(Ident::new("example"))
            .parse(input)
            .into_result()
            .map_err(|_| ())
    }

    #[test]
    fn parses_nested_and_empty_module_blocks() {
        let module = parse_module("mod outer { mod inner {} }").unwrap();
        assert_eq!(module.ident, Ident::new("outer"));
        assert_eq!(module.elements.len(), 1);
        assert!(matches!(module.elements[0], ModElement::Mod(_)));

        let module = parse_module("mod outer {}").unwrap();
        assert!(module.elements.is_empty());

        let module = parse_module("mod outer;").unwrap();
        assert_eq!(module.ident, Ident::new("outer"));
        assert!(module.elements.is_empty());
    }

    #[test]
    fn parses_a_whole_module_source_file() {
        let module = parse_module_file("const VALUE: u8 = 1;").unwrap();
        assert_eq!(module.ident, Ident::new("example"));
        assert_eq!(module.elements.len(), 1);
        assert!(matches!(module.elements[0], ModElement::Var(_)));

        let empty = parse_module_file("").unwrap();
        assert_eq!(empty.ident, Ident::new("example"));
        assert!(empty.elements.is_empty());
    }

    #[test]
    fn distinguishes_module_declarations_from_inline_modules() {
        let source = "mod external; mod local { mod nested; }";
        let module = parse_module_file(source).unwrap();
        assert_eq!(module.elements.len(), 2);

        let ModElement::ModDecl(decl) = &module.elements[0] else {
            panic!("expected module declaration");
        };
        assert_eq!(decl.atom, Ident::new("external"));
        assert_eq!(decl.span, Span::new(0, "mod external;".len()));

        let ModElement::Mod(inline) = &module.elements[1] else {
            panic!("expected inline module");
        };
        assert_eq!(inline.ident, Ident::new("local"));
        assert_eq!(inline.elements.len(), 1);
        assert!(matches!(
            &inline.elements[0],
            ModElement::ModDecl(decl) if decl.atom == Ident::new("nested")
        ));
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
