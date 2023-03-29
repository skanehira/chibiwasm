use super::{module::*, value::Value};
use crate::binary::{
    module::Module,
    types::{ExprValue, Mutability},
};
use anyhow::{bail, Context, Result};

#[derive(Debug, Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub tables: Vec<TableInst>,
    pub memories: Vec<MemoryInst>,
    pub globals: Vec<GlobalInst>,
}

impl Store {
    pub fn new(module: &Module) -> Result<Self> {
        let mut funcs = vec![];
        match module.function_section {
            Some(ref functions) => {
                for (idx, func_idx) in functions.iter().enumerate() {
                    let types = module
                        .type_section
                        .as_ref()
                        .context("not found type section")?;

                    let func_type = types.get(*func_idx as usize).context("not found type")?;

                    let func_type = FuncType {
                        params: func_type.params.clone(),
                        results: func_type.results.clone(),
                    };

                    let func_body = module
                        .code_section
                        .as_ref()
                        .context("not found code section")?
                        .get(idx)
                        .context("not found code")?;

                    let func = Func {
                        type_idx: idx as u32,
                        locals: func_body
                            .locals
                            .iter()
                            .map(|local| local.value_type.clone())
                            .collect(),
                        body: func_body.code.clone(),
                    };
                    let func_inst = FuncInst {
                        func_type,
                        code: func,
                    };
                    funcs.push(func_inst);
                }
            }
            _ => {}
        }

        // NOTE: only support one memory now
        let mut memories = match &module.memory_section {
            Some(memory) => {
                let memory = memory.get(0);
                match memory {
                    Some(memory) => {
                        // https://www.w3.org/TR/wasm-core-1/#memories%E2%91%A5
                        let min = memory.limits.min * PAGE_SIZE;
                        vec![MemoryInst {
                            data: vec![0; min as usize],
                            max: memory.limits.max,
                        }]
                    }
                    None => vec![],
                }
            }
            _ => {
                vec![]
            }
        };

        // 10. copy data to memory
        match module.data {
            Some(ref data) => {
                for d in data {
                    let memory = memories.get_mut(d.memory_index as usize).unwrap();
                    let offset = {
                        let offset: i32 = d.offset.clone().into();
                        offset as usize
                    };

                    let data = &d.init;
                    if offset + data.len() > memory.data.len() {
                        bail!("data is too large to fit in memory");
                    }
                    memory.data[offset..offset + data.len()].copy_from_slice(data);
                }
            }
            _ => {}
        }

        let globals = match &module.global_section {
            Some(globals) => globals
                .iter()
                .map(|g| {
                    let value = match g.init_expr {
                        ExprValue::I32(v) => Value::I32(v),
                        ExprValue::I64(v) => Value::I64(v),
                        ExprValue::F32(v) => Value::F32(v),
                        ExprValue::F64(v) => Value::F64(v),
                    };
                    GlobalInst {
                        value,
                        mutability: g.global_type.mutability == Mutability::Var,
                    }
                })
                .collect(),
            None => vec![],
        };

        let store = Self {
            funcs,
            memories,
            globals,
            ..Self::default()
        };

        Ok(store)
    }
}
