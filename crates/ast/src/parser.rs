/// Parsers related to defining memory locations
pub mod memory;

/// Parser for modules
pub mod module;

/// Parser for composite types (struct, enum, union)
pub mod composite_type;

/// Parser for expressions
pub mod expression;

/// Parser for statements
pub mod statement;

/// Parser for function header
pub mod function;

use crate::{
    lexer::token::Token,
    parser::{
        composite_type::{
            CompositeType, EnumType, Generics, TypeDecl, TypeUsed, parser_composite_type,
            parser_enum_type, parser_generics, parser_type_decl, parser_type_used,
        },
        expression::{Expr, parser_expr},
        function::{FuncDecl, parser_function},
        memory::{
            MemStorage, ParamDecl, ParamStorage, ParamType, ReturnType, VarDecl,
            parser_mem_storage, parser_param_decl, parser_param_storage, parser_param_type,
            parser_return_type, parser_var_decl,
        },
        module::{
            ModElement, Module, parser_module, parser_module_element, parser_module_file_based,
            parser_path,
        },
        statement::{Statement, parser_block, parser_statement},
    },
};
use asmanite_common::{ident::Ident, path::Path, span::Span};
use chumsky::{
    Boxed, Parser, error::Rich, extra, input::ValueInput, recursive::Recursive, span::SimpleSpan,
};

/// Convert a chumsky span into our own span type
#[inline]
fn convert_span(span: SimpleSpan) -> Span {
    Span::new(span.start, span.end)
}

/// Alias for boxed sub-parser
pub type SubParser<'tok, I, T> = Boxed<'tok, 'tok, I, T, extra::Err<Rich<'tok, Token>>>;

/// Set of all the sub-parsers of the language.
///
/// Each sub-parser is instantiated only once and shared (by cloning the boxed
/// parser) with every parser that depends on it. Mutually recursive parsers
/// (types, composite types and generics) are tied together here.
#[derive(Clone)]
pub struct Parsers<'tok, I>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    /// Parse a path to a module, variable, function, type, etc..
    pub path: SubParser<'tok, I, Path>,

    /// Expression without assignments
    pub expr_const: SubParser<'tok, I, Expr>,

    /// Expression allowing assignments
    pub expr: SubParser<'tok, I, Expr>,

    /// Parse a type being used (variable, parameter type)
    pub type_used: SubParser<'tok, I, TypeUsed>,

    /// Parse generics in a type or function
    pub generics: SubParser<'tok, I, Generics>,

    /// Parse a struct or union type
    pub composite_type: SubParser<'tok, I, CompositeType>,

    /// Parse an enumeration type
    pub enum_type: SubParser<'tok, I, EnumType>,

    /// Parse a type decleration (struct, union, enum or alias)
    pub type_decl: SubParser<'tok, I, TypeDecl>,

    /// Parse a memory storage specifier for a variable or function
    pub mem_storage: SubParser<'tok, I, MemStorage>,

    /// Parse a function parameter storage specifier
    pub param_storage: SubParser<'tok, I, ParamStorage>,

    /// Parse a variable declaration (var, static, const)
    pub var_decl: SubParser<'tok, I, VarDecl>,

    /// Parse a parameter type
    pub param_type: SubParser<'tok, I, ParamType>,

    /// Parse a parameter declaration (reg, mem, let, stack, etc..)
    pub param_decl: SubParser<'tok, I, ParamDecl>,

    /// Parse a return type
    pub return_type: SubParser<'tok, I, ReturnType>,

    /// Parse a function statement / instruction
    pub statement: SubParser<'tok, I, Statement>,

    /// Parse a block of code
    pub block: SubParser<'tok, I, Vec<Statement>>,

    /// Parse a function
    pub function: SubParser<'tok, I, FuncDecl>,

    /// Parse a module element
    pub module_element: SubParser<'tok, I, ModElement>,
}

impl<'tok, I> Parsers<'tok, I>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    /// Instantiate every sub-parser once and wire them together.
    pub fn new() -> Self {
        let path = parser_path().boxed();
        let expr_const = parser_expr(path.clone(), false).boxed();
        let expr = parser_expr(path.clone(), true).boxed();

        // Types, composite types and generics are mutually recursive
        let mut type_used_rec = Recursive::declare();
        let type_used: SubParser<'tok, I, TypeUsed> = type_used_rec.clone().boxed();
        let generics = parser_generics(type_used.clone()).boxed();
        let composite_type = parser_composite_type(type_used.clone(), generics.clone()).boxed();
        let enum_type = parser_enum_type(expr_const.clone()).boxed();
        type_used_rec.define(
            parser_type_used(
                expr_const.clone(),
                path.clone(),
                composite_type.clone(),
                enum_type.clone(),
            )
            .boxed(),
        );
        let type_decl = parser_type_decl(
            type_used.clone(),
            generics.clone(),
            composite_type.clone(),
            enum_type.clone(),
        )
        .boxed();

        let mem_storage = parser_mem_storage(expr_const.clone()).boxed();
        let param_storage = parser_param_storage(expr_const.clone()).boxed();
        let var_decl =
            parser_var_decl(expr.clone(), type_used.clone(), mem_storage.clone()).boxed();
        let param_type = parser_param_type(type_used.clone(), param_storage.clone()).boxed();
        let param_decl = parser_param_decl(expr_const.clone(), param_type.clone()).boxed();
        let return_type = parser_return_type(type_used.clone(), param_storage.clone()).boxed();

        let statement = parser_statement(
            expr_const.clone(),
            expr.clone(),
            var_decl.clone(),
            param_type.clone(),
        )
        .boxed();
        let block = parser_block(statement.clone()).boxed();
        let function = parser_function(
            generics.clone(),
            block.clone(),
            mem_storage.clone(),
            param_decl.clone(),
            return_type.clone(),
        )
        .boxed();
        let module_element =
            parser_module_element(type_decl.clone(), var_decl.clone(), function.clone()).boxed();

        Self {
            path,
            expr_const,
            expr,
            type_used,
            generics,
            composite_type,
            enum_type,
            type_decl,
            mem_storage,
            param_storage,
            var_decl,
            param_type,
            param_decl,
            return_type,
            statement,
            block,
            function,
            module_element,
        }
    }

    /// Parser for an inline module block `mod name { ... }`
    pub fn module(
        &self,
    ) -> impl Parser<'tok, I, Module, extra::Err<Rich<'tok, Token>>> + use<'tok, I> {
        parser_module(self.module_element.clone())
    }

    /// Parser for the contents of a whole module source file
    pub fn module_file(
        &self,
        ident: Ident,
    ) -> impl Parser<'tok, I, Module, extra::Err<Rich<'tok, Token>>> + use<'tok, I> {
        parser_module_file_based(ident, self.module_element.clone())
    }
}

impl<'tok, I> Default for Parsers<'tok, I>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Parsers;
    use crate::lexer::{init_test_registers, token::Token};
    use asmanite_common::ident::Ident;
    use chumsky::{
        Parser,
        input::{Input as _, Stream},
    };
    use logos::Logos as _;

    #[test]
    fn parses_a_complex_module() {
        let source = r#"
use root::runtime::{Buffer, helpers::*};

struct Packet<T, const SIZE: u8> {
    tag: u8,
    data: [T; SIZE],
    callback: fn(u8) -> u8,
}

union WordBytes {
    word: u16,
    bytes: [u8; 2],
}

enum State as u16 {
    Ready = 0,
    Busy = 1,
    Failed,
}

mod my_module {
    type Handler<T> = fn(T) -> u8;
    type Pair = (u8, u16);
}

const LIMIT: u8 = 10;
const MARKER: u8 = 'A';
static DEVICE: u8 at 0x0010 = 0;
static RATIO: f32 = 1.5;
var packet: Packet<u8, 4> in ram;

inline fn transform<T, const SIZE: u8> in code (
    input: u8 in ram,
    a,
    stack scratch: u8,
    let flags: u8 = 0,
) -> u8 in ram {
    use self::helpers::*;

    'entry: var local: u8 at 0x20 = input + 1;
    in scratch {
        var temporary: u8;
    }

    log("ready");
    a = helper::convert(input, 2) + (local * -3) as u8;
    if ready && local != 0 {
        a |= local;
    } else if false {
        a = 1;
    } else {
        a ^= 0x1;
    }

    while local < LIMIT {
        local += 1;
        if local == 5 {
            continue 'entry;
        }
    }
    do {
        local -= 1;
    } while local > 0;
    for index: u8 in ram of 0..=4 step 2 {
        local += index;
    }
    for x of 0..3 {
        a = x;
    }
    loop {
        break;
    }

    switch state {
        0 => { local = 0; }
        _ => { local = 1; }
    }
    defer { local = 0; }
    goto 'finish;
    'finish: return local;
}

recursive fn recurse(value: u8 at 0x30 = 1) -> x {
    if value > 0 {
        return recurse(value - 1);
    }
    return;
}
        "#;

        init_test_registers();
        let tokens = Token::lexer(source)
            .spanned()
            .map(|(token, span)| (token.expect("module source should lex"), span.into()))
            .collect::<Vec<_>>();
        let input =
            Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));

        let module = Parsers::new().module_file(Ident::new("my_module"));

        module
            .parse(input)
            .into_result()
            .expect("the complete module source should parse");
    }
}
