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
    pub memory: MemoryInst,
    pub globals: Vec<GlobalInst>,
}

impl Store {
    pub fn new(module: &Module) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref functions) => functions.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];
        if let Some(ref code_section) = module.code_section {
            if code_section.len() != func_type_idxs.len() {
                bail!("code section length must be equal to function section length");
            }
            for (func_body, typeidx) in code_section.iter().zip(func_type_idxs.iter()) {
                let func_type = module
                    .type_section
                    .as_ref()
                    .with_context(|| "cannot get type section")?
                    .get(*typeidx as usize)
                    .with_context(|| "cannot get func type from type section")?;
                let func_type = FuncType {
                    params: func_type.params.clone(),
                    results: func_type.results.clone(),
                };

                let mut locals = Vec::with_capacity(func_body.locals.len());
                for local in func_body.locals.iter() {
                    for _ in 0..local.type_count {
                        locals.push(local.value_type.clone());
                    }
                }

                // NOTE: locals length must be func_type.params + func_body.locals
                let func = Func {
                    type_idx: *typeidx,
                    locals,
                    body: func_body.code.clone(),
                };

                let func_inst = FuncInst {
                    func_type,
                    code: func,
                };
                funcs.push(func_inst);
            }
        }

        // NOTE: only support one memory now
        let mut memory = match module.memory_section {
            Some(ref memory) => {
                let memory = memory.get(0);
                match memory {
                    Some(memory) => {
                        // https://www.w3.org/TR/wasm-core-1/#memories%E2%91%A5
                        let min = memory.limits.min * PAGE_SIZE;
                        MemoryInst {
                            data: vec![0; min as usize],
                            max: memory.limits.max,
                        }
                    }
                    None => MemoryInst::default(),
                }
            }
            _ => MemoryInst::default(),
        };

        // table
        let tables = match module.table_section {
            Some(ref tables) => {
                let table = tables.get(0).expect("cannot get table from table section"); // NOTE: only support one table now

                let elem = match &module.element_section {
                    Some(elems) => {
                        let mut elem = vec![0; table.limits.max.unwrap_or(0) as usize];
                        for i in 0..elem.len() {
                            elem[i] = *elems
                                .get(0) // NOTE: only support one elem now
                                .with_context(|| {
                                    format!(
                                "cannot get element from element section, element_section: {:#?}",
                                elems
                            )
                                })?
                                .init
                                .get(i)
                                .with_context(|| {
                                    format!("cannot get func index from element_section")
                                })? as usize;
                        }
                        elem
                    }
                    None => vec![],
                };
                let table_inst = TableInst {
                    elem,
                    max: table.limits.max,
                };
                vec![table_inst]
            }
            None => vec![],
        };

        // 10. copy data to memory
        match module.data {
            Some(ref data) => {
                for d in data {
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

        let globals = match module.global_section {
            Some(ref globals) => globals
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
            tables,
            memory,
            globals,
        };

        Ok(store)
    }
}
