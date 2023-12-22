use super::{
    module::{ExternalFuncInst, InternalFuncInst},
    store::Store,
    value::{Frame, Label, LabelKind, StackAccess, Value},
};
use crate::{
    binary::{instruction::Instruction, types::ValueType},
    execution::error::Error,
    impl_binary_operation, impl_cvtop_operation, impl_unary_operation,
};
use anyhow::{bail, Context as _, Result};
use log::trace;
use std::{cell::RefCell, rc::Rc};

pub fn local_get(locals: &[Value], stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value = locals
        .get(idx)
        .with_context(|| Error::NotFoundLocalVariable(idx))?;
    stack.push(value.clone());
    Ok(())
}

pub fn local_set(locals: &mut Vec<Value>, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    if locals.len() <= idx {
        for _ in 0..(idx + 1) - locals.len() {
            locals.push(0.into());
        }
    }
    locals[idx] = value;
    Ok(())
}

pub fn local_tee(locals: &mut Vec<Value>, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1()?;
    stack.push(value.clone());
    stack.push(value);
    local_set(locals, stack, idx)?;
    Ok(())
}

pub fn global_set(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let value: Value = stack.pop1().with_context(|| Error::StackPopError)?;
    let mut global = store
        .globals
        .get(idx)
        .with_context(|| Error::NotFoundGlobalVariable(idx))?
        .borrow_mut();
    global.value = value;
    Ok(())
}

pub fn global_get(store: &mut Store, stack: &mut impl StackAccess, idx: usize) -> Result<()> {
    let global = store
        .globals
        .get(idx)
        .with_context(|| Error::NotFoundGlobalVariable(idx))?;
    stack.push(global.borrow().value.clone());
    Ok(())
}

pub fn popcnt(stack: &mut impl StackAccess) -> Result<()> {
    let value = stack.pop1().with_context(|| Error::StackPopError)?;

    match value {
        Value::I32(v) => {
            stack.push(v.count_ones() as i32);
        }
        Value::I64(v) => {
            stack.push(v.count_ones() as i64);
        }
        _ => bail!(Error::UnexpectedStackValueType(value)),
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
        _ => bail!(Error::UnexpectedStackValueType(value)),
    }
    Ok(())
}

pub fn get_end_address(insts: &[Instruction], pc: isize) -> Result<usize> {
    let mut pc = pc as usize;
    let mut depth = 0;
    loop {
        pc += 1;
        let inst = insts
            .get(pc)
            .with_context(|| Error::NotFoundInstruction(pc))?;
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

pub fn get_else_or_end_address(insts: &[Instruction], pc: isize) -> Result<usize> {
    let mut pc = pc as usize;
    let mut depth = 0;
    loop {
        pc += 1;
        let inst = insts
            .get(pc)
            .with_context(|| Error::NotFoundInstruction(pc))?;
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

pub fn push_frame(stack: &mut Vec<Value>, call_stack: &mut Vec<Frame>, func: &InternalFuncInst) {
    let arity = func.func_type.results.len();
    let bottom = stack.len() - func.func_type.params.len();
    let mut locals = stack.split_off(bottom);

    for local in func.code.locals.iter() {
        match local {
            ValueType::I32 => locals.push(Value::I32(0)),
            ValueType::I64 => locals.push(Value::I64(0)),
            ValueType::F32 => locals.push(Value::F32(0.0)),
            ValueType::F64 => locals.push(Value::F64(0.0)),
        }
    }

    let sp = stack.len();
    let frame = Frame {
        pc: -1,
        sp,
        insts: func.code.body.clone(),
        arity,
        locals,
        labels: vec![],
    };
    trace!("call internal function: {:?}", &frame);
    call_stack.push(frame);
}

pub fn stack_unwind(stack: &mut Vec<Value>, sp: usize, arity: usize) -> Result<()> {
    if arity > 0 {
        let value = stack.pop().with_context(|| Error::StackPopError)?;
        stack.drain(sp..);
        stack.push(value);
    } else {
        stack.drain(sp..);
    }
    Ok(())
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
        .with_context(|| Error::NotFoundLabel(label_index))?;

    let pc = if kind == LabelKind::Loop {
        // NOTE: we still need loop label to jump to the beginning of the loop.
        labels.drain(label_index + 1..);
        // NOTE: since it jumps to the beginning of the loop,
        // the stack is unwound without considering the return value.
        stack_unwind(stack, sp, 0)?;
        start.with_context(|| Error::NotFoundStartPc)?
    } else {
        labels.drain(label_index..);
        stack_unwind(stack, sp, arity)?;
        pc as isize
    };
    Ok(pc)
}

fn is_wasi_modules(module: &str) -> bool {
    ["wasi_snapshot_preview1", "wasi_ephemeral_nn"].contains(&module)
}

pub fn invoke_external(
    store: Rc<RefCell<Store>>,
    stack: &mut impl StackAccess,
    func: ExternalFuncInst,
) -> Result<Option<Value>> {
    trace!("invoke external function: {:?}", &func);
    let mut args = Vec::with_capacity(func.func_type.params.len());
    for _ in 0..func.func_type.params.len() {
        args.push(stack.pop1()?);
    }
    args.reverse();

    // if module is a WASI module, it dosn't have store, so we suould use current store.
    let module_store = if is_wasi_modules(&func.module) {
        store.clone()
    } else {
        let brrowed = store.borrow();
        let Some(imports) = brrowed.imports.as_ref() else {
            bail!(Error::NoImports);
        };

        let import = imports
            .get(&func.module)
            .with_context(|| Error::NotFoundImportModule(func.module.clone()))?;

        let Some(module_store) = import.get(&func.module)? else {
            bail!(Error::NotFoundImportModule(func.module.clone()));
        };

        module_store.clone()
    };

    let store_for_invoke = module_store.clone();

    let module_store = store.borrow();
    let importer = module_store
        .imports
        .as_ref()
        .with_context(|| Error::NoImports)?
        .get(&func.module)
        .with_context(|| Error::NotFoundImportModule(func.module.clone()))?;

    importer.invoke(store_for_invoke, func, args)
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
