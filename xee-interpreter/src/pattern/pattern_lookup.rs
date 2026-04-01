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
    fn match_predicate_with_context(
        &mut self,
        inline_function_id: function::InlineFunctionId,
        item: &Item,
        position: usize,
        size: usize,
    ) -> bool {
        let function = function::InlineFunctionData::new(inline_function_id, Vec::new()).into();
        let arguments = [item.clone().into(), (position as u64).into(), (size as u64).into()];

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

    pub(crate) fn lookup_after(
        &self,
        current: &V,
        mut matches: impl FnMut(&Pattern<function::InlineFunctionId>) -> bool,
    ) -> Option<&V>
    where
        V: PartialEq,
    {
        let mut seen_current = false;
        for (pattern, value) in &self.patterns {
            if !matches(pattern) {
                continue;
            }
            if !seen_current {
                if value == current {
                    seen_current = true;
                }
                continue;
            }
            return Some(value);
        }
        None
    }
}
