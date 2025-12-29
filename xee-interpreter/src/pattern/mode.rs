use crate::function;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleEntry {
    pub function_id: function::InlineFunctionId,
    pub priority: rust_decimal::Decimal,
    pub import_level: u32,
    pub declaration_order: i64,
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModeId(usize);

impl ModeId {
    pub fn new(id: usize) -> Self {
        ModeId(id)
    }

    pub fn get(&self) -> usize {
        self.0
    }
}
