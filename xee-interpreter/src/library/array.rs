// https://www.w3.org/TR/xpath-functions-31/#array-functions

use ibig::IBig;

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

#[xpath_fn("array:get($array as array(*), $position as xs:integer) as item()*")]
fn get(array: function::Array, position: IBig) -> error::Result<sequence::Sequence> {
    let position = convert_position(position)?;
    let item = array
        .index(position as usize)
        .ok_or(error::Error::FOAY0001)?;
    Ok(item.clone())
}

#[xpath_fn("array:size($array as array(*)) as xs:integer")]
fn size(array: function::Array) -> IBig {
    array.len().into()
}

#[xpath_fn(
    "array:put($array as array(*), $position as xs:integer, $member as item()*) as array(*)"
)]
fn put(
    array: function::Array,
    position: IBig,
    member: &sequence::Sequence,
) -> error::Result<function::Array> {
    let position = convert_position(position)?;
    array.put(position, member).ok_or(error::Error::FOAY0001)
}

#[xpath_fn("array:append($array as array(*), $appendage as item()*) as array(*)")]
fn append(array: function::Array, appendage: &sequence::Sequence) -> function::Array {
    array.append(appendage)
}

#[xpath_fn("array:subarray($array as array(*), $start as xs:integer) as array(*)")]
fn subarray2(array: function::Array, start: IBig) -> error::Result<function::Array> {
    let start = convert_position(start)?;
    let length = array.len() - start;
    array.subarray(start, length).ok_or(error::Error::FOAY0001)
}

#[xpath_fn(
    "array:subarray($array as array(*), $start as xs:integer, $length as xs:integer) as array(*)"
)]
fn subarray3(array: function::Array, start: IBig, length: IBig) -> error::Result<function::Array> {
    let start = convert_position(start)?;
    let length = convert_length(length)?;
    array.subarray(start, length).ok_or(error::Error::FOAY0001)
}

#[xpath_fn("array:remove($array as array(*), $positions as xs:integer*) as array(*)")]
fn remove(array: function::Array, positions: &[IBig]) -> error::Result<function::Array> {
    let positions = positions
        .iter()
        .map(|position| convert_position(position.clone()))
        .collect::<error::Result<Vec<usize>>>()?;
    array
        .remove_positions(&positions)
        .ok_or(error::Error::FOAY0001)
}

#[xpath_fn("array:insert-before($array as array(*), $position as xs:integer, $member as item()*) as array(*)")]
fn insert_before(
    array: function::Array,
    position: IBig,
    member: &sequence::Sequence,
) -> error::Result<function::Array> {
    let position = convert_position(position)?;
    array
        .insert_before(position, member)
        .ok_or(error::Error::FOAY0001)
}

#[xpath_fn("array:head($array as array(*)) as item()*")]
fn head(array: function::Array) -> error::Result<sequence::Sequence> {
    let item = array.index(0).ok_or(error::Error::FOAY0001)?;
    Ok(item.clone())
}

#[xpath_fn("array:tail($array as array(*)) as item()*")]
fn tail(array: function::Array) -> error::Result<function::Array> {
    array
        .subarray(1, array.len() - 1)
        .ok_or(error::Error::FOAY0001)
}

#[xpath_fn("array:reverse($array as array(*)) as array(*)")]
fn reverse(array: function::Array) -> function::Array {
    array.reversed()
}

#[xpath_fn("array:join($arrays as array(*)*) as array(*)")]
fn join(arrays: &[function::Array]) -> function::Array {
    function::Array::join(arrays)
}

#[xpath_fn(
    "array:for-each($array as array(*), $action as function(item()*) as item()*) as array(*)"
)]
fn for_each(
    interpreter: &mut Interpreter,
    array: function::Array,
    action: sequence::Item,
) -> error::Result<function::Array> {
    let function = action.to_function()?;
    let mut result = function::Array::new(vec![]);
    for sequence in array.iter() {
        let sequence =
            interpreter.call_function_with_arguments(function.clone(), &[sequence.clone()])?;
        result.push(sequence);
    }
    Ok(result)
}

#[xpath_fn(
    "array:filter($array as array(*), $function as function(item()*) as xs:boolean) as array(*)"
)]
fn filter(
    interpreter: &mut Interpreter,
    array: function::Array,
    function: sequence::Item,
) -> error::Result<function::Array> {
    let function = function.to_function()?;
    let mut result = function::Array::new(vec![]);
    for sequence in array.iter() {
        let include =
            interpreter.call_function_with_arguments(function.clone(), &[sequence.clone()])?;
        let include: atomic::Atomic = include.items().one()?.to_atomic()?;
        let include: bool = include.try_into()?;
        if include {
            result.push(sequence.clone());
        }
    }
    Ok(result)
}

#[xpath_fn("array:fold-left($array as array(*), $zero as item()*, $function as function(item()*, item()*) as item()*) as item()*")]
fn fold_left(
    interpreter: &mut Interpreter,
    array: function::Array,
    zero: &sequence::Sequence,
    function: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = function.to_function()?;

    let mut accumulator = zero.clone();
    for sequence in array.iter() {
        accumulator = interpreter
            .call_function_with_arguments(function.clone(), &[accumulator, sequence.clone()])?;
    }
    Ok(accumulator)
}

#[xpath_fn("array:fold-right($array as array(*), $zero as item()*, $function as function(item()*, item()*) as item()*) as item()*")]
fn fold_right(
    interpreter: &mut Interpreter,
    array: function::Array,
    zero: &sequence::Sequence,
    function: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = function.to_function()?;

    let mut accumulator = zero.clone();
    for sequence in array.iter().rev() {
        accumulator = interpreter
            .call_function_with_arguments(function.clone(), &[sequence.clone(), accumulator])?;
    }
    Ok(accumulator)
}

#[xpath_fn("array:for-each-pair($array1 as array(*), $array2 as array(*), $function as function(item()*, item()*) as item()*) as array(*)")]
fn for_each_pair(
    interpreter: &mut Interpreter,
    array1: function::Array,
    array2: function::Array,
    function: sequence::Item,
) -> error::Result<function::Array> {
    let function = function.to_function()?;

    let mut result = function::Array::new(vec![]);
    for (sequence1, sequence2) in array1.iter().zip(array2.iter()) {
        let sequence = interpreter.call_function_with_arguments(
            function.clone(),
            &[sequence1.clone(), sequence2.clone()],
        )?;
        result.push(sequence);
    }
    Ok(result)
}

#[xpath_fn("array:sort($array as array(*)) as array(*)")]
fn sort1(
    context: &context::DynamicContext,
    input: function::Array,
) -> error::Result<function::Array> {
    sort_without_key(
        context,
        input,
        context.static_context.default_collation_uri(),
    )
}

#[xpath_fn("array:sort($array as array(*), $collation as xs:string?) as array(*)")]
fn sort2(
    context: &context::DynamicContext,
    input: function::Array,
    collation: Option<&str>,
) -> error::Result<function::Array> {
    let collation = collation.unwrap_or(context.static_context.default_collation_uri());
    sort_without_key(context, input, collation)
}

#[xpath_fn("array:sort($array as array(*), $collation as xs:string?, $key as function(item()*) as xs:anyAtomicType*) as array(*)")]
fn sort3(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    input: function::Array,
    collation: Option<&str>,
    key: sequence::Item,
) -> error::Result<function::Array> {
    let collation = collation.unwrap_or(context.static_context.default_collation_uri());
    let function = key.to_function()?;
    sort_by_sequence(context, input, collation, |sequence| {
        let new_sequence =
            interpreter.call_function_with_arguments(function.clone(), &[sequence.clone()])?;
        Ok(new_sequence)
    })
}

fn sort_without_key(
    context: &context::DynamicContext,
    input: function::Array,
    collation: &str,
) -> error::Result<function::Array> {
    sort_by_sequence(context, input, collation, |sequence| {
        // the sequivalent of fn:data()
        let atoms = sequence
            .atomized(context.xot())
            .collect::<error::Result<Vec<_>>>()?;
        Ok(atoms.into())
    })
}

fn sort_by_sequence<F>(
    context: &context::DynamicContext,
    input: function::Array,
    collation: &str,
    mut get: F,
) -> error::Result<function::Array>
where
    F: FnMut(&sequence::Sequence) -> error::Result<sequence::Sequence>,
{
    // see also sort_by_sequence in hof.rs. The signatures are sufficiently
    // different we don't want to try to unify them.

    let collation = context.static_context.collation(collation)?;
    let sequences = input.iter().collect::<Vec<_>>();
    let keys = sequences
        .iter()
        .map(|sequence| get(sequence))
        .collect::<error::Result<Vec<_>>>()?;

    let mut keys_and_sequences = keys.into_iter().zip(sequences).collect::<Vec<_>>();
    // sort by key. unfortunately sort_by requires the compare function
    // to be infallible. It's not in reality, so we make any failures
    // sort less, so they appear early on in the sequence.
    keys_and_sequences.sort_by(|(a_key, _), (b_key, _)| {
        a_key.compare(b_key, &collation, context.implicit_timezone())
    });
    // a pass to detect any errors; if sorting between two items is
    // impossible we want to raise a type error
    for ((a_key, _), (b_key, _)) in keys_and_sequences
        .iter()
        .zip(keys_and_sequences.iter().skip(1))
    {
        a_key.fallible_compare(b_key, &collation, context.implicit_timezone())?;
    }
    // now pick up items again
    let sequences = keys_and_sequences
        .into_iter()
        .map(|(_, sequence)| sequence.clone())
        .collect::<Vec<_>>();
    Ok(function::Array::new(sequences))
}

#[xpath_fn("array:flatten($input as item()*) as item()*")]
fn flatten(input: &sequence::Sequence) -> error::Result<sequence::Sequence> {
    flatten_helper(input)
}

fn flatten_helper(input: &sequence::Sequence) -> error::Result<sequence::Sequence> {
    let mut result = vec![];
    for item in input.items() {
        let item = item?;
        if let Ok(array) = item.to_array() {
            for sequence in array.iter() {
                for item in flatten_helper(sequence)?.items() {
                    result.push(item?.clone());
                }
            }
        } else {
            result.push(item.clone());
        }
    }
    Ok(result.into())
}

fn convert_position(position: IBig) -> error::Result<usize> {
    let position: i64 = position.try_into()?;
    let position = position - 1;
    if position < 0 {
        return Err(error::Error::FOAY0001);
    }
    Ok(position as usize)
}

fn convert_length(length: IBig) -> error::Result<usize> {
    let length: i64 = length.try_into()?;
    if length < 0 {
        return Err(error::Error::FOAY0002);
    }
    let length = length as usize;
    Ok(length)
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(get),
        wrap_xpath_fn!(size),
        wrap_xpath_fn!(put),
        wrap_xpath_fn!(append),
        wrap_xpath_fn!(subarray2),
        wrap_xpath_fn!(subarray3),
        wrap_xpath_fn!(remove),
        wrap_xpath_fn!(insert_before),
        wrap_xpath_fn!(head),
        wrap_xpath_fn!(tail),
        wrap_xpath_fn!(reverse),
        wrap_xpath_fn!(join),
        wrap_xpath_fn!(for_each),
        wrap_xpath_fn!(filter),
        wrap_xpath_fn!(fold_left),
        wrap_xpath_fn!(fold_right),
        wrap_xpath_fn!(for_each_pair),
        wrap_xpath_fn!(sort1),
        wrap_xpath_fn!(sort2),
        wrap_xpath_fn!(sort3),
        wrap_xpath_fn!(flatten),
    ]
}
