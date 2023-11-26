use crate::{
    module::{ExternalFuncInst, FuncInst, GlobalInst, InternalMemoryInst, InternalTableInst},
    ExternalVal, Importer, Runtime, Store, Value,
};
use anyhow::{bail, Context as _, Result};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default, Clone)]
pub struct Imports(pub HashMap<String, Import>);

#[derive(Clone)]
pub struct Import((String, Rc<RefCell<Store>>));

impl Import {
    pub fn new(name: String, store: Rc<RefCell<Store>>) -> Self {
        Self((name, store))
    }
}

impl Importer for Import {
    fn name(&self) -> &str {
        let (name, _) = &self.0;
        name.as_str()
    }

    fn get(&self, name: &str) -> Result<Option<Rc<RefCell<Store>>>> {
        if self.name() != name {
            return Ok(None);
        }
        let (_, store) = &self.0;
        Ok(Some(Rc::clone(store)))
    }

    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        let mut runtime = Runtime::instantiate(Rc::clone(&store))?;
        runtime.call(func.field, args)
    }

    fn resolve_table(
        &self,
        name: &str,
        field: &str,
    ) -> Result<Option<Rc<RefCell<InternalTableInst>>>> {
        let store = self.get(name)?;
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

                Ok(Some(Rc::clone(table)))
            }
            None => {
                bail!("cannot resolve table. not found module: {name} in imports",);
            }
        }
    }

    fn resolve_global(&self, name: &str, field: &str) -> Result<Option<GlobalInst>> {
        let store = self.get(name)?;
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

                Ok(Some(Rc::clone(global)))
            }
            None => {
                bail!("cannot resolve global. not found module: {name} in imports",);
            }
        }
    }

    fn resolve_func(&self, name: &str, field: &str) -> Result<Option<FuncInst>> {
        let store = self.get(name)?;
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

                Ok(Some(func.clone()))
            }
            None => {
                bail!("cannot resolve function. not found module: {name} in imports",);
            }
        }
    }

    fn resolve_memory(
        &self,
        name: &str,
        field: &str,
    ) -> Result<Option<Rc<RefCell<InternalMemoryInst>>>> {
        let store = self.get(name)?;
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

                Ok(Some(Rc::clone(memory)))
            }
            None => {
                bail!("cannot resolve memory. not found module: {name} in imports",);
            }
        }
    }
}
