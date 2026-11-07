use crate::compiler::Opcode;
use crate::vm::inst::VMInst;

pub fn optimize(code: &mut Vec<VMInst>) {
    let old_len = code.len();
    if old_len == 0 {
        return;
    }
    let mut map = vec![0usize; old_len + 1];
    let mut out = Vec::with_capacity(old_len);
    let mut i = 0;
    while i < old_len {
        map[i] = out.len();
        if i + 1 < old_len {
            if let (VMInst::Op(Opcode::LoadVar(name)), VMInst::Op(Opcode::GetIndex)) =
                (&code[i], &code[i + 1])
            {
                out.push(VMInst::LoadVarGetIndex(*name));
                map[i + 1] = out.len() - 1;
                i += 2;
                continue;
            }
        }
        out.push(code[i].clone());
        i += 1;
    }
    map[old_len] = out.len();
    for j in 1..=old_len {
        if j < old_len && map[j] < map[j.saturating_sub(1)] {
            map[j] = map[j - 1];
        }
    }
    let out_len = out.len();
    for inst in &mut out {
        if let VMInst::Op(op) = inst {
            *op = op
                .clone()
                .remap_targets(|t| *map.get(t).unwrap_or(&out_len));
        }
    }
    *code = out;
}
