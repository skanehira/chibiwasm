use super::address::*;
use super::indices::TypeIdx;
use super::value::{ExternalVal, Numberic, Value};
use crate::binary::instruction::{Instruction, MemoryArg};
use crate::binary::module::Module;
use crate::binary::types::ValueType;
use anyhow::{bail, Result};
use std::collections::HashMap;

// https://www.w3.org/TR/wasm-core-1/#memory-instances%E2%91%A0
pub const PAGE_SIZE: u32 = 65536; // 64Ki

#[derive(Debug, Clone, PartialEq)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

#[derive(Debug, Clone)]
pub struct Func {
    pub type_idx: TypeIdx,
    pub locals: Vec<ValueType>,
    pub body: Vec<Instruction>,
}

#[derive(Debug, Clone)]
pub struct FuncInst {
    pub func_type: FuncType,
    pub code: Func,
}

#[derive(Debug, Clone)]
pub struct TableInst {
    pub elem: Vec<FuncAddr>,
    pub max: Option<u32>,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryInst {
    pub data: Vec<u8>,
    pub max: Option<u32>,
}

// size of MemoryInst
impl MemoryInst {
    pub fn size(&self) -> usize {
        self.data.len() / PAGE_SIZE as usize
    }

    // https://www.w3.org/TR/wasm-core-1/#grow-mem
    pub fn grow(&mut self, grow_size: u32) -> Result<()> {
        let size = self.size() as u32;
        if size % PAGE_SIZE != 0 {
            bail!("memory size is not page aligned");
        }
        let len = size + grow_size;
        if let Some(max) = self.max {
            if max < len {
                bail!("page overflow");
            }
        }
        self.data.resize((len * PAGE_SIZE) as usize, 0);
        Ok(())
    }

    pub fn load<T: Numberic>(&self, addr: usize, arg: &MemoryArg) -> Result<T> {
        // TODO: check align and memory size
        let at = addr + arg.offset as usize;
        Numberic::read(&self.data, at)
    }

    pub fn write<T: Numberic>(&mut self, addr: usize, arg: &MemoryArg, value: T) -> Result<()> {
        // TODO: check align and memory size
        let at = addr + arg.offset as usize;
        Numberic::write(&mut self.data, at, value)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalInst {
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
    pub func_addrs: Vec<FuncAddr>,
    pub table_addrs: Vec<TableAddr>,
    pub memory_addrs: Vec<MemoryAddr>,
    pub global_addrs: Vec<GlobalAddr>,
    pub exports: HashMap<String, ExportInst>,
}

impl ModuleInst {
    // https://www.w3.org/TR/wasm-core-1/#modules%E2%91%A6
    pub fn allocate(module: &Module) -> Self {
        let func_types = Self::into_func_types(module);
        let exports = Self::into_exports(module);
        let func_addrs = Self::into_func_addrs(module);
        let table_addrs = Self::into_table_addrs(module);
        let memory_addrs = Self::into_memory_addrs(module);
        let global_addrs = Self::into_global_addrs(module);

        ModuleInst {
            func_types,
            func_addrs,
            table_addrs,
            memory_addrs,
            global_addrs,
            exports,
        }
    }

    fn into_func_types(module: &Module) -> Vec<FuncType> {
        let mut types = vec![];
        if let Some(ref func_types) = module.type_section {
            for ty in func_types {
                let func_type = FuncType {
                    params: ty.params.clone(),
                    results: ty.results.clone(),
                };
                types.push(func_type);
            }
        };
        types
    }

    fn into_func_addrs(module: &Module) -> Vec<FuncAddr> {
        let mut func_addrs = vec![];
        if let Some(ref functions) = module.function_section {
            for addr in 0..functions.len() {
                func_addrs.push(addr);
            }
        }

        func_addrs
    }

    fn into_table_addrs(module: &Module) -> Vec<TableAddr> {
        let mut table_addrs = vec![];
        if let Some(ref tables) = module.table_section {
            for addr in 0..tables.len() {
                table_addrs.push(addr);
            }
        }

        table_addrs
    }

    fn into_memory_addrs(module: &Module) -> Vec<MemoryAddr> {
        let mut memory_addrs = vec![];
        if let Some(ref memories) = module.memory_section {
            for addr in 0..memories.len() {
                memory_addrs.push(addr);
            }
        }

        memory_addrs
    }

    fn into_global_addrs(module: &Module) -> Vec<GlobalAddr> {
        let mut global_addrs = vec![];
        if let Some(ref globals) = module.global_section {
            for addr in 0..globals.len() {
                global_addrs.push(addr);
            }
        }

        global_addrs
    }

    fn into_exports(module: &Module) -> HashMap<String, ExportInst> {
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
        exports
    }
}
