// https://webassembly.github.io/spec/core/binary/types.html#value-types
#[derive(Debug, Clone)]
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
#[derive(Debug, Default, Clone)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}
