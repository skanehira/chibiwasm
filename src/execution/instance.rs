#![allow(unused)]

use super::address::*;
use super::indices::TypeIdx;
use super::store::Store;
use super::value::{ExternalVal, Value};
use crate::binary::instruction::Instruction;
use crate::binary::module::Module;
use crate::binary::types::ValueType;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
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
    // pub module: Rc<ModuleInst>, TODO: add module instance
    pub code: Func,
}

#[derive(Debug)]
pub struct TableInst {
    pub elem: Vec<FuncAddr>,
    pub max: Option<u32>,
}

#[derive(Debug)]
pub struct MemoryInst {
    pub data: Vec<u8>,
    pub max: Option<u32>,
}

#[derive(Debug)]
pub struct GlobalInst {
    pub value: Value,
    pub mutability: bool,
}

#[derive(Debug)]
pub struct ExportInst {
    pub name: String,
    pub desc: ExternalVal,
}

#[derive(Debug, Default)]
pub struct ModuleInst {
    pub func_types: Vec<FuncType>,
    pub func_addrs: Vec<FuncAddr>,
    pub table_addrs: Vec<TableAddr>,
    pub memory_addrs: Vec<MemoryAddr>,
    pub global_addrs: Vec<GlobalAddr>,
    pub exports: HashMap<String, ExportInst>,
}

impl ModuleInst {
    pub fn new(store: &Store, module: &Module) -> Self {
        let mut exports = HashMap::default();

        match module.export_section.as_ref() {
            Some(sections) => {
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
            }
            None => {}
        }

        let module_inst = ModuleInst {
            func_types: vec![],
            func_addrs: vec![],
            table_addrs: vec![],
            memory_addrs: vec![],
            global_addrs: vec![],
            exports,
        };
        module_inst
    }
}
