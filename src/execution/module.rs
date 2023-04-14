use super::indices::TypeIdx;
use super::value::{ExternalVal, Numeric, Value};
use crate::binary::instruction::{Instruction, MemoryArg};
use crate::binary::module::Module;
use crate::binary::types::{FuncType, ValueType};
use crate::execution::error::Error;
use anyhow::{bail, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// https://www.w3.org/TR/wasm-core-1/#memory-instances%E2%91%A0
pub const PAGE_SIZE: u32 = 65536; // 64Ki

#[derive(Debug, Clone)]
pub struct Func {
    pub type_idx: TypeIdx,
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct InternalFuncInst {
    pub func_type: FuncType,
    pub code: Func,
}

#[derive(Debug, Clone)]
pub struct ExternalFuncInst {
    pub module: String,
    pub field: String, // function name
    pub func_type: FuncType,
}

#[derive(Debug, Clone)]
pub enum FuncInst {
    Internal(InternalFuncInst),
    External(ExternalFuncInst),
}

#[derive(Debug, Clone, Default)]
pub struct InternalTableInst {
    pub funcs: Vec<Option<FuncInst>>,
    pub max: Option<u32>,
}
pub type TableInst = Rc<RefCell<InternalTableInst>>;

#[derive(Default, Debug, Clone)]
pub struct InternalMemoryInst {
    pub data: Vec<u8>,
    pub max: Option<u32>,
}
pub type MemoryInst = Rc<RefCell<InternalMemoryInst>>;

impl InternalMemoryInst {
    pub fn size(&self) -> usize {
        self.data.len() / PAGE_SIZE as usize
    }

    // https://www.w3.org/TR/wasm-core-1/#grow-mem
    pub fn grow(&mut self, grow_size: u32) -> Result<()> {
        let size = self.size() as u32;
        if size % PAGE_SIZE != 0 {
            bail!(Error::MemorySizeNotPageAligned(PAGE_SIZE));
        }
        let len = size + grow_size;
        if let Some(max) = self.max {
            if max < len {
                bail!(Error::MemoryPageOverflow(max, len));
            }
        }
        self.data.resize((len * PAGE_SIZE) as usize, 0);
        Ok(())
    }

    pub fn load<T: Numeric>(&self, addr: usize, arg: &MemoryArg) -> Result<T> {
        // TODO: check align and memory size
        let at = addr + arg.offset as usize;
        Numeric::read(&self.data, at)
    }

    pub fn write<T: Numeric>(&mut self, addr: usize, arg: &MemoryArg, value: T) -> Result<()> {
        // TODO: check align and memory size
        let at = addr + arg.offset as usize;
        Numeric::write(&mut self.data, at, value)
    }
}

pub type GlobalInst = Rc<RefCell<InternalGlobalInst>>;

#[derive(Debug, Clone)]
pub struct InternalGlobalInst {
    pub value: Value,
    pub mutability: bool,
}

#[derive(Debug, Clone)]
pub struct ExportInst {
    pub name: String,
    pub desc: ExternalVal,
}

#[derive(Debug, Default, Clone)]
pub struct ModuleInst {
    pub func_types: Vec<FuncType>,
    pub exports: HashMap<String, ExportInst>,
}

impl ModuleInst {
    // https://www.w3.org/TR/wasm-core-1/#modules%E2%91%A6
    pub fn allocate(module: &Module) -> Self {
        // func types
        let mut func_types = vec![];
        if let Some(ref section) = module.type_section {
            for ty in section {
                let func_type = FuncType {
                    params: ty.params.clone(),
                    results: ty.results.clone(),
                };
                func_types.push(func_type);
            }
        };

        // exports
        let mut exports = HashMap::default();
        if let Some(ref sections) = module.export_section {
            for export in sections {
                let desc = match export.desc {
                    crate::binary::types::ExportDesc::Func(idx) => ExternalVal::Func(idx),
                    crate::binary::types::ExportDesc::Table(idx) => ExternalVal::Table(idx),
                    crate::binary::types::ExportDesc::Memory(idx) => ExternalVal::Memory(idx),
                    crate::binary::types::ExportDesc::Global(idx) => ExternalVal::Global(idx),
                };
                let name = export.name.clone();
                let export_inst = ExportInst {
                    name: name.clone(),
                    desc,
                };
                exports.insert(name, export_inst);
            }
        };

        ModuleInst {
            func_types,
            exports,
        }
    }
}
