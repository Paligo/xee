use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xee_xpath_ast::WithSpan;

use crate::error;
use crate::function;
use crate::interpreter;
use crate::sequence;
use crate::stack;
use crate::xml;

fn type_for_value<'a, F>(value: &stack::Value, get_signature: F) -> error::Result<ast::SequenceType>
where
    F: Fn(&function::Function) -> &'a function::Signature,
{
    match value {
        stack::Value::Empty => Ok(ast::SequenceType::Empty),
        stack::Value::One(item) => Ok(ast::SequenceType::Item(ast::Item {
            item_type: item_type_for_item(item, get_signature),
            occurrence: ast::Occurrence::One,
        })),
        stack::Value::Many(items) => Ok(ast::SequenceType::Item(ast::Item {
            item_type: item_type_for_items(items.as_ref(), get_signature),
            occurrence: ast::Occurrence::Many,
        })),
        stack::Value::Absent => Err(error::Error::ComponentAbsentInDynamicContext),
    }
}

fn item_type_for_item<'a, F>(item: &sequence::Item, get_signature: F) -> ast::ItemType
where
    F: Fn(&function::Function) -> &'a function::Signature,
{
    match item {
        sequence::Item::Atomic(atomic) => {
            // TODO: it's annoying that this is not an Xs type already, as we're
            // only going to convert it back to an Xs type later. Explore whether
            // we could use Xs types directly in the AST, or have an IR version
            // of SequenceType.
            ast::ItemType::AtomicOrUnionType(atomic.type_name().with_span((0..0).into()))
        }
        sequence::Item::Function(function) => match function.as_ref() {
            f @ function::Function::Inline {
                inline_function_id, ..
            } => {
                ast::ItemType::FunctionTest(Box::new(function_test_for_function(f, get_signature)))
            }
            f @ function::Function::Static {
                static_function_id, ..
            } => {
                ast::ItemType::FunctionTest(Box::new(function_test_for_function(f, get_signature)))
            }
            function::Function::Map(..) => {
                todo!();
            }
            function::Function::Array(..) => {
                todo!();
            }
        },
        sequence::Item::Node(node) => ast::ItemType::KindTest(kind_test_for_node(node)),
    }
}

fn function_test_for_function<'a, F>(
    function: &function::Function,
    get_signature: F,
) -> ast::FunctionTest
where
    F: Fn(&function::Function) -> &'a function::Signature,
{
    let signature = get_signature(function);
    function_test_for_signature(signature)
}

fn function_test_for_signature(signature: &function::Signature) -> ast::FunctionTest {
    ast::FunctionTest::TypedFunctionTest(typed_function_test_for_signature(signature))
}

fn typed_function_test_for_signature(signature: &function::Signature) -> ast::TypedFunctionTest {
    // TODO: putting in the defaults or even the whole TypedFunctionTest could
    // be done when the signature is created.
    let parameter_types = signature
        .parameter_types
        .iter()
        .map(|parameter_type| parameter_type.clone().unwrap_or(default_sequence_type()))
        .collect();
    let return_type = signature
        .return_type
        .clone()
        .unwrap_or(default_sequence_type());
    ast::TypedFunctionTest {
        parameter_types,
        return_type,
    }
}

fn default_sequence_type() -> ast::SequenceType {
    ast::SequenceType::Item(ast::Item {
        item_type: ast::ItemType::Item,
        occurrence: ast::Occurrence::Many,
    })
}

fn kind_test_for_node(node: &xml::Node) -> ast::KindTest {
    todo!()
}

fn item_type_for_items<'a, F>(items: &[sequence::Item], get_signature: F) -> ast::ItemType
where
    F: Fn(&function::Function) -> &'a function::Signature,
{
    let mut combined_item_type = item_type_for_item(&items[0], &get_signature);
    for item in items[1..].iter() {
        let item_type = item_type_for_item(item, &get_signature);
        combined_item_type = combine_item_types(&combined_item_type, &item_type);
        // if we're the most general item type, we're done
        if matches!(combined_item_type, ast::ItemType::Item) {
            break;
        }
    }
    combined_item_type
}

fn combine_sequence_types(a: &ast::SequenceType, b: &ast::SequenceType) -> ast::SequenceType {
    match (a, b) {
        (ast::SequenceType::Empty, ast::SequenceType::Empty) => ast::SequenceType::Empty,
        (ast::SequenceType::Item(a), ast::SequenceType::Item(b)) => {
            ast::SequenceType::Item(combine_items(a, b))
        }

        (ast::SequenceType::Item(a), ast::SequenceType::Empty) => {
            ast::SequenceType::Item(combine_item_empty(a))
        }
        (ast::SequenceType::Empty, ast::SequenceType::Item(b)) => {
            ast::SequenceType::Item(combine_item_empty(b))
        }
    }
}

fn contra_combine_sequence_types(
    a: &ast::SequenceType,
    b: &ast::SequenceType,
) -> Option<ast::SequenceType> {
    match (a, b) {
        (ast::SequenceType::Empty, ast::SequenceType::Empty) => Some(ast::SequenceType::Empty),
        (ast::SequenceType::Item(a), ast::SequenceType::Item(b)) => {
            Some(ast::SequenceType::Item(contra_combine_items(a, b)?))
        }
        (ast::SequenceType::Item(a), ast::SequenceType::Empty) => contra_combine_item_empty(a),
        (ast::SequenceType::Empty, ast::SequenceType::Item(b)) => contra_combine_item_empty(b),
    }
}

fn combine_item_empty(a: &ast::Item) -> ast::Item {
    let occurrence = match a.occurrence {
        ast::Occurrence::One => ast::Occurrence::Option,
        ast::Occurrence::Option => ast::Occurrence::Option,
        ast::Occurrence::NonEmpty => ast::Occurrence::Many,
        ast::Occurrence::Many => ast::Occurrence::Many,
    };
    ast::Item {
        item_type: a.item_type.clone(),
        occurrence,
    }
}

fn contra_combine_item_empty(a: &ast::Item) -> Option<ast::SequenceType> {
    match a.occurrence {
        ast::Occurrence::One => None,
        ast::Occurrence::Option => Some(ast::SequenceType::Empty),
        ast::Occurrence::NonEmpty => None,
        ast::Occurrence::Many => Some(ast::SequenceType::Empty),
    }
}

fn combine_items(a: &ast::Item, b: &ast::Item) -> ast::Item {
    ast::Item {
        item_type: combine_item_types(&a.item_type, &b.item_type),
        occurrence: combine_occurrences(&a.occurrence, &b.occurrence),
    }
}

fn contra_combine_items(a: &ast::Item, b: &ast::Item) -> Option<ast::Item> {
    Some(ast::Item {
        item_type: contra_combine_item_types(&a.item_type, &b.item_type)?,
        occurrence: contra_combine_occurrences(&a.occurrence, &b.occurrence),
    })
}

fn combine_occurrences(a: &ast::Occurrence, b: &ast::Occurrence) -> ast::Occurrence {
    match (a, b) {
        (ast::Occurrence::Many, ast::Occurrence::Many) => ast::Occurrence::Many,
        (ast::Occurrence::NonEmpty, ast::Occurrence::NonEmpty) => ast::Occurrence::NonEmpty,
        (ast::Occurrence::Option, ast::Occurrence::Option) => ast::Occurrence::Option,
        (ast::Occurrence::One, ast::Occurrence::One) => ast::Occurrence::One,
        (ast::Occurrence::Many, ast::Occurrence::NonEmpty) => ast::Occurrence::Many,
        (ast::Occurrence::Many, ast::Occurrence::Option) => ast::Occurrence::Many,
        (ast::Occurrence::Many, ast::Occurrence::One) => ast::Occurrence::Many,
        (ast::Occurrence::NonEmpty, ast::Occurrence::Many) => ast::Occurrence::Many,
        (ast::Occurrence::NonEmpty, ast::Occurrence::Option) => ast::Occurrence::Many,
        (ast::Occurrence::NonEmpty, ast::Occurrence::One) => ast::Occurrence::Many,
        (ast::Occurrence::Option, ast::Occurrence::Many) => ast::Occurrence::Many,
        (ast::Occurrence::Option, ast::Occurrence::NonEmpty) => ast::Occurrence::Many,
        (ast::Occurrence::Option, ast::Occurrence::One) => ast::Occurrence::Option,
        (ast::Occurrence::One, ast::Occurrence::Many) => ast::Occurrence::Many,
        (ast::Occurrence::One, ast::Occurrence::NonEmpty) => ast::Occurrence::NonEmpty,
        (ast::Occurrence::One, ast::Occurrence::Option) => ast::Occurrence::Option,
    }
}

fn contra_combine_occurrences(a: &ast::Occurrence, b: &ast::Occurrence) -> ast::Occurrence {
    match (a, b) {
        (ast::Occurrence::Many, ast::Occurrence::Many) => ast::Occurrence::Many,
        (ast::Occurrence::NonEmpty, ast::Occurrence::NonEmpty) => ast::Occurrence::NonEmpty,
        (ast::Occurrence::Option, ast::Occurrence::Option) => ast::Occurrence::Option,
        (ast::Occurrence::One, ast::Occurrence::One) => ast::Occurrence::One,
        (ast::Occurrence::Many, ast::Occurrence::NonEmpty) => ast::Occurrence::NonEmpty,
        (ast::Occurrence::Many, ast::Occurrence::Option) => ast::Occurrence::Option,
        (ast::Occurrence::Many, ast::Occurrence::One) => ast::Occurrence::One,
        (ast::Occurrence::NonEmpty, ast::Occurrence::Many) => ast::Occurrence::NonEmpty,
        (ast::Occurrence::NonEmpty, ast::Occurrence::Option) => ast::Occurrence::One,
        (ast::Occurrence::NonEmpty, ast::Occurrence::One) => ast::Occurrence::One,
        (ast::Occurrence::Option, ast::Occurrence::Many) => ast::Occurrence::Option,
        (ast::Occurrence::Option, ast::Occurrence::NonEmpty) => ast::Occurrence::One,
        (ast::Occurrence::Option, ast::Occurrence::One) => ast::Occurrence::One,
        (ast::Occurrence::One, ast::Occurrence::Many) => ast::Occurrence::One,
        (ast::Occurrence::One, ast::Occurrence::NonEmpty) => ast::Occurrence::One,
        (ast::Occurrence::One, ast::Occurrence::Option) => ast::Occurrence::One,
    }
}

fn contra_combine_item_types(a: &ast::ItemType, b: &ast::ItemType) -> Option<ast::ItemType> {
    match (a, b) {
        (ast::ItemType::AtomicOrUnionType(a), ast::ItemType::AtomicOrUnionType(b)) => Some(
            ast::ItemType::AtomicOrUnionType(contra_combine_atomic_types(a, b)?),
        ),
        (ast::ItemType::FunctionTest(a), ast::ItemType::FunctionTest(b)) => Some(
            ast::ItemType::FunctionTest(Box::new(contra_combine_function_tests(a, b)?)),
        ),
        (ast::ItemType::KindTest(a), ast::ItemType::KindTest(b)) => {
            Some(ast::ItemType::KindTest(contra_combine_kind_tests(a, b)?))
        }
        (ast::ItemType::MapTest(a), ast::ItemType::MapTest(b)) => Some(ast::ItemType::MapTest(
            Box::new(contra_combine_map_tests(a.as_ref(), b.as_ref())?),
        )),
        (ast::ItemType::ArrayTest(a), ast::ItemType::ArrayTest(b)) => {
            Some(ast::ItemType::ArrayTest(Box::new(
                contra_combine_array_tests(a.as_ref(), b.as_ref())?,
            )))
        }
        // TODO: can we combine map/array with FunctionTest?
        _ => None,
    }
}

fn combine_item_types(a: &ast::ItemType, b: &ast::ItemType) -> ast::ItemType {
    match (a, b) {
        (ast::ItemType::AtomicOrUnionType(a), ast::ItemType::AtomicOrUnionType(b)) => {
            ast::ItemType::AtomicOrUnionType(combine_atomic_types(a, b))
        }
        (ast::ItemType::FunctionTest(a), ast::ItemType::FunctionTest(b)) => {
            ast::ItemType::FunctionTest(Box::new(combine_function_tests(a.as_ref(), b.as_ref())))
        }
        (ast::ItemType::KindTest(a), ast::ItemType::KindTest(b)) => {
            ast::ItemType::KindTest(combine_kind_tests(a, b))
        }
        (ast::ItemType::MapTest(a), ast::ItemType::MapTest(b)) => {
            ast::ItemType::MapTest(Box::new(combine_map_tests(a.as_ref(), b.as_ref())))
        }
        (ast::ItemType::ArrayTest(a), ast::ItemType::ArrayTest(b)) => {
            ast::ItemType::ArrayTest(Box::new(combine_array_tests(a.as_ref(), b.as_ref())))
        }
        // TODO: can we combine map/array with FunctionTest?
        _ => ast::ItemType::Item,
    }
}

fn combine_atomic_types(a: &ast::NameS, b: &ast::NameS) -> ast::NameS {
    if a.value == b.value {
        return a.clone();
    }
    let a_xs = Xs::by_name(a.value.namespace(), a.value.local_name()).unwrap();
    let b_xs = Xs::by_name(b.value.namespace(), b.value.local_name()).unwrap();
    if a_xs.derives_from(b_xs) {
        return b.clone();
    }
    if b_xs.derives_from(a_xs) {
        return a.clone();
    }
    ast::Name::new(
        "anyAtomicType".to_string(),
        Some(Xs::namespace().to_string()),
        Some("xs".to_string()),
    )
    .with_span((0..0).into())
}

fn contra_combine_atomic_types(a: &ast::NameS, b: &ast::NameS) -> Option<ast::NameS> {
    if a.value == b.value {
        return Some(a.clone());
    }
    let a_xs = Xs::by_name(a.value.namespace(), a.value.local_name()).unwrap();
    let b_xs = Xs::by_name(b.value.namespace(), b.value.local_name()).unwrap();
    if a_xs.derives_from(b_xs) {
        return Some(a.clone());
    }
    if b_xs.derives_from(a_xs) {
        return Some(b.clone());
    }
    None
}

fn combine_function_tests(a: &ast::FunctionTest, b: &ast::FunctionTest) -> ast::FunctionTest {
    match (a, b) {
        (ast::FunctionTest::TypedFunctionTest(a), ast::FunctionTest::TypedFunctionTest(b)) => {
            combine_typed_function_tests(a, b)
        }
        _ => ast::FunctionTest::AnyFunctionTest,
    }
}

fn contra_combine_function_tests(
    a: &ast::FunctionTest,
    b: &ast::FunctionTest,
) -> Option<ast::FunctionTest> {
    match (a, b) {
        (ast::FunctionTest::TypedFunctionTest(a), ast::FunctionTest::TypedFunctionTest(b)) => {
            contra_combine_typed_function_tests(a, b)
        }
        (ast::FunctionTest::TypedFunctionTest(a), ast::FunctionTest::AnyFunctionTest) => {
            Some(ast::FunctionTest::TypedFunctionTest(a.clone()))
        }
        (ast::FunctionTest::AnyFunctionTest, ast::FunctionTest::TypedFunctionTest(b)) => {
            Some(ast::FunctionTest::TypedFunctionTest(b.clone()))
        }
        (ast::FunctionTest::AnyFunctionTest, ast::FunctionTest::AnyFunctionTest) => {
            Some(ast::FunctionTest::AnyFunctionTest)
        }
    }
}

fn combine_typed_function_tests(
    a: &ast::TypedFunctionTest,
    b: &ast::TypedFunctionTest,
) -> ast::FunctionTest {
    if a.parameter_types.len() != b.parameter_types.len() {
        return ast::FunctionTest::AnyFunctionTest;
    }
    // parameters are contravariant, that is, when combining two functions
    // we want to take the most strict of both types.
    let parameter_types = a
        .parameter_types
        .iter()
        .zip(b.parameter_types.iter())
        .map(|(a, b)| contra_combine_sequence_types(a, b))
        .collect::<Option<Vec<_>>>();
    if let Some(parameter_types) = parameter_types {
        // return type are covariant
        let return_type = combine_sequence_types(&a.return_type, &b.return_type);
        ast::FunctionTest::TypedFunctionTest(ast::TypedFunctionTest {
            parameter_types,
            return_type,
        })
    } else {
        // If there is no contravariant combination of parameters, all we know
        // if that we have two functions, the most generic combination.
        ast::FunctionTest::AnyFunctionTest
    }
}

fn contra_combine_typed_function_tests(
    a: &ast::TypedFunctionTest,
    b: &ast::TypedFunctionTest,
) -> Option<ast::FunctionTest> {
    if a.parameter_types.len() != b.parameter_types.len() {
        return None;
    }
    // we're in the inverse situation here, so parameters are covariant,
    // return type contravariant
    let return_type = contra_combine_sequence_types(&a.return_type, &b.return_type)?;
    let parameter_types = a
        .parameter_types
        .iter()
        .zip(b.parameter_types.iter())
        .map(|(a, b)| combine_sequence_types(a, b))
        .collect::<Vec<_>>();
    Some(ast::FunctionTest::TypedFunctionTest(
        ast::TypedFunctionTest {
            parameter_types,
            return_type,
        },
    ))
}

fn combine_kind_tests(a: &ast::KindTest, b: &ast::KindTest) -> ast::KindTest {
    todo!()
    // match (a, b) {
    //     (ast::KindTest::Document(a), ast::KindTest::Document(b)) => {
    //         ast::KindTest::Document(combine_document_tests(a, b))
    //     }
    //     (ast::KindTest::Element(a), ast::KindTest::Element(b)) => {
    //         ast::KindTest::Element(combine_element_or_attribute_tests(a, b))
    //     }
    //     (ast::KindTest::Attribute(a), ast::KindTest::Attribute(b)) => {
    //         ast::KindTest::Attribute(combine_element_or_attribute_tests(a, b))
    //     }
    //     (ast::KindTest::SchemaElement(a), ast::KindTest::SchemaElement(b)) => {
    //         ast::KindTest::SchemaElement(combine_schema_element_tests(a, b))
    //     }
    //     (ast::KindTest::SchemaAttribute(a), ast::KindTest::SchemaAttribute(b)) => {
    //         ast::KindTest::SchemaAttribute(combine_schema_attribute_tests(a, b))
    //     }
    //     (ast::KindTest::ProcessingInstruction(a), ast::KindTest::ProcessingInstruction(b)) => {
    //         ast::KindTest::ProcessingInstruction(combine_processing_instruction_tests(a, b))
    //     }
    //     (ast::KindTest::Comment, ast::KindTest::Comment) => ast::KindTest::Comment,
    //     (ast::KindTest::Text, ast::KindTest::Text) => ast::KindTest::Text,
    //     (ast::KindTest::NamespaceNode, ast::KindTest::NamespaceNode) => {
    //         ast::KindTest::NamespaceNode
    //     }
    //     (ast::KindTest::Any, ast::KindTest::Any) => ast::KindTest::Any,
    //     _ => ast::KindTest::Any,
    // }
}

fn contra_combine_kind_tests(a: &ast::KindTest, b: &ast::KindTest) -> Option<ast::KindTest> {
    todo!()
}

fn combine_map_tests(a: &ast::MapTest, b: &ast::MapTest) -> ast::MapTest {
    todo!()
}

fn contra_combine_map_tests(a: &ast::MapTest, b: &ast::MapTest) -> Option<ast::MapTest> {
    todo!()
}

fn combine_array_tests(a: &ast::ArrayTest, b: &ast::ArrayTest) -> ast::ArrayTest {
    todo!()
}

fn contra_combine_array_tests(a: &ast::ArrayTest, b: &ast::ArrayTest) -> Option<ast::ArrayTest> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    use ibig::ibig;
    use insta::assert_debug_snapshot;
    use rust_decimal_macros::dec;
    use xee_xpath_ast::Namespaces;
    use xot::Xot;

    use crate::{atomic, context};

    // fn with_runnable<F>(f: F)
    // where
    //     F: FnOnce(&interpreter::Runnable),
    // {
    //     let program = function::Program::new("dummy src".to_string());
    //     let namespaces = Namespaces::default();
    //     let static_context = context::StaticContext::new(&namespaces);
    //     let xot = Xot::new();
    //     let dynamic_context = context::DynamicContext::new(&xot, &static_context);
    //     let runnable = interpreter::Runnable::new(&program, &dynamic_context);

    //     f(&runnable);
    // }

    #[test]
    fn test_integer() {
        let a: atomic::Atomic = ibig!(1).into();
        let value: stack::Value = a.into();

        let sequence_type = type_for_value(&value, |_| unreachable!()).unwrap();
        assert_eq!(
            sequence_type,
            ast::SequenceType::Item(ast::Item {
                item_type: ast::ItemType::AtomicOrUnionType(
                    ast::Name::new(
                        "integer".to_string(),
                        Some("http://www.w3.org/2001/XMLSchema".to_string()),
                        Some("xs".to_string()),
                    )
                    .with_span((0..0).into())
                ),
                occurrence: ast::Occurrence::One,
            })
        );
    }

    #[test]
    fn test_two_integers() {
        let a: sequence::Item = ibig!(1).into();
        let b: sequence::Item = ibig!(2).into();
        let value: stack::Value = vec![a, b].into();

        let sequence_type = type_for_value(&value, |_| unreachable!()).unwrap();

        assert_eq!(
            sequence_type,
            ast::SequenceType::Item(ast::Item {
                item_type: ast::ItemType::AtomicOrUnionType(
                    ast::Name::new(
                        "integer".to_string(),
                        Some("http://www.w3.org/2001/XMLSchema".to_string()),
                        Some("xs".to_string()),
                    )
                    .with_span((0..0).into())
                ),
                occurrence: ast::Occurrence::Many,
            })
        );
    }

    #[test]
    fn test_integer_and_string() {
        let a: sequence::Item = ibig!(1).into();
        let b: sequence::Item = "foo".to_string().into();
        let value: stack::Value = vec![a, b].into();

        let sequence_type = type_for_value(&value, |_| unreachable!()).unwrap();
        assert_eq!(
            sequence_type,
            ast::SequenceType::Item(ast::Item {
                item_type: ast::ItemType::AtomicOrUnionType(
                    ast::Name::new(
                        "anyAtomicType".to_string(),
                        Some("http://www.w3.org/2001/XMLSchema".to_string()),
                        Some("xs".to_string()),
                    )
                    .with_span((0..0).into())
                ),
                occurrence: ast::Occurrence::Many,
            })
        );
    }

    #[test]
    fn test_integer_and_decimal() {
        let a: sequence::Item = ibig!(1).into();
        let b: sequence::Item = dec!(2.3).into();
        let value: stack::Value = vec![a, b].into();

        let sequence_type = type_for_value(&value, |_| unreachable!()).unwrap();
        assert_eq!(
            sequence_type,
            ast::SequenceType::Item(ast::Item {
                item_type: ast::ItemType::AtomicOrUnionType(
                    ast::Name::new(
                        "decimal".to_string(),
                        Some("http://www.w3.org/2001/XMLSchema".to_string()),
                        Some("xs".to_string()),
                    )
                    .with_span((0..0).into())
                ),
                occurrence: ast::Occurrence::Many,
            })
        );
    }

    #[test]
    fn test_function() {
        let value: stack::Value = function::Function::Static {
            // the id is irrelevant, as get_signature is hardcoded to return
            // the same signature
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        }
        .into();

        let integer_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(
                ast::Name::new(
                    "integer".to_string(),
                    Some("http://www.w3.org/2001/XMLSchema".to_string()),
                    Some("xs".to_string()),
                )
                .with_span((0..0).into()),
            ),
            occurrence: ast::Occurrence::One,
        });

        let signature = function::Signature {
            parameter_types: vec![Some(integer_type.clone())],
            return_type: Some(integer_type.clone()),
        };

        assert_debug_snapshot!(type_for_value(&value, |_| &signature).unwrap());
    }

    #[test]
    fn test_function_multiple() {
        let a: sequence::Item = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        }
        .into();
        let b: sequence::Item = function::Function::Static {
            static_function_id: function::StaticFunctionId(2),
            closure_vars: vec![],
        }
        .into();
        let value = vec![a, b].into();

        let integer_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(
                ast::Name::new(
                    "integer".to_string(),
                    Some("http://www.w3.org/2001/XMLSchema".to_string()),
                    Some("xs".to_string()),
                )
                .with_span((0..0).into()),
            ),
            occurrence: ast::Occurrence::One,
        });

        let decimal_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(
                ast::Name::new(
                    "decimal".to_string(),
                    Some("http://www.w3.org/2001/XMLSchema".to_string()),
                    Some("xs".to_string()),
                )
                .with_span((0..0).into()),
            ),
            occurrence: ast::Occurrence::One,
        });

        let signature1 = function::Signature {
            parameter_types: vec![Some(integer_type.clone())],
            return_type: Some(integer_type.clone()),
        };
        let signature2 = function::Signature {
            parameter_types: vec![Some(decimal_type.clone())],
            return_type: Some(integer_type.clone()),
        };

        // the expected merged function type is more general, meaning it
        // takes the more specific argument type (due to contravariance), thus
        // it takes an integer
        assert_debug_snapshot!(type_for_value(&value, |function| {
            match function {
                function::Function::Static {
                    static_function_id: function::StaticFunctionId(1),
                    ..
                } => &signature1,
                function::Function::Static {
                    static_function_id: function::StaticFunctionId(2),
                    ..
                } => &signature2,
                _ => unreachable!(),
            }
        })
        .unwrap());
    }

    #[test]
    fn test_function_multiple_return_value() {
        let a: sequence::Item = function::Function::Static {
            static_function_id: function::StaticFunctionId(1),
            closure_vars: vec![],
        }
        .into();
        let b: sequence::Item = function::Function::Static {
            static_function_id: function::StaticFunctionId(2),
            closure_vars: vec![],
        }
        .into();
        let value = vec![a, b].into();

        let integer_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(
                ast::Name::new(
                    "integer".to_string(),
                    Some("http://www.w3.org/2001/XMLSchema".to_string()),
                    Some("xs".to_string()),
                )
                .with_span((0..0).into()),
            ),
            occurrence: ast::Occurrence::One,
        });

        let decimal_type = ast::SequenceType::Item(ast::Item {
            item_type: ast::ItemType::AtomicOrUnionType(
                ast::Name::new(
                    "decimal".to_string(),
                    Some("http://www.w3.org/2001/XMLSchema".to_string()),
                    Some("xs".to_string()),
                )
                .with_span((0..0).into()),
            ),
            occurrence: ast::Occurrence::One,
        });

        let signature1 = function::Signature {
            parameter_types: vec![Some(integer_type.clone())],
            return_type: Some(integer_type.clone()),
        };
        let signature2 = function::Signature {
            parameter_types: vec![Some(integer_type.clone())],
            return_type: Some(decimal_type.clone()),
        };

        // the expected merged function type is more general, meaning it
        // returns a decimal (as that's the more general type, and the return
        // value is covariant)
        assert_debug_snapshot!(type_for_value(&value, |function| {
            match function {
                function::Function::Static {
                    static_function_id: function::StaticFunctionId(1),
                    ..
                } => &signature1,
                function::Function::Static {
                    static_function_id: function::StaticFunctionId(2),
                    ..
                } => &signature2,
                _ => unreachable!(),
            }
        })
        .unwrap());
    }
}
