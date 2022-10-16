use crate::{
    section::{Export, FunctionBody, Section},
    types::FuncType,
};

#[derive(Debug, Default)]
pub struct Module {
    pub magic: String,
    pub version: u32,
    pub type_section: Option<Vec<FuncType>>,
    pub function_section: Option<Vec<u32>>,
    pub code_section: Option<Vec<FunctionBody>>,
    pub export_section: Option<Vec<Export>>,
}

impl Module {
    pub fn add_section(&mut self, section: Section) {
        match section {
            Section::Type(section) => self.type_section = Some(section),
            Section::Function(section) => self.function_section = Some(section),
            Section::Code(section) => self.code_section = Some(section),
            Section::Export(section) => self.export_section = Some(section),
        };
    }
}
