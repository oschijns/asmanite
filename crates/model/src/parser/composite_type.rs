use crate::lexer::{atom::*, token::Token};
use chumsky::{Parser, error::Rich, extra, input::ValueInput, select, span::SimpleSpan};

// MARK: AST

/// Type used by a variable
#[derive(Debug, Clone, PartialEq)]
pub struct TypeUsed {
    // TODO: for now we only have primitives
    pub primitive: Primitive,
}

// MARK: Parsers

/// Open '<' token
const GEN_OPEN: Token = Token::Operator(Operator::new(OpType::Lt));

/// Close '>' token
const GEN_CLOSE: Token = Token::Operator(Operator::new(OpType::Gt));

/// Parse a type usage.
pub fn parser_type_used<'tok, 'src: 'tok, I>()
-> impl Parser<'tok, I, TypeUsed, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    select! { Token::Primitive(primitive) => TypeUsed{ primitive } }
}

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
}
