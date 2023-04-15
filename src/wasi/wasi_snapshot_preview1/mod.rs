pub mod process;
pub mod types;

use crate::{
    binary::types::FuncType,
    module::{ExternalFuncInst, FuncInst},
    Importer, Store, Value,
};
use anyhow::{Context as _, Result};
use std::io::prelude::*;
use std::{cell::RefCell, fs::File, os::fd::FromRawFd, rc::Rc};

pub struct Wasi {}

fn fd_write(store: Rc<RefCell<Store>>, args: Vec<Value>) -> Result<Option<Value>> {
    let args: Vec<i32> = args.into_iter().map(Into::into).collect();
    let (fd, iovs, iovs_len) = (args[0], args[1], args[2]);

    let store = store.borrow();
    let memory = store.memory.get(0).with_context(|| "not found memory")?;
    let memory = memory.borrow_mut();

    let data = memory
        .data
        .get((iovs as usize)..iovs_len as usize)
        .with_context(|| "not found iovs")?;

    unsafe {
        let mut fd = File::from_raw_fd(fd);
        match fd.write_all(data) {
            Ok(_) => {
                return Ok(Some(Value::I32(data.len() as i32)));
            }
            Err(err) => {
                println!("error: {}", err);
            }
        }
    }
    Ok(None)
}

impl Importer for Wasi {
    fn invoke(
        &self,
        store: Rc<RefCell<Store>>,
        func: ExternalFuncInst,
        args: Vec<Value>,
    ) -> Result<Option<Value>> {
        match func.field.as_str() {
            "fd_write" => fd_write(store, args),
            _ => todo!(),
        }
    }

    fn get(&self, _name: &str) -> anyhow::Result<Option<Rc<RefCell<Store>>>> {
        Ok(None)
    }

    fn resolve_table(
        &self,
        _module: &str,
        _field: &str,
    ) -> anyhow::Result<Option<Rc<RefCell<crate::module::InternalTableInst>>>> {
        Ok(None)
    }

    fn resolve_global(
        &self,
        _module: &str,
        _field: &str,
    ) -> anyhow::Result<Option<crate::module::GlobalInst>> {
        Ok(None)
    }

    fn resolve_func(&self, module: &str, field: &str) -> anyhow::Result<Option<FuncInst>> {
        let func = ExternalFuncInst {
            module: module.into(),
            field: field.into(),
            func_type: FuncType::default(),
        };
        match field {
            "print" => Ok(Some(FuncInst::External(func))),
            _ => Ok(None),
        }
    }

    fn resolve_memory(
        &self,
        _name: &str,
        _field: &str,
    ) -> anyhow::Result<Option<Rc<RefCell<crate::module::InternalMemoryInst>>>> {
        Ok(None)
    }

    fn add(&mut self, _name: &str, _module: Rc<RefCell<Store>>) {
        todo!()
    }
}
