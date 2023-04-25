use super::value::Value;
use thiserror::Error;
use std::fmt::{Display, Formatter};

#[derive(Error, Debug)]
pub enum Error {
    #[error("integer divide by zero")]
    IntegerDivideByZero,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("integer overflow")]
    DivisionOverflow,
    #[error("cannot pop value from stack")]
    StackPopError,
    #[error("memory size is not page aligned, page size is {0}")]
    MemorySizeNotPageAligned(u32),
    #[error("memory page is overflow. max is {0}, grow size is {1}")]
    MemoryPageOverflow(u32, u32),
    #[error("unexpected stack value type: {0:?}")]
    UnexpectedStackValueType(Value),
    #[error("not found local variable with index: {0}")]
    NotFoundLocalVariable(usize),
    #[error("not found global variable with index: {0}")]
    NotFoundGlobalVariable(usize),
    #[error("not found import module: {0}")]
    NotFoundImportModule(String),
    #[error("no any imports")]
    NoImports,
    #[error("not found instruction with pc: {0}")]
    NotFoundInstruction(usize),
    #[error("not found label with index: {0}")]
    NotFoundLabel(usize),
    #[error("cannot get start pc in the label")]
    NotFoundStartPc,
    #[error("not found exported instance by name: {0}")]
    NotFoundExportInstance(String),
    #[error("not found exported function by index: {0}")]
    NotFoundExportedFunction(u32),
    #[error("not found exported table by index: {0}")]
    NotFoundExportedTable(u32),
    #[error("not found exported memory by index: {0}")]
    NotFoundExportedMemory(u32),
    #[error("not found exported global by index: {0}")]
    NotFoundExportedGlobal(u32),
    #[error("not found memory by index: {0}")]
    NotFoundMemory(usize),
    #[error("cannot pop call stack when execute instruction: {0}")]
    CallStackPopError(String),
    #[error("invalid br_table index: {0}")]
    InvalidBrTableIndex(usize),
    #[error("cannot pop label when instruction: {0}")]
    LabelPopError(String),
    #[error("not found function by index: {0}")]
    NotFoundFunction(usize),
    #[error("not found table by index: {0}")]
    NotFoundTable(usize),
    #[error("undefined element")]
    UndefinedElement,
    #[error("uninitialized element {0}")]
    UninitializedElement(usize),
    #[error("not found function type by index: {0}")]
    NotFoundFuncType(usize),
    #[error("indirect call type mismatch")]
    TypeMismatchIndirectCall,
    #[error("not found type section")]
    NotFoundTypeSection,
    #[error("can not lock {0} for thread")]
    CanNotLockForThread(Resource),
}

#[derive(Debug)]
pub enum Resource {
    Global,
    Memory,
    Store,
    Table,
}

impl Display for Resource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Resource::Global => {
                write!(f, "global")
            },
            Resource::Memory => {
                write!(f, "memory")
            },
            Resource::Store => {
                write!(f, "store")
            }
            Resource::Table => {
                write!(f, "table")
            },
        }
    }
}
