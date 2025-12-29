use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use rust_decimal::Decimal;
use xee_interpreter::declaration::{ModeConfig, OnNoMatch};
use xee_interpreter::pattern::{ModeId, RuleEntry};
use xee_xpath_ast::Pattern;

use crate::function_compiler::Scopes;
use crate::{ir, FunctionBuilder, FunctionCompiler};

use xee_interpreter::{
    declaration::{GlobalParam, TemplateParam},
    error, function, interpreter,
};
use xee_xpath_ast::pattern::transform_pattern;

#[derive(Debug, Clone)]
pub(crate) struct RuleBuilder {
    priority: Decimal,
    declaration_order: i64,
    import_level: u32,
    is_builtin: bool,
    pattern: Pattern<function::InlineFunctionId>,
    function_id: function::InlineFunctionId,
}

impl RuleBuilder {
    fn rule(self) -> (Pattern<function::InlineFunctionId>, RuleEntry) {
        (
            self.pattern,
            RuleEntry {
                function_id: self.function_id,
                priority: self.priority,
                import_level: self.import_level,
                declaration_order: self.declaration_order,
                is_builtin: self.is_builtin,
            },
        )
    }
}

pub type ModeIds = HashMap<ir::ApplyTemplatesModeValue, ModeId>;

pub struct DeclarationCompiler<'a> {
    program: &'a mut interpreter::Program,
    scopes: Scopes,
    rule_declaration_order: i64,
    rule_builders: HashMap<ir::ModeValue, Vec<RuleBuilder>>,
    mode_ids: ModeIds,
    user_function_ids: Vec<function::InlineFunctionId>,
    named_template_ids: HashMap<xot::xmlname::OwnedName, function::InlineFunctionId>,
}

impl<'a> DeclarationCompiler<'a> {
    pub fn new(program: &'a mut interpreter::Program) -> Self {
        Self {
            program,
            scopes: Scopes::new(),
            rule_declaration_order: 0,
            rule_builders: HashMap::new(),
            mode_ids: HashMap::new(),
            user_function_ids: Vec::new(),
            named_template_ids: HashMap::new(),
        }
    }

    fn function_compiler(&mut self) -> FunctionCompiler<'_> {
        let function_builder = FunctionBuilder::new(self.program);
        FunctionCompiler::new(
            function_builder,
            &mut self.scopes,
            &self.mode_ids,
            &self.user_function_ids,
            &self.named_template_ids,
        )
    }

    pub fn compile_declarations(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        // first keep track of what modes exist, to create a ModeId for them. We do
        // this early so any mode reference within apply-templates will resolve.
        self.compile_modes(declarations);
        self.compile_mode_configs(declarations);

        self.prepare_user_function_ids(declarations)?;
        self.prepare_named_template_ids(declarations)?;
        self.compile_user_functions(declarations)?;
        self.compile_named_templates(declarations)?;

        for rule in &declarations.rules {
            self.compile_rule(rule)?;
        }
        self.compile_global_params(declarations)?;
        // now add compiled rules from builder to the program
        self.add_rules();
        let mut function_compiler = self.function_compiler();
        function_compiler.compile_function_definition(&declarations.main, (0..0).into())
    }

    fn compile_modes(&mut self, declarations: &ir::Declarations) {
        for rule in &declarations.rules {
            for mode_value in &rule.modes {
                // we don't register All modes
                if matches!(mode_value, ir::ModeValue::All) {
                    continue;
                }
                let apply_templates_mode_value = match mode_value {
                    ir::ModeValue::All => continue,
                    ir::ModeValue::Named(name) => ir::ApplyTemplatesModeValue::Named(name.clone()),
                    ir::ModeValue::Unnamed => ir::ApplyTemplatesModeValue::Unnamed,
                };
                // we want the mode id to be unique and not overwritten
                if self.mode_ids.contains_key(&apply_templates_mode_value) {
                    continue;
                }
                let mode_id = ModeId::new(self.mode_ids.len());
                self.mode_ids.insert(apply_templates_mode_value, mode_id);
            }
        }
        for (mode_name, _) in &declarations.modes {
            let apply_templates_mode_value = match mode_name {
                Some(name) => ir::ApplyTemplatesModeValue::Named(name.clone()),
                None => ir::ApplyTemplatesModeValue::Unnamed,
            };
            if self.mode_ids.contains_key(&apply_templates_mode_value) {
                continue;
            }
            let mode_id = ModeId::new(self.mode_ids.len());
            self.mode_ids.insert(apply_templates_mode_value, mode_id);
        }
    }

    fn compile_mode_configs(&mut self, declarations: &ir::Declarations) {
        self.program.declarations.mode_configs.clear();
        for (mode_name, mode) in &declarations.modes {
            let apply_templates_mode_value = match mode_name {
                Some(name) => ir::ApplyTemplatesModeValue::Named(name.clone()),
                None => ir::ApplyTemplatesModeValue::Unnamed,
            };
            if let Some(mode_id) = self.mode_ids.get(&apply_templates_mode_value).cloned() {
                let on_no_match = mode.on_no_match.as_ref().map(|m| match m {
                    ir::OnNoMatch::DeepCopy => OnNoMatch::DeepCopy,
                    ir::OnNoMatch::ShallowCopy => OnNoMatch::ShallowCopy,
                    ir::OnNoMatch::DeepSkip => OnNoMatch::DeepSkip,
                    ir::OnNoMatch::ShallowSkip => OnNoMatch::ShallowSkip,
                    ir::OnNoMatch::TextOnlyCopy => OnNoMatch::TextOnlyCopy,
                    ir::OnNoMatch::Fail => OnNoMatch::Fail,
                });
                self.program
                    .declarations
                    .mode_configs
                    .insert(mode_id, ModeConfig { on_no_match });
            }
        }
    }

    fn prepare_user_function_ids(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        self.user_function_ids.clear();
        if declarations.functions.is_empty() {
            self.program.declarations.user_functions.clear();
            return Ok(());
        }
        let start_index = self
            .program
            .reserve_function_slots(declarations.functions.len());
        for offset in 0..declarations.functions.len() {
            let index = start_index + offset;
            if index > u16::MAX as usize {
                return Err(error::Error::Unsupported(
                    "Too many user functions".to_string(),
                )
                .into());
            }
            self.user_function_ids
                .push(function::InlineFunctionId::new(index));
        }
        self.program
            .declarations
            .user_functions
            .clone_from(&self.user_function_ids);
        Ok(())
    }

    fn compile_user_functions(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        if declarations.functions.is_empty() {
            return Ok(());
        }
        for (index, function_binding) in declarations.functions.iter().enumerate() {
            let expected_id = self.user_function_ids[index];
            let mut function_compiler = self.function_compiler();
            function_compiler
                .compile_function_id_at(&function_binding.main, expected_id, (0..0).into())?;
        }
        Ok(())
    }

    fn prepare_named_template_ids(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        self.named_template_ids.clear();
        self.program.declarations.named_templates.clear();
        if declarations.named_templates.is_empty() {
            return Ok(());
        }
        let start_index = self
            .program
            .reserve_function_slots(declarations.named_templates.len());
        for (offset, template) in declarations.named_templates.iter().enumerate() {
            let index = start_index + offset;
            if index > u16::MAX as usize {
                return Err(error::Error::Unsupported(
                    "Too many named templates".to_string(),
                )
                .into());
            }
            let id = function::InlineFunctionId::new(index);
            if self
                .named_template_ids
                .insert(template.name.clone(), id)
                .is_some()
            {
                return Err(error::Error::Unsupported(
                    "Duplicate named template".to_string(),
                )
                .into());
            }
        }
        self.program
            .declarations
            .named_templates
            .clone_from(&self.named_template_ids);
        Ok(())
    }

    fn compile_named_templates(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        if declarations.named_templates.is_empty() {
            return Ok(());
        }
        for template in &declarations.named_templates {
            let expected_id = self
                .named_template_ids
                .get(&template.name)
                .copied()
                .ok_or_else(|| {
                    error::Error::Unsupported(String::from("Named template not registered"))
                })?;
            let mut function_compiler = self.function_compiler();
            function_compiler.compile_function_id_at(
                &template.function_definition,
                expected_id,
                (0..0).into(),
            )?;
            let compiled_template_params = Self::build_template_params(
                &template.template_params,
                &template.function_definition,
                &mut function_compiler,
            )?;
            if !compiled_template_params.is_empty() {
                self.program
                    .declarations
                    .template_params
                    .insert(expected_id, compiled_template_params);
            }
        }
        Ok(())
    }

    fn compile_rule(&mut self, rule: &ir::Rule) -> error::SpannedResult<()> {
        let (function_id, pattern, compiled_template_params) = {
            let mut function_compiler = self.function_compiler();
            let function_id =
                function_compiler.compile_function_id(&rule.function_definition, (0..0).into())?;

            let pattern = transform_pattern(&rule.pattern, |function_definition| {
                function_compiler.compile_function_id(function_definition, (0..0).into())
            })?;

            let compiled_template_params = Self::build_template_params(
                &rule.template_params,
                &rule.function_definition,
                &mut function_compiler,
            )?;
            (function_id, pattern, compiled_template_params)
        };
        if !compiled_template_params.is_empty() {
            self.program
                .declarations
                .template_params
                .insert(function_id, compiled_template_params);
        }
        self.add_rule(
            &rule.modes,
            rule.priority,
            rule.import_level,
            rule.is_builtin,
            &pattern,
            function_id,
        );
        Ok(())
    }

    fn add_rule(
        &mut self,
        modes: &[ir::ModeValue],
        priority: Decimal,
        import_level: u32,
        is_builtin: bool,
        pattern: &Pattern<function::InlineFunctionId>,
        function_id: function::InlineFunctionId,
    ) {
        // ensure there are no duplicate modes
        let mut mode_seen = HashSet::new();

        let declaration_order = self.rule_declaration_order;
        self.rule_declaration_order += 1;
        for mode in modes {
            if mode_seen.contains(mode) {
                continue;
            }
            mode_seen.insert(mode);
            self.rule_builders
                .entry(mode.clone())
                .or_default()
                .push(RuleBuilder {
                    priority,
                    declaration_order,
                    import_level,
                    is_builtin,
                    pattern: pattern.clone(),
                    function_id,
                });
        }
    }

    fn add_rules(&mut self) {
        // we don't want to register #all normally
        let all_rule_builders = self.rule_builders.remove(&ir::ModeValue::All);

        // we add the all rule builders to each rule builders, as they apply to
        // all modes. We do this before the final registration so we benefit
        // from priority sorting later
        if let Some(all_rule_builders) = all_rule_builders {
            for rule_builders in self.rule_builders.values_mut() {
                for all_rule_builder in &all_rule_builders {
                    rule_builders.push(all_rule_builder.clone());
                }
            }
        }

        for (mode, mut rule_builders) in self.rule_builders.drain() {
            // higher priorities first; lower import_level (higher precedence) first;
            // same priorities + import_level -> last declaration order wins
            rule_builders.sort_by_key(|rule_builder| {
                (
                    -rule_builder.priority,
                    rule_builder.import_level as i64,
                    -rule_builder.declaration_order,
                )
            });
            let rules = rule_builders
                .drain(..)
                .map(|rule_builder| rule_builder.rule())
                .collect::<Vec<_>>();
            let apply_templates_mode_value = match mode {
                ir::ModeValue::Named(name) => ir::ApplyTemplatesModeValue::Named(name),
                ir::ModeValue::Unnamed => ir::ApplyTemplatesModeValue::Unnamed,
                ir::ModeValue::All => {
                    unreachable!()
                }
            };
            let mode_id = self
                .mode_ids
                .get(&apply_templates_mode_value)
                .cloned()
                .expect("Mode should have been registered");
            self.program
                .declarations
                .mode_rules
                .insert(mode_id, rules);
        }
    }

    fn compile_global_params(&mut self, declarations: &ir::Declarations) -> error::SpannedResult<()> {
        if declarations.global_params.is_empty() {
            return Ok(());
        }
        let compiled = {
            let mut function_compiler = self.function_compiler();
            let params = declarations
                .global_params
                .iter()
                .map(|param| ir::Param {
                    name: param.var_name.clone(),
                    type_: None,
                })
                .collect::<Vec<_>>();
            let mut compiled = Vec::with_capacity(declarations.global_params.len());
            for global_param in &declarations.global_params {
                let default_function = if let Some(default_expr) = &global_param.default_expr {
                    let function_definition = ir::FunctionDefinition {
                        params: params.clone(),
                        return_type: None,
                        body: Box::new(default_expr.clone()),
                    };
                    Some(
                        function_compiler.compile_function_id(&function_definition, (0..0).into())?,
                    )
                } else {
                    None
                };
                compiled.push(GlobalParam {
                    name: global_param.name.clone(),
                    required: global_param.required,
                    overrideable: global_param.overrideable,
                    default: default_function,
                });
            }
            compiled
        };
        self.program
            .declarations
            .global_params
            .extend(compiled);
        Ok(())
    }

    fn build_template_params(
        template_params: &[ir::TemplateParam],
        function_definition: &ir::FunctionDefinition,
        function_compiler: &mut FunctionCompiler<'_>,
    ) -> error::SpannedResult<Vec<TemplateParam>> {
        if template_params.is_empty() {
            return Ok(Vec::new());
        }
        let mut params = function_definition.params.clone();
        for param in &mut params {
            param.type_ = None;
        }
        let mut compiled = Vec::with_capacity(template_params.len());
        for template_param in template_params {
            let default = if let Some(default_expr) = &template_param.default_expr {
                let function_definition = ir::FunctionDefinition {
                    params: params.clone(),
                    return_type: None,
                    body: Box::new(default_expr.clone()),
                };
                Some(function_compiler.compile_function_id(&function_definition, (0..0).into())?)
            } else {
                None
            };
            compiled.push(TemplateParam {
                name: template_param.name.clone(),
                required: template_param.required,
                default,
            });
        }
        Ok(compiled)
    }
}
