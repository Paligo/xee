use ahash::{HashMap, HashMapExt};

use crate::{function, pattern::ModeLookup};
use xot::xmlname::OwnedName;

#[derive(Debug, Clone)]
pub struct GlobalVariableDeclaration {
    pub name: function::Name,
    pub function_id: function::InlineFunctionId,
    pub original_name: Option<OwnedName>,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct NamedTemplateDeclaration {
    pub name: function::Name,
    pub function_id: function::InlineFunctionId,
}

#[derive(Debug, Clone)]
pub struct TemplateParamDeclaration {
    pub name: String,
    pub tunnel: bool,
}

#[derive(Debug)]
pub struct Declarations {
    pub mode_lookup: ModeLookup<function::InlineFunctionId>,
    pub global_variables: Vec<GlobalVariableDeclaration>,
    pub named_templates: Vec<NamedTemplateDeclaration>,
    template_params: HashMap<function::InlineFunctionId, Vec<TemplateParamDeclaration>>,
}

impl Declarations {
    pub(crate) fn new() -> Self {
        Self {
            mode_lookup: ModeLookup::new(),
            global_variables: Vec::new(),
            named_templates: Vec::new(),
            template_params: HashMap::new(),
        }
    }

    pub fn add_global_variable(&mut self, global_variable: GlobalVariableDeclaration) {
        self.global_variables.push(global_variable);
    }

    pub fn global_variable(&self, index: usize) -> &GlobalVariableDeclaration {
        &self.global_variables[index]
    }

    pub fn add_named_template(&mut self, named_template: NamedTemplateDeclaration) {
        self.named_templates.push(named_template);
    }

    pub fn named_template(&self, index: usize) -> &NamedTemplateDeclaration {
        &self.named_templates[index]
    }

    pub fn named_template_by_name(&self, name: &str) -> Option<&NamedTemplateDeclaration> {
        self.named_templates
            .iter()
            .find(|named_template| named_template.name == function::Name::new(name.to_string()))
    }

    pub fn add_template_params(
        &mut self,
        function_id: function::InlineFunctionId,
        params: Vec<TemplateParamDeclaration>,
    ) {
        self.template_params.insert(function_id, params);
    }

    pub fn template_params(
        &self,
        function_id: function::InlineFunctionId,
    ) -> Option<&[TemplateParamDeclaration]> {
        self.template_params.get(&function_id).map(Vec::as_slice)
    }
}
