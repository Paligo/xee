use xee_xpath_ast::Pattern;
use xot::Xot;

use crate::function;
use crate::interpreter::Interpreter;
use crate::pattern::pattern_core::PredicateMatcher;
use crate::sequence::Item;

#[derive(Debug, Default)]
pub struct PatternLookup<V: Clone> {
    pub(crate) patterns: Vec<(Pattern<function::InlineFunctionId>, V)>,
}

pub(crate) struct InterpreterPredicateMatcher<'a> {
    interpreter: &'a mut Interpreter<'a>,
}

impl<'a> InterpreterPredicateMatcher<'a> {
    pub(crate) fn new(interpreter: &'a mut Interpreter<'a>) -> Self {
        Self { interpreter }
    }
}

impl PredicateMatcher for Interpreter<'_> {
    fn match_predicate(
        &mut self,
        inline_function_id: function::InlineFunctionId,
        item: &Item,
    ) -> bool {
        // TODO: extract 'call_function_id_with_arguments' that is used also by
        // apply_templates_sequence. call it with context, position and length,
        // again see apply_templates_sequence
        let function = function::InlineFunctionData::new(inline_function_id, Vec::new()).into();
        let arguments = if let Item::Node(node) = item {
            if let Some(parent) = self.xot().parent(*node) {
                let position = self.xot().child_index(parent, *node).unwrap() + 1;
                let size = self.xot().children(parent).count();
                [
                    item.clone().into(),
                    (position as u64).into(),
                    (size as u64).into(),
                ]
            } else {
                [item.clone().into(), 1.into(), 1.into()]
            }
        } else {
            [item.clone().into(), 1.into(), 1.into()]
        };

        // the specification says to swallow any errors
        // TODO: log errors somehow here?
        let value = self.call_function_with_arguments(&function, &arguments);
        if let Ok(value) = value {
            value.effective_boolean_value().unwrap_or(false)
        } else {
            false
        }
    }

    fn xot(&self) -> &Xot {
        self.xot()
    }
}

impl<V: Clone> PatternLookup<V> {
    pub(crate) fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    pub(crate) fn add_rules(&mut self, rules: Vec<(Pattern<function::InlineFunctionId>, V)>) {
        self.patterns.extend(rules);
    }

    pub(crate) fn lookup(
        &self,
        mut matches: impl FnMut(&Pattern<function::InlineFunctionId>) -> bool,
    ) -> Option<&V> {
        self.patterns
            .iter()
            .find(|(pattern, _)| matches(pattern))
            .map(|(_, value)| value)
    }
}
