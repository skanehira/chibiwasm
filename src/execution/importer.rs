use crate::{
    module::{ExternalFuncInst, FuncInst, GlobalInst, InternalMemoryInst, InternalTableInst},
    Store, Value,
};
use anyhow::Result;
use std::{cell::RefCell, rc::Rc};

pub trait Importer {
    fn get(&self, _name: &str) -> Result<Option<Rc<RefCell<Store>>>> {
        Ok(None)
    }
    fn add(&mut self, _name: &str, _module: Rc<RefCell<Store>>) {
        // do nothing
    }
    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>>;
    fn resolve_table(
        &self,
        _module: &str,
        _field: &str,
    ) -> Result<Option<Rc<RefCell<InternalTableInst>>>> {
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
    ) -> Result<Option<Rc<RefCell<InternalMemoryInst>>>> {
        Ok(None)
    }
}
