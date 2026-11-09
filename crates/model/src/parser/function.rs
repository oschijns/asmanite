use crate::lexer::{atom::*, token::Token};

// MARK: AST

// MARK: Parsers

/// Open brace token
const BRACE_OPEN: Token = Token::Delimiter(delimiter::BRACE_OPEN);

/// Close brace token
const BRACE_CLOSE: Token = Token::Delimiter(delimiter::BRACE_CLOSE);

// MARK: Tests

#[cfg(test)]
mod tests {
    use super::*;
}
