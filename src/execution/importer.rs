use crate::{
    module::{ExternalFuncInst, FuncInst, GlobalInst, InternalMemoryInst, InternalTableInst},
    Store, Value,
};
use anyhow::Result;
use std::sync::{ Arc, Mutex };

pub trait Importer {
    fn get(&self, _name: &str) -> Result<Option<Arc<Mutex<Store>>>> {
        Ok(None)
    }
    fn add(&mut self, _name: &str, _module: Arc<Mutex<Store>>) {
        // do nothing
    }
    fn invoke(
        &self,
        store: Arc<Mutex<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>>;
    fn resolve_table(
        &self,
        _module: &str,
        _field: &str,
    ) -> Result<Option<Arc<Mutex<InternalTableInst>>>> {
        Ok(None)
    }
    fn resolve_global(&self, _module: &str, _field: &str) -> Result<Option<GlobalInst>> {
        Ok(None)
    }
    fn resolve_func(&self, _module: &str, _field: &str) -> Result<Option<FuncInst>> {
        Ok(None)
    }
    fn resolve_memory(
        &self,
        _name: &str,
        _field: &str,
    ) -> Result<Option<Arc<Mutex<InternalMemoryInst>>>> {
        Ok(None)
    }
}
