// https://webassembly.github.io/spec/core/binary/types.html#number-types
#[derive(Debug, Clone)]
pub enum NumberType {
    I32, // 0x7F
    I64, // 0x7E
    F32, // 0x7D
    F64, // 0x7C
}

// https://webassembly.github.io/spec/core/binary/types.html#value-types
#[derive(Debug, Clone)]
pub enum ValueType {
    NumberType(NumberType),
    Unknown(u8),
}

impl From<u8> for ValueType {
    fn from(value_type: u8) -> Self {
        match value_type {
            0x7F => Self::NumberType(NumberType::I32),
            0x7E => Self::NumberType(NumberType::I64),
            0x7D => Self::NumberType(NumberType::F32),
            0x7C => Self::NumberType(NumberType::F64),
            _ => Self::Unknown(value_type),
        }
    }
}

// https://webassembly.github.io/spec/core/binary/types.html#function-types
#[derive(Debug, Default, Clone)]
pub struct FuncType {
    pub params: Vec<ValueType>,
    pub results: Vec<ValueType>,
}
