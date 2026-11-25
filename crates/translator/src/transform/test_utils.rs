//! Helpers shared by the unit tests of the translator

use crate::transform::transform;
use asmanite_ast::parser::module::Module;
use asmanite_ast::{
    chumsky::{
        Parser as _,
        input::{Input as _, Stream},
    },
    lexer::token::Token,
    logos::Logos as _,
    parser::Parsers,
};
use asmanite_common::{ToBox, ident::Ident, operator::Operator, path::Path};
use asmanite_common::{
    primitive::def::U8,
    register::{REGISTERS, RegDef, RegIdent, RegSet},
};
use asmanite_ir::LabelId;
use asmanite_ir::intermediate::{
    Condition, IrFunc, IrProgram, JumpTarget, Label, OpBinary, OpBranch, OpCode, OpCompare,
    OpTransfer, Operand,
};

pub fn parse(source: &str) -> Module {
    REGISTERS.get_or_init(|| {
        RegSet::new(&[
            ("a", RegDef::new(U8)),
            ("x", RegDef::new(U8)),
            ("y", RegDef::new(U8)),
        ])
    });
    let tokens: Vec<_> = Token::lexer(source)
        .spanned()
        .map(|(token, span)| (token.expect("test source should lex"), span.into()))
        .collect();
    let input =
        Stream::from_iter(tokens).map((0..source.len()).into(), |(token, span)| (token, span));
    Parsers::new()
        .module_file(Ident::new("main"))
        .parse(input)
        .into_result()
        .expect("test source should parse")
}

pub fn translate(source: &str) -> IrProgram {
    transform(&parse(source)).expect("test source should translate")
}

pub fn function<'p>(program: &'p IrProgram, name: &str) -> &'p IrFunc {
    program
        .functions
        .iter()
        .find(|func| func.name == Path::from(name))
        .expect("function should exist")
}

pub fn reg(name: &str) -> Operand {
    Operand::Reg(RegIdent::new(name))
}

pub fn mem(path: &str) -> Operand {
    Operand::mem(Path::from(path))
}

pub fn label(id: LabelId) -> OpCode {
    OpCode::Label(Label::Local(id))
}

pub fn jump(id: LabelId) -> OpCode {
    OpCode::Jump(JumpTarget::Label(Label::Local(id)))
}

pub fn transfer(dst: Operand, src: Operand) -> OpCode {
    OpCode::Transfer(OpTransfer { dst, src }.to_box())
}

pub fn branch_cmp(op: Operator, lhs: Operand, rhs: Operand, target: LabelId) -> OpCode {
    OpCode::Branch(
        OpBranch {
            cond: Condition::Compare(OpCompare { op, lhs, rhs }.to_box()),
            target: JumpTarget::Label(Label::Local(target)),
        }
        .to_box(),
    )
}

pub fn binary(op: Operator, dst: Operand, lhs: Operand, rhs: Operand) -> OpCode {
    OpCode::Binary(OpBinary { op, dst, lhs, rhs }.to_box())
}
