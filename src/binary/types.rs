use super::instruction::Instruction;
use num_derive::FromPrimitive;

// https://webassembly.github.io/spec/core/binary/types.html#value-types
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    I32, // 0x7F
    I64, // 0x7E
    F32, // 0x7D
    F64, // 0x7C
}

impl From<u8> for ValueType {
    fn from(value_type: u8) -> Self {
        match value_type {
            0x7F => Self::I32,
            0x7E => Self::I64,
            0x7D => Self::F32,
            0x7C => Self::F64,
            _ => panic!("Invalid value type: {}", value_type),
        }
    }
}

// https://webassembly.github.io/spec/core/binary/types.html#function-types
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}

// https://webassembly.github.io/spec/core/binary/modules.html#binary-codesec
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct FunctionLocal {
    pub type_count: u32,
    pub value_type: ValueType,
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct FunctionBody {
    pub locals: Vec<FunctionLocal>,
    pub code: Vec<Instruction>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExportDesc {
    Func(u32),
    Table(u32),
    Memory(u32),
    Global(u32),
}

#[derive(Debug, PartialEq)]
pub struct Export {
    pub name: String,
    pub desc: ExportDesc,
}

#[derive(Debug, PartialEq, FromPrimitive)]
pub enum ElemType {
    FuncRef = 0x70,
}

#[derive(Debug, PartialEq)]
pub struct Table {
    pub elem_type: ElemType,
    pub limits: Limits,
}

#[derive(Debug, PartialEq)]
pub struct Memory {
    pub limits: Limits,
}

#[derive(Debug, PartialEq)]
pub struct Limits {
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug, PartialEq, FromPrimitive)]
pub enum Mutability {
    Const = 0x00,
    Var = 0x01,
}

#[derive(Debug, PartialEq)]
pub struct GlobalType {
    pub value_type: ValueType,
    pub mutability: Mutability,
}

#[derive(Debug, PartialEq)]
pub enum ExprValue {
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
}

#[derive(Debug, PartialEq)]
pub struct Global {
    pub global_type: GlobalType,
    pub init_expr: ExprValue,
}

#[derive(Debug, PartialEq)]
pub enum ImportKind {
    Func(u32),
    Table(Table),
    Memory(Memory),
    Global(GlobalType),
}

#[derive(Debug, PartialEq)]
pub struct Import {
    pub module_name: String,
    pub field_name: String,
    pub kind: ImportKind,
}
