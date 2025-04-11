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

pub struct DeclarationCompiler<'a> {
    program: &'a mut interpreter::Program,
    scopes: Scopes,
    rule_declaration_order: i64,
    rule_builders: HashMap<ir::ModeValue, Vec<RuleBuilder>>,
    mode_ids: ModeIds,
}

impl<'a> DeclarationCompiler<'a> {
    pub fn new(program: &'a mut interpreter::Program) -> Self {
        Self {
            program,
            scopes: Scopes::new(),
            rule_declaration_order: 0,
            rule_builders: HashMap::new(),
            mode_ids: HashMap::new(),
        }
    }

    fn function_compiler(&mut self) -> FunctionCompiler<'_> {
        let function_builder = FunctionBuilder::new(self.program);
        FunctionCompiler::new(function_builder, &mut self.scopes, &self.mode_ids)
    }

    pub fn compile_declarations(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        // first keep track of what modes exist, to create a ModeId for them. We do
        // this early so any mode reference within apply-templates will resolve.
        self.compile_modes(declarations);

        for rule in &declarations.rules {
            self.compile_rule(rule)?;
        }
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
    }

    fn compile_rule(&mut self, rule: &ir::Rule) -> error::SpannedResult<()> {
        let mut function_compiler = self.function_compiler();
        let function_id =
            function_compiler.compile_function_id(&rule.function_definition, (0..0).into())?;

        let pattern = transform_pattern(&rule.pattern, |function_definition| {
            function_compiler.compile_function_id(function_definition, (0..0).into())
        })?;

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
            for rule_builders in self.rule_builders.values_mut() {
                for all_rule_builder in &all_rule_builders {
                    rule_builders.push(all_rule_builder.clone());
                }
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
