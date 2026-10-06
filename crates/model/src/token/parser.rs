/*
 * TODO:
 * https://codeberg.org/zesterer/chumsky/src/branch/main/examples/logos.rs
 *
 * identify scopes/groups using pair of delimiters (), [], {}
 *
 */

use crate::token::{define::*, lexer::Token};
use tokora::{
    parser::{PrattLHS, PrattRHS},
    token::PrattToken,
};

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

impl PrattToken<'_, f64, Precedence> for Token {
    fn try_pratt_lhs(&self) -> Option<PrattLHS<(), (), Precedence>> {
        Some(match self {
            //Token::Num(_) => PrattLHS::Operand(()),
            //Token::Minus => PrattLHS::Prefix(Precedenced::new((), PREC_NEG)),
            //// `(` is a prefix with PREC_PAREN; the recursive pratt_in starts at
            //// min_power = PREC_PAREN so that `)` (postfix with the same power) is
            //// consumed inside that call rather than by the outer loop.
            //Token::LParen => PrattLHS::Prefix(Precedenced::new((), PREC_PAREN)),
            _ => return None,
        })
    }

    fn try_pratt_rhs(&self) -> Option<PrattRHS<(), (), (), (), Precedence>> {
        Some(match self {
            //Token::Plus => PrattRHS::Infix(Precedenced::new(PrattInfix::Left(()), PREC_SUM)),
            //Token::Minus => PrattRHS::Infix(Precedenced::new(PrattInfix::Left(()), PREC_SUM)),
            //Token::Star => PrattRHS::Infix(Precedenced::new(PrattInfix::Left(()), PREC_PROD)),
            //Token::Slash => PrattRHS::Infix(Precedenced::new(PrattInfix::Left(()), PREC_PROD)),
            //Token::Caret => PrattRHS::Infix(Precedenced::new(PrattInfix::Right(()), PREC_EXP)),
            //// `)` is a postfix with PREC_PAREN; consumed only when min_power <= PREC_PAREN.
            //Token::RParen => PrattRHS::Postfix(Precedenced::new((), PREC_PAREN)),
            _ => return None,
        })
    }
}
