use ahash::{HashMap, HashMapExt};

use crate::{
    function,
    pattern::{ModeId, RuleEntry},
};
use xee_xpath_ast::Pattern;

#[derive(Debug, Clone)]
pub struct GlobalParam {
    pub name: xot::xmlname::OwnedName,
    pub required: bool,
    pub overrideable: bool,
    pub default: Option<function::InlineFunctionId>,
}

#[derive(Debug, Clone)]
pub struct TemplateParam {
    pub name: xot::xmlname::OwnedName,
    pub required: bool,
    pub default: Option<function::InlineFunctionId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatchError {
    Any,
    Namespace(String),
    Local(String),
    Name(xot::xmlname::OwnedName),
}

#[derive(Debug, Clone)]
pub struct CatchClause {
    pub errors: Vec<CatchError>,
}

#[derive(Debug, Clone)]
pub struct TryCatch {
    pub rollback_output: bool,
    pub catches: Vec<CatchClause>,
}

#[derive(Debug)]
pub struct Declarations {
    pub mode_rules: HashMap<ModeId, Vec<(Pattern<function::InlineFunctionId>, RuleEntry)>>,
    pub mode_configs: HashMap<ModeId, ModeConfig>,
    pub named_templates: HashMap<xot::xmlname::OwnedName, function::InlineFunctionId>,
    pub user_functions: Vec<function::InlineFunctionId>,
    pub global_params: Vec<GlobalParam>,
    pub template_params: HashMap<function::InlineFunctionId, Vec<TemplateParam>>,
    pub try_catches: Vec<TryCatch>,
}

impl Declarations {
    pub(crate) fn new() -> Self {
        Self {
            mode_rules: HashMap::new(),
            mode_configs: HashMap::new(),
            named_templates: HashMap::new(),
            user_functions: Vec::new(),
            global_params: Vec::new(),
            template_params: HashMap::new(),
            try_catches: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnNoMatch {
    DeepCopy,
    ShallowCopy,
    DeepSkip,
    ShallowSkip,
    TextOnlyCopy,
    Fail,
}

#[derive(Debug, Clone, Copy)]
pub struct ModeConfig {
    pub on_no_match: Option<OnNoMatch>,
}
