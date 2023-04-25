use super::{error::{ Error, Resource }, module::*, value::Value};
use crate::{
    binary::{
        module::{Decoder, Module},
        types::{Expr, ExprValue, FuncType, Mutability},
    },
    Importer,
};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::{Cursor, Read},
    sync::{ Arc, Mutex }
};

#[derive(Debug)]
pub enum Exports {
    Func(FuncInst),
    Table(TableInst),
    Memory(MemoryInst),
    Global(GlobalInst),
}

#[derive(Default)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub tables: Vec<TableInst>,
    pub memory: Vec<MemoryInst>,
    pub globals: Vec<GlobalInst>,
    pub imports: Option<Box<dyn Importer + Send + Sync>>,
    pub module: ModuleInst,
    pub start: Option<u32>,
}

impl Store {
    pub fn from_file(file: &str, import: Option<Box<dyn Importer + Send + Sync>>) -> Result<Self> {
        let file = fs::File::open(file)?;
        let mut decoder = Decoder::new(file);
        let module = decoder.decode()?;
        Self::new(&module, import)
    }

    pub fn from_reader(reader: &mut impl Read, imports: Option<Box<dyn Importer + Send + Sync>>) -> Result<Self> {
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T, imports: Option<Box<dyn Importer + Send + Sync>>) -> Result<Self> {
        let buf = Cursor::new(b);
        let mut decoder = Decoder::new(buf);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn new(module: &Module, imports: Option<Box<dyn Importer + Send + Sync>>) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref functions) => functions.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];
        let mut tables = vec![];
        let mut globals = vec![];
        let mut memories = vec![];

        if let Some(ref section) = module.import_section {
            let imports = imports
                .as_ref()
                .with_context(|| "module has import section, but not found any imported module")?;

            for import in section {
                let module_name = import.module.as_str();
                let field = import.field.as_str();

                match import.kind {
                    crate::binary::types::ImportKind::Func(typeidx) => {
                        let idx = typeidx as usize;
                        let func_type = module
                            .type_section
                            .as_ref()
                            .with_context(|| Error::NotFoundTypeSection)?
                            .get(idx)
                            .with_context(|| Error::NotFoundFuncType(idx))?;

                        let func_type = FuncType {
                            params: func_type.params.clone(),
                            results: func_type.results.clone(),
                        };
                        let func = FuncInst::External(ExternalFuncInst {
                            module: module_name.to_string(),
                            field: field.to_string(),
                            func_type,
                        });
                        funcs.push(func);
                    }
                    crate::binary::types::ImportKind::Table(_) => {
                        let table = imports
                            .resolve_table(module_name, field)?
                            .with_context(|| Error::NoImports)?; // TODO: define error enum
                        tables.push(table);
                    }
                    crate::binary::types::ImportKind::Global(_) => {
                        let global = imports
                            .resolve_global(module_name, field)?
                            .with_context(|| Error::NoImports)?;
                        globals.push(global);
                    }
                    crate::binary::types::ImportKind::Memory(_) => {
                        let memory = imports
                            .resolve_memory(module_name, field)?
                            .with_context(|| Error::NoImports)?;
                        memories.push(memory);
                    }
                }
            }
        }

        if let Some(ref section) = module.global_section {
            for global in section {
                let value = match global.init_expr {
                    ExprValue::I32(v) => Value::I32(v),
                    ExprValue::I64(v) => Value::I64(v),
                    ExprValue::F32(v) => Value::F32(v),
                    ExprValue::F64(v) => Value::F64(v),
                };
                let global = InternalGlobalInst {
                    value,
                    mutability: global.global_type.mutability == Mutability::Var,
                };
                globals.push(Arc::new(Mutex::new(global)));
            }
        }

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
                    .with_context(|| "cannot get func type from type section")?
                    .clone();

                let mut locals = Vec::with_capacity(func_body.locals.len());
                for local in func_body.locals.iter() {
                    for _ in 0..local.type_count {
                        locals.push(local.value_type.clone());
                    }
                }

                // NOTE: locals length must be func_type.params + func_body.locals
                let func = InternalFuncInst {
                    func_type,
                    code: Func {
                        type_idx: *typeidx,
                        locals,
                        body: func_body.code.clone(),
                    },
                };
                funcs.push(FuncInst::Internal(func));
            }
        }

        // NOTE: only support one memory now
        if let Some(ref section) = module.memory_section {
            for memory in section {
                let min = memory.limits.min * PAGE_SIZE;
                let memory = InternalMemoryInst {
                    data: vec![0; min as usize],
                    max: memory.limits.max,
                };
                memories.push(Arc::new(Mutex::new(memory)));
            }
        }

        // eval for offset in the table
        let eval = |globals: &Vec<GlobalInst>, offset: Expr| -> Result<usize> {
            match offset {
                Expr::Value(value) => Ok(i32::from(value) as usize),
                Expr::GlobalIndex(idx) => {
                    let global = globals
                        .get(idx)
                        .with_context(|| "not found offset from globals")?
                        .lock()
                        .ok()
                        .with_context(|| Error::CanNotLockForThread(Resource::Global))?;
                    Ok(i32::from(global.value.clone()) as usize)
                }
            }
        };

        // table will be shared by all module instance
        // so if element is exists in the same index, overwrite the table
        let update_funcs_in_table = |entries: &mut Vec<Option<FuncInst>>| -> Result<()> {
            if let Some(ref elems) = module.element_section {
                for elem in elems {
                    let offset = eval(&globals, elem.offset.clone())?;
                    if entries.len() <= offset {
                        entries.resize(entries.len() + offset + elem.init.len(), None);
                    }
                    for (i, func_idx) in elem.init.iter().enumerate() {
                        let func = funcs
                            .get(*func_idx as usize)
                            .with_context(|| format!("not found function by {func_idx}"))?;
                        entries[offset + i] = Some(func.clone());
                    }
                }
            };
            Ok(())
        };

        // table
        if let Some(ref table_section) = module.table_section {
            let table = table_section
                .get(0) // NOTE: only support one table now
                .with_context(|| "cannot get table from table section")?; // NOTE: only support one table now
            let min = table.limits.min as usize;

            let mut entries = vec![None; min];
            update_funcs_in_table(&mut entries)?;

            let table_inst = InternalTableInst {
                funcs: entries,
                max: table.limits.max,
            };
            tables.push(Arc::new(Mutex::new(table_inst)));
        } else {
            // update table if element section exists
            if !tables.is_empty() {
                let entries = &mut tables
                    .first()
                    .with_context(|| "not found table")?
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Table))?
                    .funcs;
                update_funcs_in_table(entries)?;
            }
        }

        // copy data to memory
        if let Some(ref data_list) = module.data {
            for data in data_list {
                let offset = eval(&globals, data.offset.clone())?;
                let init_data = &data.init;
                let mut memory = memories
                    .get(data.memory_index as usize)
                    .with_context(|| "not found memory")?
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;
                if offset + init_data.len() > memory.data.len() {
                    bail!("data is too large to fit in memory");
                }
                memory.data[offset..offset + init_data.len()].copy_from_slice(init_data);
            }
        }

        let module_inst = ModuleInst::allocate(module);

        let store = Self {
            funcs,
            tables,
            memory: memories,
            globals,
            imports,
            module: module_inst,
            start: module.start_section,
        };

        Ok(store)
    }
}
