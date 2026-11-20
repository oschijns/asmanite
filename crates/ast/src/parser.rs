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

use asmanite_common::span::Span;

/// Convert a chumsky span into our own span type
#[inline]
fn convert_span(span: chumsky::span::SimpleSpan) -> Span {
    Span::new(span.start, span.end)
}

#[cfg(test)]
mod tests {
    use crate::{
        lexer::{init_test_registers, token::Token},
        parser::module::parser_module_file_named,
    };
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
        fn nested() {
            return;
        }
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

        let module = parser_module_file_named(Ident::new("my_module"));

        module
            .parse(input)
            .into_result()
            .expect("the complete module source should parse");
    }
}
