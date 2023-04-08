#![allow(clippy::needless_range_loop)]

use super::{
    module::*,
    value::{ExternalVal, Value},
};
use crate::binary::{
    module::{Decoder, Module},
    types::{Expr, ExprValue, Mutability},
};
use anyhow::{bail, Context, Result};
use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    io::{Cursor, Read},
    rc::Rc,
};

#[derive(Debug)]
pub enum Exports {
    Func(FuncInst),
    Table(TableInst),
    Memory(MemoryInst),
    Global(GlobalInst),
}

#[derive(Default, Debug, Clone)]
pub struct Imports(HashMap<String, Rc<RefCell<Store>>>);

impl Imports {
    pub fn add(&mut self, name: &str, module: Rc<RefCell<Store>>) {
        self.0.insert(name.into(), module);
    }

    pub fn resolve_table(&self, name: &str, field: &str) -> Result<Rc<RefCell<InternalTableInst>>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store.borrow();

                let export_inst = store
                    .module
                    .exports
                    .get(field)
                    .context(format!("not found exported table '{field}' from {name}"))?;

                let external_val = &export_inst.desc;
                let ExternalVal::Table(idx) = external_val else {
                    bail!("invalid export desc: {:?}", external_val);
                };

                let table = store
                    .tables
                    .get(*idx as usize)
                    .with_context(|| format!("not found table {idx} in module: {name}"))?;

                Ok(Rc::clone(table))
            }
            None => {
                bail!(
                    "cannot resolve function. not found module: {name} in imports: {:?}",
                    self.0
                );
            }
        }
    }

    pub fn resolve_global(&self, name: &str, field: &str) -> Result<GlobalInst> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store.borrow();
                let export_inst = store
                    .module
                    .exports
                    .get(field)
                    .context(format!("not found exported global '{field}' from {name}"))?;
                let external_val = &export_inst.desc;

                let ExternalVal::Global(idx) = external_val else {
                    bail!("invalid export desc: {:?}", external_val);
                };
                let global = store
                    .globals
                    .get(*idx as usize)
                    .with_context(|| format!("not found global index '{idx}' from {name}"))?;

                Ok(Rc::clone(global))
            }
            None => {
                bail!(
                    "cannot resolve global. not found module: {name} in imports: {:?}",
                    self.0
                );
            }
        }
    }

    pub fn resolve_func(&self, name: &str, field: &str) -> Result<FuncInst> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store.borrow();

                let export_inst = store
                    .module
                    .exports
                    .get(field)
                    .context(format!("not found exported function '{field}' from {name}"))?;
                let external_val = &export_inst.desc;

                let ExternalVal::Func(idx) = external_val else {
                    bail!("invalid export desc: {:?}", external_val);
                };
                let func = store
                    .funcs
                    .get(*idx as usize)
                    .with_context(|| format!("not found function by {name}"))?;

                Ok(Rc::clone(func))
            }
            None => {
                bail!(
                    "cannot resolve function. not found module: {name} in imports: {:?}",
                    self.0
                );
            }
        }
    }

    pub fn resolve_memory(
        &self,
        name: &str,
        field: &str,
    ) -> Result<Rc<RefCell<InternalMemoryInst>>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store.borrow();

                let export_inst = store
                    .module
                    .exports
                    .get(field)
                    .context(format!("not found exported memory '{field}' from {name}"))?;
                let external_val = &export_inst.desc;

                let ExternalVal::Memory(idx) = external_val else {
                    bail!("invalid export desc: {:?}", external_val);
                };
                let memory = store
                    .memory
                    .get(*idx as usize)
                    .with_context(|| format!("not found memory from {name}"))?;

                Ok(Rc::clone(memory))
            }
            None => {
                bail!(
                    "cannot resolve memory. not found module: {name} in imports: {:?}",
                    self.0
                );
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Store {
    pub funcs: Vec<FuncInst>,
    pub tables: Vec<TableInst>,
    pub memory: Vec<MemoryInst>,
    pub globals: Vec<GlobalInst>,
    pub imports: Option<Imports>,
    pub module: ModuleInst,
    pub start: Option<u32>,
}

impl Store {
    pub fn from_file(file: &str, import: Option<Imports>) -> Result<Self> {
        let file = fs::File::open(file)?;
        let mut decoder = Decoder::new(file);
        let module = decoder.decode()?;
        Self::new(&module, import)
    }

    pub fn from_reader(reader: &mut impl Read, imports: Option<Imports>) -> Result<Self> {
        let mut decoder = Decoder::new(reader);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn from_bytes<T: AsRef<[u8]>>(b: T, imports: Option<Imports>) -> Result<Self> {
        let buf = Cursor::new(b);
        let mut decoder = Decoder::new(buf);
        let module = decoder.decode()?;
        Self::new(&module, imports)
    }

    pub fn new(module: &Module, imports: Option<Imports>) -> Result<Self> {
        let func_type_idxs = match module.function_section {
            Some(ref functions) => functions.clone(),
            _ => vec![],
        };

        let mut funcs = vec![];
        let mut tables = vec![];
        let mut globals = vec![];
        let mut memories = vec![];

        if let Some(ref section) = module.import_section {
            let imports = imports.as_ref().with_context(|| {
                "the module has import section, but not found any imported module"
            })?;

            for import in section {
                let module = import.module.as_str();
                let field = import.field.as_str();

                match import.kind {
                    crate::binary::types::ImportKind::Func(_) => {
                        let func = imports.resolve_func(module, field)?;
                        funcs.push(func);
                    }
                    crate::binary::types::ImportKind::Table(_) => {
                        let table = imports.resolve_table(module, field)?;
                        tables.push(table);
                    }
                    crate::binary::types::ImportKind::Global(_) => {
                        let global = imports.resolve_global(module, field)?;
                        globals.push(global);
                    }
                    crate::binary::types::ImportKind::Memory(_) => {
                        let memory = imports.resolve_memory(module, field)?;
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
                globals.push(Rc::new(RefCell::new(global)));
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
                let func = InternalFuncInst {
                    func_type,
                    code: Func {
                        type_idx: *typeidx,
                        locals,
                        body: func_body.code.clone(),
                    },
                };
                funcs.push(Rc::new(func));
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
                memories.push(Rc::new(RefCell::new(memory)));
            }
        }

        let eval = |globals: &Vec<GlobalInst>, offset: Expr| -> Result<usize> {
            match offset {
                Expr::Value(value) => Ok(i32::from(value) as usize),
                Expr::GlobalIndex(idx) => {
                    let global = globals
                        .get(idx)
                        .with_context(|| "not found offset from globals")?
                        .borrow();
                    Ok(i32::from(global.value.clone()) as usize)
                }
            }
        };

        let update_funcs_in_table =
            |entries: &mut Vec<Option<Rc<InternalFuncInst>>>| -> Result<()> {
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
                            entries[offset + i] = Some(Rc::clone(func));
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
            tables.push(Rc::new(RefCell::new(table_inst)));
        } else {
            // update table if element section exists
            if !tables.is_empty() {
                let entries = &mut tables
                    .first()
                    .with_context(|| "not found table")?
                    .borrow_mut()
                    .funcs;
                update_funcs_in_table(entries)?;
            }
        }

        // 10. copy data to memory
        if let Some(ref data) = module.data {
            for d in data {
                let offset = eval(&globals, d.offset.clone())?;
                let data = &d.init;
                let mut memory = memories
                    .get(d.memory_index as usize)
                    .with_context(|| "not found memory")?
                    .borrow_mut();
                if offset + data.len() > memory.data.len() {
                    bail!("data is too large to fit in memory");
                }
                memory.data[offset..offset + data.len()].copy_from_slice(data);
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
