use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xee_xpath_ast::WithSpan;

use crate::error;
use crate::function;
use crate::interpreter;
use crate::sequence;
use crate::stack;
use crate::xml;

fn type_for_value(
    value: &stack::Value,
    runnable: &interpreter::Runnable,
) -> error::Result<ast::SequenceType> {
    match value {
        stack::Value::Empty => Ok(ast::SequenceType::Empty),
        stack::Value::One(item) => Ok(ast::SequenceType::Item(ast::Item {
            item_type: item_type_for_item(item, runnable),
            occurrence: ast::Occurrence::One,
        })),
        stack::Value::Many(items) => Ok(ast::SequenceType::Item(ast::Item {
            item_type: item_type_for_items(items.as_ref(), runnable),
            occurrence: ast::Occurrence::Many,
        })),
        stack::Value::Absent => Err(error::Error::ComponentAbsentInDynamicContext),
    }
}

fn item_type_for_item(item: &sequence::Item, runnable: &interpreter::Runnable) -> ast::ItemType {
    match item {
        sequence::Item::Atomic(atomic) => {
            // TODO: it's annoying that this is not an Xs type already, as we're
            // only going to convert it back to an Xs type later. Explore whether
            // we could use Xs types directly in the AST, or have an IR version
            // of SequenceType.
            ast::ItemType::AtomicOrUnionType(atomic.type_name().with_span((0..0).into()))
        }
        sequence::Item::Function(function) => match function.as_ref() {
            function::Function::Inline {
                inline_function_id, ..
            } => ast::ItemType::FunctionTest(Box::new(function_test_for_inline_function(
                *inline_function_id,
                runnable,
            ))),
            function::Function::Static {
                static_function_id, ..
            } => ast::ItemType::FunctionTest(Box::new(function_test_for_static_function(
                *static_function_id,
                runnable,
            ))),
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

fn function_test_for_inline_function(
    inline_function_id: function::InlineFunctionId,
    runnable: &interpreter::Runnable,
) -> ast::FunctionTest {
    let signature = runnable.inline_function(inline_function_id).signature();
    function_test_for_signature(signature)
}

fn function_test_for_static_function(
    static_function_id: function::StaticFunctionId,
    runnable: &interpreter::Runnable,
) -> ast::FunctionTest {
    let signature = runnable.static_function(static_function_id).signature();
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

fn item_type_for_items(
    items: &[sequence::Item],
    runnable: &interpreter::Runnable,
) -> ast::ItemType {
    let mut combined_item_type = item_type_for_item(&items[0], runnable);
    for item in items[1..].iter() {
        let item_type = item_type_for_item(item, runnable);
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
