use ahash::{HashMap, HashMapExt};
use std::fmt::{Debug, Formatter};
use xot::xmlname::NameStrInfo;

use xee_name::{Name, Namespaces};
use xee_xpath_ast::ast;

use crate::context::DynamicContext;
use crate::error;
use crate::function;
use crate::interpreter;
use crate::library::static_function_descriptions;
use crate::sequence;
use crate::stack;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Copy)]
pub(crate) enum FunctionKind {
    // generate a function with one less arity that takes the
    // item as the first argument
    ItemFirst,
    // generate a function with one less arity that takes the item
    // as the last argument
    ItemLast,
    // generate just one function, but it takes an additional last
    // argument that contains an option of the context item
    ItemLastOptional,
    // this function takes position as the implicit only argument
    Position,
    // this function takes size as the implicit only argument
    Size,
    // generate a function with one less arity that takes the collation
    // as the last argument
    Collation,
    // this function is anonymous and takes a closure value as the
    // last argument
    AnonymousClosure,
}

impl FunctionKind {
    pub(crate) fn parse(s: &str) -> Option<FunctionKind> {
        match s {
            "" => None,
            "context_first" => Some(FunctionKind::ItemFirst),
            "context_last" => Some(FunctionKind::ItemLast),
            "context_last_optional" => Some(FunctionKind::ItemLastOptional),
            "position" => Some(FunctionKind::Position),
            "size" => Some(FunctionKind::Size),
            "collation" => Some(FunctionKind::Collation),
            "anonymous_closure" => Some(FunctionKind::AnonymousClosure),
            _ => panic!("Unknown function kind {}", s),
        }
    }
}

pub(crate) type StaticFunctionType = fn(
    context: &DynamicContext,
    interpreter: &mut interpreter::Interpreter,
    arguments: &[sequence::Sequence],
) -> error::Result<sequence::Sequence>;

pub(crate) struct StaticFunctionDescription {
    pub(crate) name: Name,
    pub(crate) signature: function::Signature,
    pub(crate) function_kind: Option<FunctionKind>,
    pub(crate) func: StaticFunctionType,
}

// Wraps a Rust function annotated with `#[xpath_fn]` and turns it
// into a StaticFunctionDescription
#[macro_export]
macro_rules! wrap_xpath_fn {
    ($function:path) => {{
        use $function as wrapped_function;
        let namespaces = &xee_name::DEFAULT_NAMESPACES;
        $crate::function::StaticFunctionDescription::new(
            wrapped_function::WRAPPER,
            wrapped_function::SIGNATURE,
            $crate::function::FunctionKind::parse(wrapped_function::KIND),
            namespaces,
        )
    }};
}

impl StaticFunctionDescription {
    pub(crate) fn new(
        func: StaticFunctionType,
        signature: &str,
        function_kind: Option<FunctionKind>,
        namespaces: &Namespaces,
    ) -> Self {
        // TODO reparse signature; the macro could have stored the parsed
        // version as code, but that's more work than I'm prepared to do
        // right now.
        let signature = ast::Signature::parse(signature, namespaces)
            .expect("Signature parse failed unexpectedly");
        let name = signature.name.value.clone();
        let signature: function::Signature = signature.into();
        Self {
            name,
            signature,
            function_kind,
            func,
        }
    }

    fn functions(&self) -> Vec<StaticFunction> {
        if let Some(function_kind) = &self.function_kind {
            self.signature
                .alternative_signatures(*function_kind)
                .into_iter()
                .map(|(signature, function_kind)| {
                    StaticFunction::new(self.func, self.name.clone(), signature, function_kind)
                })
                .collect()
        } else {
            vec![StaticFunction::new(
                self.func,
                self.name.clone(),
                self.signature.clone(),
                None,
            )]
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FunctionRule {
    ItemFirst,
    ItemLast,
    ItemLastOptional,
    PositionFirst,
    SizeFirst,
    Collation,
    AnonymousClosure,
}

impl From<FunctionKind> for FunctionRule {
    fn from(function_kind: FunctionKind) -> Self {
        match function_kind {
            FunctionKind::ItemFirst => FunctionRule::ItemFirst,
            FunctionKind::ItemLast => FunctionRule::ItemLast,
            FunctionKind::ItemLastOptional => FunctionRule::ItemLastOptional,
            FunctionKind::Position => FunctionRule::PositionFirst,
            FunctionKind::Size => FunctionRule::SizeFirst,
            FunctionKind::Collation => FunctionRule::Collation,
            FunctionKind::AnonymousClosure => FunctionRule::AnonymousClosure,
        }
    }
}

pub struct StaticFunction {
    name: Name,
    signature: function::Signature,
    arity: usize,
    pub function_rule: Option<FunctionRule>,
    func: StaticFunctionType,
}

impl Debug for StaticFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StaticFunction")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("function_rule", &self.function_rule)
            .finish()
    }
}

impl StaticFunction {
    pub(crate) fn new(
        func: StaticFunctionType,
        name: Name,
        signature: function::Signature,
        function_kind: Option<FunctionKind>,
    ) -> Self {
        let function_rule = function_kind.map(|k| k.into());
        let arity = signature.arity();
        Self {
            name,
            signature,
            arity,
            function_rule,
            func,
        }
    }

    pub(crate) fn needs_context(&self) -> bool {
        match self.function_rule {
            None | Some(FunctionRule::Collation) => false,
            Some(_) => true,
        }
    }

    pub(crate) fn invoke(
        &self,
        context: &DynamicContext,
        interpreter: &mut interpreter::Interpreter,
        arguments: Vec<sequence::Sequence>,
        closure_values: &[stack::Value],
    ) -> error::Result<sequence::Sequence> {
        if let Some(function_rule) = &self.function_rule {
            match function_rule {
                FunctionRule::ItemFirst | FunctionRule::PositionFirst | FunctionRule::SizeFirst => {
                    let mut new_arguments: Vec<sequence::Sequence> =
                        vec![closure_values[0].clone().try_into()?];
                    new_arguments.extend(arguments);
                    (self.func)(context, interpreter, &new_arguments)
                }
                FunctionRule::ItemLast => {
                    let mut new_arguments = arguments;
                    new_arguments.push(closure_values[0].clone().try_into()?);
                    (self.func)(context, interpreter, &new_arguments)
                }
                FunctionRule::ItemLastOptional | FunctionRule::AnonymousClosure => {
                    let mut new_arguments = arguments;
                    let value: sequence::Sequence =
                        if !closure_values.is_empty() && !closure_values[0].is_absent() {
                            closure_values[0].clone().try_into()?
                        } else {
                            sequence::Sequence::default()
                        };
                    new_arguments.push(value);
                    (self.func)(context, interpreter, &new_arguments)
                }
                FunctionRule::Collation => {
                    let mut new_arguments = arguments;
                    // the default collation query
                    new_arguments.push(context.static_context().default_collation_uri().into());
                    (self.func)(context, interpreter, &new_arguments)
                }
            }
        } else {
            (self.func)(context, interpreter, &arguments)
        }
    }

    pub(crate) fn name(&self) -> Option<&Name> {
        match self.function_rule {
            Some(FunctionRule::AnonymousClosure) => None,
            _ => Some(&self.name),
        }
    }

    pub(crate) fn arity(&self) -> usize {
        self.arity
    }

    pub(crate) fn signature(&self) -> &function::Signature {
        &self.signature
    }

    pub fn display_representation(&self) -> String {
        let name = self.name.full_name();
        let signature = self.signature.display_representation();
        format!("{}{}", name, signature)
    }
}

fn into_sequences(values: &[stack::Value]) -> error::Result<Vec<sequence::Sequence>> {
    values
        .iter()
        .map(|v| match v {
            stack::Value::Sequence(sequence) => Ok(sequence.clone()),
            stack::Value::Absent => Err(error::Error::XPDY0002),
        })
        .collect()
}

#[derive(Debug)]
pub struct StaticFunctions {
    by_name: HashMap<(Name, u8), function::StaticFunctionId>,
    by_internal_name: HashMap<(Name, u8), function::StaticFunctionId>,
    by_index: Vec<StaticFunction>,
}

impl StaticFunctions {
    pub(crate) fn new() -> Self {
        let mut by_name = HashMap::new();
        let mut by_internal_name = HashMap::new();
        let descriptions = static_function_descriptions();
        let mut by_index = Vec::new();
        for description in descriptions {
            by_index.extend(description.functions());
        }

        for (i, static_function) in by_index.iter().enumerate() {
            let map = match static_function.function_rule {
                Some(FunctionRule::AnonymousClosure) => &mut by_internal_name,
                _ => &mut by_name,
            };
            map.insert(
                (static_function.name.clone(), static_function.arity as u8),
                function::StaticFunctionId(i),
            );
        }
        Self {
            by_name,
            by_internal_name,
            by_index,
        }
    }

    pub fn get_by_name(&self, name: &Name, arity: u8) -> Option<function::StaticFunctionId> {
        // TODO annoying clone
        self.by_name.get(&(name.clone(), arity)).copied()
    }

    pub fn get_by_internal_name(
        &self,
        name: &Name,
        arity: u8,
    ) -> Option<function::StaticFunctionId> {
        // TODO annoying clone
        self.by_internal_name.get(&(name.clone(), arity)).copied()
    }

    pub fn get_by_index(&self, static_function_id: function::StaticFunctionId) -> &StaticFunction {
        &self.by_index[static_function_id.0]
    }
}
