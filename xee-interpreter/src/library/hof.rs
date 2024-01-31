// https://www.w3.org/TR/xpath-functions-31/#higher-order-functions

use ibig::IBig;

use xee_name::Name;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::context;
use crate::error;
use crate::function;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::occurrence::Occurrence;
use crate::sequence;
use crate::wrap_xpath_fn;
use crate::xml;

// we use the special marker context_last_optional here. The last node
// argument, $arg, is not part of the official signature, but it is
// required to create a static closure from the context, bind it to
// a context if supplied. The context is also optional; the resulting
// function won't be bound to any context if the context isn't present.
#[xpath_fn(
    "fn:function-lookup($name as xs:QName, $arity as xs:integer, $arg as node()?) as function(*)?",
    context_last_optional
)]
fn function_lookup(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    name: Name,
    arity: IBig,
    arg: Option<xml::Node>,
) -> Option<sequence::Item> {
    let arity: u8 = if let Ok(arity) = arity.try_into() {
        arity
    } else {
        return None;
    };
    let static_function_id = context.static_context.functions.get_by_name(&name, arity);
    static_function_id.map(|static_function_id| {
        interpreter
            .create_static_closure_from_context(static_function_id, arg)
            .into()
    })
}

#[xpath_fn("fn:function-name($func as function(*)) as xs:QName?")]
fn function_name(interpreter: &Interpreter, func: sequence::Item) -> error::Result<Option<Name>> {
    let function = func.to_function()?;
    Ok(interpreter.function_name(function.as_ref()))
}

#[xpath_fn("fn:function-arity($func as function(*)) as xs:integer")]
fn function_arity(interpreter: &Interpreter, func: sequence::Item) -> error::Result<IBig> {
    let function = func.to_function()?;
    Ok(interpreter.function_arity(function.as_ref()).into())
}

#[xpath_fn("fn:for-each($seq as item()*, $action as function(item()) as item()*) as item()*")]
fn for_each(
    interpreter: &mut Interpreter,
    seq: &sequence::Sequence,
    action: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let mut result: Vec<sequence::Item> = Vec::with_capacity(seq.len());
    let function = action.to_function()?;

    for item in seq.items() {
        let item = item?;
        let value = interpreter.call_function_with_arguments(function.clone(), &[item.into()])?;
        for item in value.items() {
            result.push(item?);
        }
    }
    Ok(result.into())
}

#[xpath_fn("fn:filter($seq as item()*, $predicate as function(item()) as xs:boolean) as item()*")]
fn filter(
    interpreter: &mut Interpreter,
    seq: &sequence::Sequence,
    predicate: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let mut result: Vec<sequence::Item> = Vec::new();
    let function = predicate.to_function()?;

    for item in seq.items() {
        let item = item?;
        let value =
            interpreter.call_function_with_arguments(function.clone(), &[item.clone().into()])?;
        let atom: atomic::Atomic = value.items().one()?.to_atomic()?;
        let value: bool = atom.try_into()?;
        if value {
            result.push(item);
        }
    }
    Ok(result.into())
}

#[xpath_fn("fn:fold-left($seq as item()*, $zero as item()*, $f as function(item()*, item()) as item()*) as item()*")]
fn fold_left(
    interpreter: &mut Interpreter,
    seq: &sequence::Sequence,
    zero: &sequence::Sequence,
    f: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = f.to_function()?;

    let mut accumulator = zero.clone();
    for item in seq.items() {
        let item = item?;
        accumulator = interpreter
            .call_function_with_arguments(function.clone(), &[accumulator, item.into()])?;
    }
    Ok(accumulator)
}

#[xpath_fn("fn:fold-right($seq as item()*, $zero as item()*, $f as function(item(), item()*) as item()*) as item()*")]
fn fold_right(
    interpreter: &mut Interpreter,
    seq: &sequence::Sequence,
    zero: &sequence::Sequence,
    f: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = f.to_function()?;

    let mut accumulator = zero.clone();
    // TODO: do not have reverse iterator, so have to collect first
    let seq = seq.items().collect::<error::Result<Vec<_>>>()?;
    for item in seq.into_iter().rev() {
        accumulator = interpreter
            .call_function_with_arguments(function.clone(), &[item.into(), accumulator])?;
    }
    Ok(accumulator)
}

#[xpath_fn("fn:for-each-pair($seq1 as item()*, $seq2 as item()*, $action as function(item(), item()) as item()*) as item()*")]
fn for_each_pair(
    interpreter: &mut Interpreter,
    seq1: &sequence::Sequence,
    seq2: &sequence::Sequence,
    action: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let mut result: Vec<sequence::Item> = Vec::with_capacity(seq1.len());
    let function = action.to_function()?;

    for (item1, item2) in seq1.items().zip(seq2.items()) {
        let item1 = item1?;
        let item2 = item2?;
        let value = interpreter
            .call_function_with_arguments(function.clone(), &[item1.into(), item2.into()])?;
        for item in value.items() {
            result.push(item?);
        }
    }
    Ok(result.into())
}

#[xpath_fn("fn:sort($input as item()*) as item()*")]
fn sort1(
    context: &context::DynamicContext,
    input: &sequence::Sequence,
) -> error::Result<sequence::Sequence> {
    input.sorted(context, context.static_context.default_collation_uri())
}

#[xpath_fn("fn:sort($input as item()*, $collation as xs:string?) as item()*")]
fn sort2(
    context: &context::DynamicContext,
    input: &sequence::Sequence,
    collation: Option<&str>,
) -> error::Result<sequence::Sequence> {
    let collation = collation.unwrap_or(context.static_context.default_collation_uri());
    input.sorted(context, collation)
}

#[xpath_fn("fn:sort($input as item()*, $collation as xs:string?, $key as function(item()) as xs:anyAtomicType*) as item()*")]
fn sort3(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    input: &sequence::Sequence,
    collation: Option<&str>,
    key: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let collation = collation.unwrap_or(context.static_context.default_collation_uri());
    let function = key.to_function()?;
    input.sorted_by_key(context, collation, |item| {
        let value =
            interpreter.call_function_with_arguments(function.clone(), &[item.clone().into()])?;
        Ok(value)
    })
}

fn sort_without_key(
    context: &context::DynamicContext,
    input: &sequence::Sequence,
    collation: &str,
) -> error::Result<sequence::Sequence> {
    sort_by_sequence(context, input, collation, |item| {
        // the equivalent of fn:data()
        let seq: sequence::Sequence = item.clone().into();
        let atoms = seq
            .atomized(context.xot())
            .collect::<error::Result<Vec<_>>>()?;
        Ok(atoms.into())
    })
}

fn sort_by_sequence<F>(
    context: &context::DynamicContext,
    input: &sequence::Sequence,
    collation: &str,
    get: F,
) -> error::Result<sequence::Sequence>
where
    F: FnMut(&sequence::Item) -> error::Result<sequence::Sequence>,
{
    // see also sort_by_sequence in array.rs. The signatures are
    // sufficiently different we don't want to try to unify them.

    let collation = context.static_context.collation(collation)?;
    let items = input.items().collect::<error::Result<Vec<_>>>()?;
    let keys = items.iter().map(get).collect::<error::Result<Vec<_>>>()?;

    let mut keys_and_items = keys.into_iter().zip(items).collect::<Vec<_>>();
    // sort by key. unfortunately sort_by requires the compare function
    // to be infallible. It's not in reality, so we make any failures
    // sort less, so they appear early on in the sequence.
    keys_and_items.sort_by(|(a_key, _), (b_key, _)| {
        a_key.compare(b_key, &collation, context.implicit_timezone())
    });
    // a pass to detect any errors; if sorting between two items is
    // impossible we want to raise a type error
    for ((a_key, _), (b_key, _)) in keys_and_items.iter().zip(keys_and_items.iter().skip(1)) {
        a_key.fallible_compare(b_key, &collation, context.implicit_timezone())?;
    }
    // now pick up items again
    let items = keys_and_items
        .into_iter()
        .map(|(_, item)| item)
        .collect::<Vec<_>>();
    Ok(items.into())
}

#[xpath_fn("fn:apply($function as function(*), $array as array(*)) as item()*")]
fn apply(
    interpreter: &mut Interpreter,
    function: sequence::Item,
    array: function::Array,
) -> error::Result<sequence::Sequence> {
    let function = function.to_function()?;
    let arity = array.len();
    if interpreter.function_arity(function.as_ref()) != arity {
        return Err(error::Error::FOAP0001);
    }
    let arguments = array.iter().cloned().collect::<Vec<_>>();

    interpreter.call_function_with_arguments(function.clone(), &arguments)
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(function_lookup),
        wrap_xpath_fn!(function_name),
        wrap_xpath_fn!(function_arity),
        wrap_xpath_fn!(for_each),
        wrap_xpath_fn!(filter),
        wrap_xpath_fn!(fold_left),
        wrap_xpath_fn!(fold_right),
        wrap_xpath_fn!(for_each_pair),
        wrap_xpath_fn!(sort1),
        wrap_xpath_fn!(sort2),
        wrap_xpath_fn!(sort3),
        wrap_xpath_fn!(apply),
    ]
}
