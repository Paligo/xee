use xee_schema_type::Xs;
use xee_xpath_ast::ast;

use super::static_function::FunctionKind;

/// A function signature.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    parameter_types: Vec<Option<ast::SequenceType>>,
    return_type: Option<ast::SequenceType>,
}

impl Signature {
    pub fn new(
        parameter_types: Vec<Option<ast::SequenceType>>,
        return_type: Option<ast::SequenceType>,
    ) -> Self {
        Self {
            parameter_types,
            return_type,
        }
    }

    pub(crate) fn map_signature() -> Self {
        let key = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(Xs::AnyAtomicType),
            occurrence: ast::Occurrence::One,
        });

        let return_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::Item,
            occurrence: ast::Occurrence::Many,
        });
        Self {
            parameter_types: vec![Some(key)],
            return_type: Some(return_type),
        }
    }

    pub(crate) fn array_signature() -> Self {
        let position = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(Xs::AnyAtomicType),
            occurrence: ast::Occurrence::One,
        });

        let return_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::Item,
            occurrence: ast::Occurrence::Many,
        });
        Self {
            parameter_types: vec![Some(position)],
            return_type: Some(return_type),
        }
    }

    pub(crate) fn alternative_signatures(
        &self,
        function_kind: FunctionKind,
    ) -> Vec<(Signature, Option<FunctionKind>)> {
        match function_kind {
            FunctionKind::ItemFirst => vec![
                (
                    Self {
                        parameter_types: self.parameter_types[1..].to_vec(),
                        return_type: self.return_type.clone(),
                    },
                    Some(function_kind),
                ),
                (self.clone(), None),
            ],
            FunctionKind::ItemLast => vec![
                (
                    Self {
                        parameter_types: self.parameter_types[..self.parameter_types.len() - 1]
                            .to_vec(),
                        return_type: self.return_type.clone(),
                    },
                    Some(function_kind),
                ),
                (self.clone(), None),
            ],
            FunctionKind::ItemLastOptional => vec![(
                Self {
                    parameter_types: self.parameter_types[..self.parameter_types.len() - 1]
                        .to_vec(),
                    return_type: self.return_type.clone(),
                },
                Some(function_kind),
            )],
            FunctionKind::Position => vec![(self.clone(), Some(function_kind))],
            FunctionKind::Size => vec![(self.clone(), Some(function_kind))],
            FunctionKind::Collation => vec![
                (
                    Self {
                        parameter_types: self.parameter_types[..self.parameter_types.len() - 1]
                            .to_vec(),
                        return_type: self.return_type.clone(),
                    },
                    Some(function_kind),
                ),
                (self.clone(), None),
            ],
        }
    }

    /// The parameter types of the function.
    pub fn parameter_types(&self) -> &[Option<ast::SequenceType>] {
        &self.parameter_types
    }

    /// The return type of the function.
    pub fn return_type(&self) -> Option<&ast::SequenceType> {
        self.return_type.as_ref()
    }

    /// Return the arity of the function signature.
    pub fn arity(&self) -> usize {
        self.parameter_types.len()
    }
}

impl From<ast::Signature> for Signature {
    fn from(signature: ast::Signature) -> Self {
        Self {
            parameter_types: signature
                .params
                .into_iter()
                .map(|p| Some(p.type_))
                .collect(),
            return_type: Some(signature.return_type),
        }
    }
}
