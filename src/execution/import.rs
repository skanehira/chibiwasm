use crate::{
    error::{ Error, Resource },
    module::{ExternalFuncInst, FuncInst, GlobalInst, InternalMemoryInst, InternalTableInst},
    ExternalVal, Importer, Runtime, Store, Value,
};
use anyhow::{bail, Context as _, Result};
use std::{collections::HashMap, sync::{ Arc, Mutex } };

#[derive(Default, Clone)]
pub struct Imports(pub HashMap<String, Arc<Mutex<Store>>>);

impl Importer for Imports {
    fn get(&self, name: &str) -> Result<Option<Arc<Mutex<Store>>>> {
        let store = self.0.get(name).with_context(|| Error::NoImports)?;
        Ok(Some(Arc::clone(store)))
    }
    fn add(&mut self, name: &str, module: Arc<Mutex<Store>>) {
        self.0.insert(name.into(), module);
    }
    fn invoke(
        &self,
        store: Arc<Mutex<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        let mut runtime = Runtime::instantiate(Arc::clone(&store))?;
        runtime.call(func.field, args)
    }
    fn resolve_table(
        &self,
        name: &str,
        field: &str,
    ) -> Result<Option<Arc<Mutex<InternalTableInst>>>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store: std::sync::MutexGuard<Store> = store
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Memory))?;

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

                Ok(Some(Arc::clone(table)))
            }
            None => {
                bail!("cannot resolve function. not found module: {name} in imports",);
            }
        }
    }

    fn resolve_global(&self, name: &str, field: &str) -> Result<Option<GlobalInst>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Store))?;
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

                Ok(Some(Arc::clone(global)))
            }
            None => {
                bail!("cannot resolve global. not found module: {name} in imports",);
            }
        }
    }

    fn resolve_func(&self, name: &str, field: &str) -> Result<Option<FuncInst>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Store))?;

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
    ) -> Result<Option<Arc<Mutex<InternalMemoryInst>>>> {
        let store = self.0.get(name);
        match store {
            Some(store) => {
                let store = store
                    .lock()
                    .ok()
                    .with_context(|| Error::CanNotLockForThread(Resource::Store))?;

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

                Ok(Some(Arc::clone(memory)))
            }
            None => {
                bail!("cannot resolve memory. not found module: {name} in imports",);
            }
        }
    }
}
