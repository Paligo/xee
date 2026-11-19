use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use rust_decimal::Decimal;
use xee_interpreter::pattern::ModeId;
use xee_xpath_ast::Pattern;

use crate::function_compiler::Scopes;
use crate::{ir, FunctionBuilder, FunctionCompiler};

use xee_interpreter::{error, function, interpreter};
use xee_xpath_ast::pattern::transform_pattern;

#[derive(Debug, Clone)]
pub(crate) struct RuleBuilder {
    priority: Decimal,
    declaration_order: i64,
    pattern: Pattern<function::InlineFunctionId>,
    function_id: function::InlineFunctionId,
}

impl RuleBuilder {
    fn rule(
        self,
    ) -> (
        Pattern<function::InlineFunctionId>,
        function::InlineFunctionId,
    ) {
        (self.pattern, self.function_id)
    }
}

pub type ModeIds = HashMap<ir::ApplyTemplatesModeValue, ModeId>;
pub type TemplateIds = HashMap<String, u16>;
pub type TemplateParams = HashMap<String, Vec<ir::Param>>;

pub struct DeclarationCompiler<'a> {
    program: &'a mut interpreter::Program,
    scopes: Scopes,
    rule_declaration_order: i64,
    rule_builders: HashMap<ir::ModeValue, Vec<RuleBuilder>>,
    mode_ids: ModeIds,
    template_ids: TemplateIds,
    template_params: TemplateParams,
    global_variable_ids: HashMap<ir::Name, u16>,
}

impl<'a> DeclarationCompiler<'a> {
    pub fn new(program: &'a mut interpreter::Program) -> Self {
        Self {
            program,
            scopes: Scopes::new(),
            rule_declaration_order: 0,
            rule_builders: HashMap::new(),
            mode_ids: HashMap::new(),
            template_ids: HashMap::new(),
            template_params: HashMap::new(),
            global_variable_ids: HashMap::new(),
        }
    }

    fn function_compiler(&mut self) -> FunctionCompiler<'_> {
        let function_builder = FunctionBuilder::new(self.program);
        FunctionCompiler::new(
            function_builder,
            &mut self.scopes,
            &self.mode_ids,
            &self.template_ids,
            &self.template_params,
            &self.global_variable_ids,
        )
    }

    pub fn compile_declarations(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        // first keep track of what modes exist, to create a ModeId for them. We do
        // this early so any mode reference within apply-templates will resolve.
        self.compile_modes(declarations);
        self.register_global_variables(declarations)?;
        self.register_templates(declarations)?;

        // compile all named templates (function bindings) early so they can be referenced
        // by name from call-template instructions
        self.compile_templates(declarations)?;
        self.compile_global_variables(declarations)?;

        for rule in &declarations.rules {
            self.compile_rule(rule)?;
        }
        // now add compiled rules from builder to the program
        self.add_rules();
        let mut function_compiler = self.function_compiler();
        function_compiler.compile_function_definition(&declarations.main, (0..0).into())
    }

    fn register_global_variables(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        for (index, global_variable) in declarations.global_variables.iter().enumerate() {
            if index > u16::MAX as usize {
                return Err(
                    error::Error::Unsupported("too many global variables".to_string()).into(),
                );
            }
            self.global_variable_ids
                .insert(global_variable.name.clone(), index as u16);
        }
        Ok(())
    }

    fn compile_modes(&mut self, declarations: &ir::Declarations) {
        self.register_mode(ir::ApplyTemplatesModeValue::Unnamed);

        for mode_name in declarations.modes.keys() {
            let mode = match mode_name {
                Some(name) => ir::ApplyTemplatesModeValue::Named(name.clone()),
                None => ir::ApplyTemplatesModeValue::Unnamed,
            };
            self.register_mode(mode);
        }

        for rule in &declarations.rules {
            for mode_value in &rule.modes {
                // we don't register All modes
                if matches!(mode_value, ir::ModeValue::All) {
                    continue;
                }
                self.register_mode(match mode_value {
                    ir::ModeValue::All => continue,
                    ir::ModeValue::Named(name) => ir::ApplyTemplatesModeValue::Named(name.clone()),
                    ir::ModeValue::Unnamed => ir::ApplyTemplatesModeValue::Unnamed,
                });
            }
        }

        for (mode, mode_id) in &self.mode_ids {
            let declaration = match mode {
                ir::ApplyTemplatesModeValue::Named(name) => declarations
                    .modes
                    .get(&Some(name.clone()))
                    .map(Self::mode_declaration)
                    .unwrap_or_default(),
                ir::ApplyTemplatesModeValue::Unnamed => declarations
                    .modes
                    .get(&None)
                    .map(Self::mode_declaration)
                    .unwrap_or_default(),
                ir::ApplyTemplatesModeValue::Current => continue,
            };
            self.program.declarations.add_mode(*mode_id, declaration);
        }
    }

    fn mode_declaration(mode: &ir::Mode) -> xee_interpreter::declaration::ModeDeclaration {
        xee_interpreter::declaration::ModeDeclaration {
            on_no_match: match mode.on_no_match {
                ir::ModeOnNoMatch::DeepCopy => {
                    xee_interpreter::declaration::ModeOnNoMatch::DeepCopy
                }
                ir::ModeOnNoMatch::ShallowCopy => {
                    xee_interpreter::declaration::ModeOnNoMatch::ShallowCopy
                }
                ir::ModeOnNoMatch::DeepSkip => {
                    xee_interpreter::declaration::ModeOnNoMatch::DeepSkip
                }
                ir::ModeOnNoMatch::ShallowSkip => {
                    xee_interpreter::declaration::ModeOnNoMatch::ShallowSkip
                }
                ir::ModeOnNoMatch::TextOnlyCopy => {
                    xee_interpreter::declaration::ModeOnNoMatch::TextOnlyCopy
                }
                ir::ModeOnNoMatch::Fail => xee_interpreter::declaration::ModeOnNoMatch::Fail,
            },
            warning_on_no_match: mode.warning_on_no_match,
            typed: match mode.typed {
                ir::ModeTyped::Yes => xee_interpreter::declaration::ModeTyped::Yes,
                ir::ModeTyped::No => xee_interpreter::declaration::ModeTyped::No,
                ir::ModeTyped::Strict => xee_interpreter::declaration::ModeTyped::Strict,
                ir::ModeTyped::Lax => xee_interpreter::declaration::ModeTyped::Lax,
            },
        }
    }

    fn register_mode(&mut self, mode: ir::ApplyTemplatesModeValue) {
        if self.mode_ids.contains_key(&mode) {
            return;
        }
        let mode_id = ModeId::new(self.mode_ids.len());
        self.mode_ids.insert(mode, mode_id);
    }

    fn compile_templates(&mut self, declarations: &ir::Declarations) -> error::SpannedResult<()> {
        for function_binding in &declarations.functions {
            self.compile_named_template(function_binding)?;
        }
        Ok(())
    }

    fn register_templates(&mut self, declarations: &ir::Declarations) -> error::SpannedResult<()> {
        for (index, function_binding) in declarations.functions.iter().enumerate() {
            if index > u16::MAX as usize {
                return Err(
                    error::Error::Unsupported("too many named templates".to_string()).into(),
                );
            }
            let template_name_key = function_binding.name.as_str().to_string();
            self.template_ids
                .insert(template_name_key.clone(), index as u16);
            self.template_params.insert(
                template_name_key,
                function_binding
                    .main
                    .params
                    .iter()
                    .skip(3)
                    .cloned()
                    .collect(),
            );
        }
        Ok(())
    }

    fn compile_global_variables(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        for global_variable in &declarations.global_variables {
            self.compile_global_variable(global_variable)?;
        }
        Ok(())
    }

    fn compile_global_variable(
        &mut self,
        global_variable: &ir::GlobalVariable,
    ) -> error::SpannedResult<()> {
        let mut function_compiler = self.function_compiler();
        let function_definition = ir::FunctionDefinition {
            params: global_variable.params.clone(),
            return_type: None,
            body: Box::new(global_variable.expr.clone()),
        };
        let function_id = function_compiler
            .compile_function_id(&function_definition, global_variable.expr.span.into())?;
        self.program.declarations.add_global_variable(
            xee_interpreter::declaration::GlobalVariableDeclaration {
                name: global_variable.name.clone(),
                function_id,
                original_name: global_variable.original_name.clone(),
                required: global_variable.required,
            },
        );
        Ok(())
    }

    fn compile_named_template(
        &mut self,
        function_binding: &ir::FunctionBinding,
    ) -> error::SpannedResult<()> {
        let mut function_compiler = self.function_compiler();
        let function_id =
            function_compiler.compile_function_id(&function_binding.main, (0..0).into())?;
        let template_params = function_binding
            .main
            .params
            .iter()
            .skip(3)
            .map(
                |param| xee_interpreter::declaration::TemplateParamDeclaration {
                    name: param
                        .original_name
                        .clone()
                        .unwrap_or_else(|| param.name.as_str().to_string()),
                    tunnel: param.tunnel,
                },
            )
            .collect::<Vec<_>>();
        if !template_params.is_empty() {
            self.program
                .declarations
                .add_template_params(function_id, template_params);
        }
        self.program.declarations.add_named_template(
            xee_interpreter::declaration::NamedTemplateDeclaration {
                name: function_binding.name.clone(),
                function_id,
            },
        );
        Ok(())
    }

    fn compile_rule(&mut self, rule: &ir::Rule) -> error::SpannedResult<()> {
        let mut function_compiler = self.function_compiler();
        let function_id =
            function_compiler.compile_function_id(&rule.function_definition, (0..0).into())?;

        let pattern = transform_pattern(&rule.pattern, |function_definition| {
            function_compiler.compile_function_id(function_definition, (0..0).into())
        })?;

        drop(function_compiler);

        let template_params = rule
            .function_definition
            .params
            .iter()
            .skip(3)
            .map(
                |param| xee_interpreter::declaration::TemplateParamDeclaration {
                    name: param
                        .original_name
                        .clone()
                        .unwrap_or_else(|| param.name.as_str().to_string()),
                    tunnel: param.tunnel,
                },
            )
            .collect::<Vec<_>>();
        if !template_params.is_empty() {
            self.program
                .declarations
                .add_template_params(function_id, template_params);
        }

        self.add_rule(&rule.modes, rule.priority, &pattern, function_id);
        Ok(())
    }

    fn add_rule(
        &mut self,
        modes: &[ir::ModeValue],
        priority: Decimal,
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
            let all_modes = self.mode_ids.keys().cloned().collect::<Vec<_>>();

            for mode in all_modes {
                let mode_value = match mode {
                    ir::ApplyTemplatesModeValue::Named(name) => ir::ModeValue::Named(name),
                    ir::ApplyTemplatesModeValue::Unnamed => ir::ModeValue::Unnamed,
                    ir::ApplyTemplatesModeValue::Current => continue,
                };
                self.rule_builders
                    .entry(mode_value)
                    .or_default()
                    .extend(all_rule_builders.iter().cloned());
            }
        }

        for (mode, mut rule_builders) in self.rule_builders.drain() {
            // higher priorities first, same priorities last declaration order wins
            rule_builders.sort_by_key(|rule_builder| {
                (-rule_builder.priority, -rule_builder.declaration_order)
            });
            let rules = rule_builders
                .drain(..)
                .map(|rule_builder| rule_builder.rule())
                .collect();
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
                .mode_lookup
                .add_rules(mode_id, rules)
        }
    }
}
