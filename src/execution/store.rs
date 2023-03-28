use super::{module::*, value::Value};
use crate::binary::{
    module::Module,
    types::{ExprValue, Mutability},
};
use anyhow::{Context, Result};

// https://www.w3.org/TR/wasm-core-1/#memory-instances%E2%91%A0
const PAGE_SIZE: u32 = 65536; // 64Ki

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
        for (idx, func_idx) in module
            .function_section
            .as_ref()
            .context("not found function section")?
            .iter()
            .enumerate()
        {
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

        // NOTE: only support one memory now
        let memories = match &module.memory_section {
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
