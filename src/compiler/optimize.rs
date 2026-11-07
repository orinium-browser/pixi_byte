use crate::compiler::{BytecodeChunk, Opcode};
use crate::value::JSValue;
use crate::value::jsvalue::JsValueKind;
use num_bigint::BigInt;
use num_traits::ToPrimitive;

pub fn optimize(chunk: &mut BytecodeChunk) {
    for _ in 0..2 {
        let mut changed = false;
        changed |= fold_constants(chunk);
        changed |= remove_dup_pop(chunk);
        if !changed {
            break;
        }
    }
}

fn fold_constants(chunk: &mut BytecodeChunk) -> bool {
    let old_len = chunk.code.len();
    if old_len < 3 {
        return false;
    }
    let mut map = vec![0usize; old_len + 1];
    let mut out = Vec::with_capacity(old_len);
    let mut changed = false;
    let mut i = 0;
    while i < old_len {
        map[i] = out.len();
        if i + 2 < old_len {
            if let (Opcode::LoadConst(a), Opcode::LoadConst(b)) =
                (chunk.code[i].clone(), chunk.code[i + 1].clone())
            {
                if let Some(folded) =
                    fold_binary_const(&chunk.constants[a], &chunk.constants[b], &chunk.code[i + 2])
                {
                    let idx = chunk.add_constant(folded);
                    out.push(Opcode::LoadConst(idx));
                    map[i + 1] = out.len() - 1;
                    map[i + 2] = out.len() - 1;
                    i += 3;
                    changed = true;
                    continue;
                }
            }
        }
        out.push(chunk.code[i].clone());
        i += 1;
    }
    if !changed {
        return false;
    }
    map[old_len] = out.len();
    for j in 1..=old_len {
        if j < old_len && map[j] < map[j.saturating_sub(1)] {
            map[j] = map[j - 1];
        }
    }
    let out_len = out.len();
    for op in &mut out {
        *op = op
            .clone()
            .remap_targets(|t| *map.get(t).unwrap_or(&out_len));
    }
    chunk.code = out;
    true
}

fn fold_binary_const(a: &JSValue, b: &JSValue, op: &Opcode) -> Option<JSValue> {
    if a.is_number() && b.is_number() {
        let (an, bn) = (a.as_number().unwrap(), b.as_number().unwrap());
        let result = match op {
            Opcode::Add => an + bn,
            Opcode::Sub => an - bn,
            Opcode::Mul => an * bn,
            Opcode::Div => an / bn,
            Opcode::Mod => an % bn,
            Opcode::Power => an.powf(bn),
            _ => return None,
        };
        return Some(JSValue::from_number(result));
    }
    if matches!(op, Opcode::Add) {
        if let (Some(sa), Some(sb)) = (a.as_string(), b.as_string()) {
            let mut out = String::with_capacity(sa.len() + sb.len());
            out.push_str(sa);
            out.push_str(sb);
            return Some(JSValue::from_string(out));
        }
    }
    if let (JsValueKind::BigInt, JsValueKind::BigInt) = (a.kind(), b.kind()) {
        let ba = a.as_bigint().unwrap();
        let bb = b.as_bigint().unwrap();
        let result = match op {
            Opcode::Add => ba + bb,
            Opcode::Sub => ba - bb,
            Opcode::Mul => ba * bb,
            Opcode::Div => {
                if *bb == BigInt::from(0) {
                    return None;
                }
                ba / bb
            }
            Opcode::Mod => {
                if *bb == BigInt::from(0) {
                    return None;
                }
                ba % bb
            }
            Opcode::Power => {
                let exp = bb.to_u32()?;
                ba.pow(exp)
            }
            _ => return None,
        };
        return Some(JSValue::from_bigint(result));
    }
    None
}

fn remove_dup_pop(chunk: &mut BytecodeChunk) -> bool {
    let old_len = chunk.code.len();
    if old_len < 2 {
        return false;
    }
    let mut keep = vec![true; old_len];
    let mut changed = false;
    let mut i = 0;
    while i + 1 < old_len {
        if matches!(chunk.code[i], Opcode::Dup) && matches!(chunk.code[i + 1], Opcode::Pop) {
            keep[i] = false;
            keep[i + 1] = false;
            changed = true;
            i += 2;
        } else {
            i += 1;
        }
    }
    if !changed {
        return false;
    }
    let mut map = vec![0usize; old_len + 1];
    let mut out = Vec::with_capacity(old_len);
    for (idx, op) in chunk.code.iter().enumerate() {
        if keep[idx] {
            map[idx] = out.len();
            out.push(op.clone());
        } else {
            map[idx] = out.len();
        }
    }
    map[old_len] = out.len();
    let mut next = out.len();
    for idx in (0..old_len).rev() {
        if keep[idx] {
            next = map[idx];
        } else {
            map[idx] = next;
        }
    }
    let out_len = out.len();
    for op in &mut out {
        *op = op
            .clone()
            .remap_targets(|t| *map.get(t).unwrap_or(&out_len));
    }
    chunk.code = out;
    true
}
