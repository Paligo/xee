// https://www.w3.org/TR/xpath-functions-31/#array-functions

use ibig::IBig;

use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::error;
use crate::function;
use crate::function::StaticFunctionDescription;
use crate::interpreter;
use crate::interpreter::Interpreter;
use crate::sequence;
use crate::wrap_xpath_fn;

#[xpath_fn("map:merge($maps as map(*)*) as map(*)")]
fn merge1(
    maps: impl Iterator<Item = error::Result<function::Map>>,
) -> error::Result<function::Map> {
    merge(
        maps,
        MergeOptions {
            duplicates: MergeDuplicates::UseFirst,
        },
    )
}

#[xpath_fn("map:merge($maps as map(*)*, $options as map(*)) as map(*)")]
fn merge2(
    interpreter: &interpreter::Interpreter,
    maps: impl Iterator<Item = error::Result<function::Map>>,
    options: function::Map,
) -> error::Result<function::Map> {
    let options = MergeOptions::from_map(&options, interpreter)?;
    merge(maps, options)
}

enum MergeDuplicates {
    // raise FOJS0003, if duplicate keys are encountered
    Reject,
    // if duplicate keys are present, the one from the earlier map takes precedence
    UseFirst,
    // if duplicate keys are present, the one from the later map takes precedence
    UseLast,
    // Implementation dependent on which duplicates take precedence
    UseAny,
    // duplicate values are concatenated into a sequence
    Combine,
}

impl MergeDuplicates {
    fn from_str(s: &str) -> error::Result<Self> {
        match s {
            "reject" => Ok(MergeDuplicates::Reject),
            "use-first" => Ok(MergeDuplicates::UseFirst),
            "use-last" => Ok(MergeDuplicates::UseLast),
            "use-any" => Ok(MergeDuplicates::UseAny),
            "combine" => Ok(MergeDuplicates::Combine),
            _ => Err(error::Error::FOJS0005),
        }
    }
}

struct MergeOptions {
    duplicates: MergeDuplicates,
}

impl MergeOptions {
    fn from_map(
        map: &function::Map,
        interpreter: &interpreter::Interpreter,
    ) -> error::Result<Self> {
        let key: atomic::Atomic = "duplicates".to_string().into();
        let duplicates = map.get(&key);
        if let Some(duplicates) = duplicates {
            let value = Self::duplicates_value(interpreter, duplicates)?;
            let duplicates = MergeDuplicates::from_str(&value)?;
            Ok(Self { duplicates })
        } else {
            // default
            Ok(Self {
                duplicates: MergeDuplicates::UseFirst,
            })
        }
    }

    fn duplicates_value(
        interpreter: &interpreter::Interpreter,
        duplicates: &sequence::Sequence,
    ) -> error::Result<String> {
        // we want a string type
        let sequence_type = ast::SequenceType::Item(ast::Item {
            occurrence: ast::Occurrence::One,
            item_type: ast::ItemType::AtomicOrUnionType(Xs::String),
        });
        // apply function conversion rules as specified by the option parameter
        // conventions
        let runnable = interpreter.runnable();
        let type_table = interpreter.state.type_table.borrow();
        let duplicates = duplicates
            .clone()
            .sequence_type_matching_function_conversion(
                &sequence_type,
                runnable.static_context(),
                interpreter.xot(),
                &type_table,
                &|function| runnable.program().function_info(function).signature(),
            )?;
        // take the first value, which should be a string
        let duplicates = duplicates.one()?;
        let atomic: atomic::Atomic = duplicates.to_atomic()?;
        atomic.to_string()
    }
}

fn merge(
    maps: impl Iterator<Item = error::Result<function::Map>>,
    options: MergeOptions,
) -> error::Result<function::Map> {
    match options.duplicates {
        MergeDuplicates::Reject => function::Map::combine(maps, |_, _| Err(error::Error::FOJS0003)),
        MergeDuplicates::UseFirst => function::Map::combine(maps, |a, _| Ok(a)),
        MergeDuplicates::UseLast => function::Map::combine(maps, |_, b| Ok(b)),
        MergeDuplicates::UseAny => function::Map::combine(maps, |a, _| Ok(a)),
        MergeDuplicates::Combine => function::Map::combine(maps, |a, b| a.concat(b)),
    }
}

#[xpath_fn("map:size($map as map(*)) as xs:integer")]
fn size(map: function::Map) -> IBig {
    map.len().into()
}

#[xpath_fn("map:keys($map as map(*)) as xs:anyAtomicType*")]
fn keys(map: function::Map) -> sequence::Sequence {
    map.keys().cloned().collect::<Vec<_>>().into()
}

#[xpath_fn("map:contains($map as map(*), $key as xs:anyAtomicType) as xs:boolean")]
fn contains(map: function::Map, key: atomic::Atomic) -> bool {
    map.get(&key).is_some()
}

#[xpath_fn("map:get($map as map(*), $key as xs:anyAtomicType) as item()*")]
fn get(map: function::Map, key: atomic::Atomic) -> sequence::Sequence {
    map.get(&key).cloned().unwrap_or_default()
}

#[xpath_fn("map:find($input as item()*, $key as xs:anyAtomicType) as array(*)")]
fn find(input: &sequence::Sequence, key: atomic::Atomic) -> error::Result<function::Array> {
    Ok(find_helper(input, atomic::MapKey::new(key.clone()).unwrap())?.into())
}

fn find_helper(
    input: &sequence::Sequence,
    key: atomic::MapKey,
) -> error::Result<Vec<sequence::Sequence>> {
    let mut result: Vec<sequence::Sequence> = Vec::new();
    for item in input.iter() {
        if let sequence::Item::Function(function) = item {
            match function {
                function::Function::Array(array) => {
                    for entry in array.iter() {
                        let found = find_helper(entry, key.clone())?;
                        result.extend(found.into_iter())
                    }
                }
                function::Function::Map(map) => {
                    for (k, v) in map.map_key_entries() {
                        if k == &key {
                            result.push(v.clone());
                        }
                        let found = find_helper(v, key.clone())?;
                        result.extend(found.into_iter())
                    }
                }
                _ => {}
            }
        }
    }
    Ok(result)
}

#[xpath_fn("map:put($map as map(*), $key as xs:anyAtomicType, $value as item()*) as map(*)")]
fn put(
    map: function::Map,
    key: atomic::Atomic,
    value: &sequence::Sequence,
) -> error::Result<function::Map> {
    map.put(key, value)
}

#[xpath_fn("map:entry($key as xs:anyAtomicType, $value as item()*) as map(*)")]
fn entry(key: atomic::Atomic, value: &sequence::Sequence) -> function::Map {
    function::Map::new(vec![(key, value.clone())]).unwrap()
}

#[xpath_fn("map:remove($map as map(*), $keys as xs:anyAtomicType*) as map(*)")]
fn remove(
    map: function::Map,
    keys: impl Iterator<Item = error::Result<atomic::Atomic>>,
) -> error::Result<function::Map> {
    map.remove_keys(keys)
}

#[xpath_fn("map:for-each($map as map(*), $action as function(xs:anyAtomicType, item()*) as item()*) as item()*")]
fn for_each(
    interpreter: &mut Interpreter,
    map: function::Map,
    action: sequence::Item,
) -> error::Result<sequence::Sequence> {
    let function = action.to_function()?;
    let mut result: Vec<sequence::Item> = Vec::with_capacity(map.len());
    for (key, value) in map.entries() {
        let r = interpreter
            .call_function_with_arguments(&function, &[key.clone().into(), value.clone()])?;
        for item in r.iter() {
            result.push(item.clone());
        }
    }
    Ok(result.into())
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(merge1),
        wrap_xpath_fn!(merge2),
        wrap_xpath_fn!(find),
        wrap_xpath_fn!(size),
        wrap_xpath_fn!(keys),
        wrap_xpath_fn!(contains),
        wrap_xpath_fn!(get),
        wrap_xpath_fn!(put),
        wrap_xpath_fn!(entry),
        wrap_xpath_fn!(remove),
        wrap_xpath_fn!(for_each),
    ]
}
