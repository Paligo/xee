use ahash::HashMap;
use xee_schema_type::Xs;

#[derive(Debug, Clone, Default)]
pub struct TypeTable {
    types: HashMap<xot::Node, Xs>,
}

impl TypeTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.types.clear();
    }

    pub fn get(&self, node: xot::Node) -> Option<Xs> {
        self.types.get(&node).copied()
    }

    pub fn set(&mut self, node: xot::Node, xs: Xs) {
        self.types.insert(node, xs);
    }

    pub fn copy_type(&mut self, from: xot::Node, to: xot::Node) {
        if let Some(xs) = self.get(from) {
            self.set(to, xs);
        }
    }

    pub fn remove(&mut self, node: xot::Node) {
        self.types.remove(&node);
    }
}
