/*
 * TODO:
 * https://codeberg.org/zesterer/chumsky/src/branch/main/examples/logos.rs
 *
 * identify scopes/groups using pair of delimiters (), [], {}
 *
 */

use crate::token::{define::*, lexer::Token};
use chumsky::{input::ValueInput, prelude::*};

/// Expression
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal
    Lit(Literal),

    /// Operator
    Op(Operator),

    /// Identifier
    Ident(Ident),

    /// Sub expression
    Group {
        /// Delimitiers of the group
        delim_type: DelimType,

        /// Content of the group
        inner: Box<[Self]>,
    },
}

fn parser<'tok, I>() -> impl Parser<'tok, I, Expr, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    recursive(|sexpr| {
        let atom = select! {
            Token::BoolLit (l) => Expr::Lit(Literal::Bool (l)),
            Token::CharLit (l) => Expr::Lit(Literal::Char (l)),
            Token::IntLit  (l) => Expr::Lit(Literal::Int  (l)),
            Token::FloatLit(l) => Expr::Lit(Literal::Float(l)),
            Token::StrLit  (l) => Expr::Lit(Literal::Str  (l)),
            Token::Ident   (i) => Expr::Ident(i),
            Token::Operator(o) => Expr::Op(o),
        };

        let list = sexpr
            .repeated()
            .collect()
            .map(|g| Expr::Group {
                delim_type: DelimType::Paren,
                inner: g,
            })
            .delimited_by(
                just(Token::Delimiter(Delimiter {
                    delim_type: DelimType::Paren,
                    is_close: false,
                })),
                just(Token::Delimiter(Delimiter {
                    delim_type: DelimType::Paren,
                    is_close: true,
                })),
            );

        atom.or(list)
    })
}
