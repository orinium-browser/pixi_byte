use crate::compiler::Opcode;
use crate::intern::NameId;

#[derive(Debug, Clone, PartialEq)]
pub enum VMInst {
    Op(Opcode),
    LoadVarGetIndex(NameId),
}

impl VMInst {
    pub fn from_opcode(op: &Opcode) -> Self {
        VMInst::Op(op.clone())
    }
}
