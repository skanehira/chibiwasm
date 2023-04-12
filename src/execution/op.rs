use super::{
    store::Store,
    value::{Label, LabelKind, StackAccess, Value},
};
use crate::{
    binary::instruction::Instruction, impl_binary_operation, impl_cvtop_operation,
    impl_unary_operation,
};
use anyhow::{bail, Context as _, Result};

pub fn local_get(locals: &mut [Value], stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value = locals.get(idx).context("not found local variable")?;
    stack.push(value.clone());
    Ok(())
}

pub fn local_set(locals: &mut [Value], stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    locals[idx] = value;
    Ok(())
}

pub fn local_tee(locals: &mut [Value], stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    stack.push(value.clone());
    stack.push(value);
    local_set(locals, stack, idx)?;
    Ok(())
}

pub fn global_set(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack
        .pop1()
        .with_context(|| "not found value in the stack")?;
    let mut global = store
        .globals
        .get(idx)
        .with_context(|| format!("not found global by index: {idx}"))?
        .borrow_mut();
    global.value = value;
    Ok(())
}

pub fn global_get(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let global = store
        .globals
        .get(idx)
        .with_context(|| format!("not found global by index: {idx}"))?;
    stack.push(global.borrow().value.clone());
    Ok(())
}

pub fn popcnt(stack: &mut impl StackAccess) -> Result<()> {
    let value = stack.pop1().context("not found value in the stack")?;

    match value {
        Value::I32(v) => {
            stack.push(v.count_ones() as i32);
        }
        Value::I64(v) => {
            stack.push(v.count_ones() as i64);
        }
        _ => bail!("unexpected value"),
    }
    Ok(())
}

pub fn i64extend_32s(stack: &mut impl StackAccess) -> Result<()> {
    let value = stack.pop1()?;
    match value {
        Value::I64(v) => {
            let result = v << 32 >> 32;
            let value: Value = result.into();
            stack.push(value);
        }
        _ => bail!("unexpected value type"),
    }
    Ok(())
}

pub fn get_end_address(insts: &[Instruction], pc: usize) -> Result<usize> {
    let mut pc = pc;
    let mut depth = 0;
    loop {
        pc += 1;
        let inst = insts.get(pc).expect("invalid end instruction");
        match inst {
            Instruction::If(_) | Instruction::Block(_) | Instruction::Loop(_) => depth += 1,
            Instruction::End => {
                if depth == 0 {
                    return Ok(pc);
                } else {
                    depth -= 1;
                }
            }
            _ => {
                // do nothing
            }
        }
    }
}

pub fn get_else_or_end_address(insts: &[Instruction], pc: usize) -> Result<usize> {
    let mut pc = pc;
    let mut depth = 0;
    loop {
        pc += 1;
        let inst = insts.get(pc).expect("invalid end instruction");
        match inst {
            Instruction::If(_) => {
                depth += 1;
            }
            Instruction::Else => {
                if depth == 0 {
                    return Ok(pc);
                }
            }
            Instruction::End => {
                if depth == 0 {
                    return Ok(pc);
                } else {
                    depth -= 1;
                }
            }
            _ => {
                // do nothing
            }
        }
    }
}

pub fn stack_unwind(stack: &mut Vec<Value>, sp: usize, arity: usize) {
    if arity > 0 {
        let value = stack.pop().expect("not found result value");
        stack.drain(sp..);
        stack.push(value);
    } else {
        stack.drain(sp..);
    }
}

pub fn br(labels: &mut Vec<Label>, stack: &mut Vec<Value>, level: &u32) -> Result<isize> {
    let label_index = labels.len() - 1 - (*level as usize);
    let Label {
        pc,
        start,
        sp,
        arity,
        kind,
    } = labels
        .get(label_index)
        .cloned()
        .expect("not found label when br");

    let pc = if kind == LabelKind::Loop {
        stack_unwind(stack, sp, arity);
        start.expect("not found start cp when loop")
    } else {
        labels.drain(label_index..);
        stack_unwind(stack, sp, arity);
        pc as isize
    };
    Ok(pc)
}

impl_unary_operation!(
    eqz, // itestop
    clz, ctz, extend8_s, extend16_s, // iunop
    abs, neg, sqrt, ceil, floor, trunc, nearest // funop
);

impl_binary_operation!(
    add, sub, mul, // binop
    div_s, div_u, rem_s, rem_u, and, or, xor, shl, shr_u, shr_s, rotl, rotr, // ibinop
    min, max, div, copysign, // fbinop
    equal, not_equal, // relop
    lt_s, lt_u, gt_s, gt_u, le_s, le_u, ge_s, ge_u, // irelop
    flt, fgt, fle, fge // frelop
);

impl_cvtop_operation!(
    i32_wrap_i64,
    i32_trunc_f32_s,
    i32_trunc_f32_u,
    i32_trunc_f64_s,
    i32_trunc_f64_u,
    i64_trunc_f32_s,
    i64_trunc_f32_u,
    i64_trunc_f64_s,
    i64_trunc_f64_u,
    i64_extend_i32_s,
    i64_extend_i32_u,
    f32_convert_i32_s,
    f32_convert_i32_u,
    f32_convert_i64_s,
    f32_convert_i64_u,
    f32_demote_f64,
    f64_convert_i32_s,
    f64_convert_i32_u,
    f64_convert_i64_s,
    f64_convert_i64_u,
    f64_demote_f32,
    i32_reinterpret_f32,
    i64_reinterpret_f64,
    f32_reinterpret_i32,
    f64_reinterpret_i64
);
