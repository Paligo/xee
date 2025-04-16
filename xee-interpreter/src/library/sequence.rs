// https://www.w3.org/TR/xpath-functions-31/#sequence-functions

use ahash::HashMap;
use ibig::IBig;
use xee_xpath_macros::xpath_fn;

use crate::atomic::op_add;
use crate::atomic::op_div;
use crate::atomic::Atomic;
use crate::atomic::AtomicCompare;
use crate::atomic::OpGt;
use crate::atomic::OpLt;
use crate::atomic::StringType;
use crate::context::DynamicContext;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::sequence;
use crate::string::Collation;
use crate::wrap_xpath_fn;

#[xpath_fn("fn:empty($arg as item()*) as xs:boolean")]
fn empty(arg: &sequence::Sequence) -> bool {
    arg.is_empty()
}

#[xpath_fn("fn:exists($arg as item()*) as xs:boolean")]
fn exists(arg: &sequence::Sequence) -> bool {
    !arg.is_empty()
}

#[xpath_fn("fn:head($arg as item()*) as item()?")]
fn head(arg: &sequence::Sequence) -> Option<sequence::Item> {
    arg.iter().next()
}

#[xpath_fn("fn:tail($arg as item()*) as item()*")]
fn tail(arg: &sequence::Sequence) -> sequence::Sequence {
    if arg.is_empty() {
        return sequence::Sequence::default();
    }
    let mut items = arg.iter();
    // skip first item
    items.next();
    // now collect the rest
    items.collect::<Vec<_>>().into()
}

#[xpath_fn(
    "fn:insert-before($target as item()*, $position as xs:integer, $inserts as item()*) as item()*"
)]
fn insert_before(
    target: &sequence::Sequence,
    position: IBig,
    inserts: &sequence::Sequence,
) -> error::Result<sequence::Sequence> {
    if target.is_empty() {
        return Ok(inserts.clone());
    }
    let position = if position < IBig::from(0) {
        IBig::from(0)
    } else {
        position
    };
    let position: usize = position
        .try_into()
        .map_err(|_| error::Error::new(error::ErrorCode::FOAR0002))?;
    let position = position.saturating_sub(1);
    let position = if position > target.len() {
        target.len()
    } else {
        position
    };

    let mut target_items = target.iter();
    let mut result = Vec::with_capacity(target.len() + inserts.len());
    let mut i = 0;
    if position > 0 {
        for item in target_items.by_ref() {
            result.push(item.clone());
            i += 1;
            if i == position {
                break;
            }
        }
    }
    for item in inserts.iter() {
        result.push(item.clone());
    }
    for item in target_items {
        result.push(item.clone());
    }
    Ok(result.into())
}

#[xpath_fn("fn:remove($target as item()*, $position as xs:integer) as item()*")]
fn remove(target: &sequence::Sequence, position: IBig) -> error::Result<sequence::Sequence> {
    let position = if position < IBig::from(0) {
        IBig::from(0)
    } else {
        position
    };
    let position: usize = position
        .try_into()
        .map_err(|_| error::Error::new(error::ErrorCode::FOAR0002))?;
    if position == 0 || position > target.len() {
        return Ok(target.clone());
    }
    let position = position.saturating_sub(1);
    let mut target = target.clone().iter().collect::<Vec<_>>();
    target.remove(position);
    Ok(target.into())
}

#[xpath_fn("fn:reverse($arg as item()*) as item()*")]
fn reverse(arg: &sequence::Sequence) -> sequence::Sequence {
    if arg.is_empty() {
        return arg.clone();
    }
    let mut items = arg.clone().iter().collect::<Vec<_>>();
    items.reverse();
    items.into()
}

#[xpath_fn("fn:subsequence($sourceSeq as item()*, $startingLoc as xs:double) as item()*")]
fn subsequence2(source_seq: &sequence::Sequence, starting_loc: f64) -> Vec<sequence::Item> {
    if starting_loc.is_nan() {
        return Vec::new();
    }
    let starting_loc = starting_loc - 1.0;
    let starting_loc = starting_loc.clamp(0.0, (source_seq.len()) as f64);
    let starting_loc = starting_loc as usize;
    source_seq.iter().skip(starting_loc).collect()
}

#[xpath_fn(
    "fn:subsequence($sourceSeq as item()*, $startingLoc as xs:double, $length as xs:double) as item()*"
)]
fn subsequence3(
    source_seq: &sequence::Sequence,
    starting_loc: f64,
    length: f64,
) -> Vec<sequence::Item> {
    let starting_loc = starting_loc.round();
    let starting_loc = starting_loc - 1.0;
    let length = length.round();
    let end = starting_loc + length;
    if end.is_nan() {
        return Vec::new();
    }
    let starting_loc = starting_loc.clamp(0.0, (source_seq.len()) as f64);
    let end = end.clamp(starting_loc, (source_seq.len()) as f64);
    let starting_loc = starting_loc as usize;
    let end = end as usize;

    source_seq
        .iter()
        .skip(starting_loc)
        .take(end - starting_loc)
        .collect()
}

#[xpath_fn("fn:unordered($sourceSeq as item()*) as item()*")]
fn unordered(source_seq: &sequence::Sequence) -> sequence::Sequence {
    source_seq.clone()
}

#[xpath_fn(
    "fn:distinct-values($arg as xs:anyAtomicType*, $collation as xs:string) as xs:anyAtomicType*",
    collation
)]
fn distinct_values(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
    collation: &str,
) -> error::Result<Vec<Atomic>> {
    let collation = context
        .static_context()
        .resolve_collation_str(Some(collation))?;
    let default_offset = context.implicit_timezone();
    // we use a HashMap first to remove items to compare. It removes easy
    // duplicates. It can't generate false positives as the default
    // string compare is in use. We store the order in the value.
    let distinct_set = arg
        .enumerate()
        .map(|(i, atom)| Ok((atom?, i)))
        .collect::<error::Result<HashMap<_, _>>>()?;
    if distinct_set.is_empty() {
        return Ok(Vec::new());
    }

    // now we sort the distinct set by the order
    let mut distinct_set = distinct_set.into_iter().collect::<Vec<_>>();
    distinct_set.sort_by_key(|(_, order)| *order);
    let distinct_values = distinct_set
        .into_iter()
        .map(|(atom, _)| atom)
        .collect::<Vec<_>>();

    // now we use an exhaustive, and expensive, deep-equal check to filter out
    // more duplicates
    let mut distinct = Vec::new();
    'outer: for atom in distinct_values {
        for seen in &distinct {
            if atom.deep_equal(seen, &collation, default_offset) {
                continue 'outer;
            }
        }
        distinct.push(atom)
    }
    Ok(distinct)
}

#[xpath_fn("fn:index-of($seq as xs:anyAtomicType*, $search as xs:anyAtomicType, $collation as xs:string) as xs:integer*", collation)]
fn index_of(
    context: &DynamicContext,
    seq: impl Iterator<Item = error::Result<Atomic>>,
    search: Atomic,
    collation: &str,
) -> error::Result<Vec<IBig>> {
    let collation = context
        .static_context()
        .resolve_collation_str(Some(collation))?;
    let default_offset = context.implicit_timezone();
    let mut indices = Vec::new();
    for (i, atom) in seq.enumerate() {
        if atom?.equal(&search, &collation, default_offset) {
            indices.push((i + 1).into());
        }
    }
    Ok(indices)
}

#[xpath_fn("fn:deep-equal($parameter1 as item()*, $parameter2 as item()*, $collation as xs:string) as xs:boolean", collation)]
fn deep_equal(
    context: &DynamicContext,
    interpreter: &Interpreter,
    parameter1: &sequence::Sequence,
    parameter2: &sequence::Sequence,
    collation: &str,
) -> error::Result<bool> {
    let collation = context
        .static_context()
        .resolve_collation_str(Some(collation))?;
    let default_offset = context.implicit_timezone();
    parameter1.deep_equal(parameter2, &collation, default_offset, interpreter.xot())
}

#[xpath_fn("fn:zero-or-one($arg as item()*) as item()?")]
fn zero_or_one(arg: &sequence::Sequence) -> error::Result<Option<sequence::Item>> {
    match arg.len() {
        0 => Ok(None),
        1 => Ok(arg.iter().next()),
        _ => Err(error::Error::new(error::ErrorCode::FORG0003)),
    }
}

#[xpath_fn("fn:one-or-more($arg as item()*) as item()+")]
fn one_or_more(arg: &sequence::Sequence) -> error::Result<sequence::Sequence> {
    if arg.is_empty() {
        Err(error::Error::new(error::ErrorCode::FORG0004))
    } else {
        Ok(arg.clone())
    }
}

#[xpath_fn("fn:exactly-one($arg as item()*) as item()")]
fn exactly_one(arg: &sequence::Sequence) -> error::Result<sequence::Item> {
    if arg.len() == 1 {
        Ok(arg.iter().next().unwrap().clone())
    } else {
        Err(error::Error::new(error::ErrorCode::FORG0005))
    }
}

#[xpath_fn("fn:count($arg as item()*) as xs:integer")]
fn count(arg: &sequence::Sequence) -> IBig {
    arg.len().into()
}

#[xpath_fn("fn:avg($arg as xs:anyAtomicType*) as xs:anyAtomicType?")]
fn avg(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
) -> error::Result<Option<Atomic>> {
    let r = sum_atoms(arg, context.implicit_timezone())?;
    let (total, count) = if let Some(r) = r {
        r
    } else {
        return Ok(None);
    };
    let count: IBig = count.into();
    Ok(Some(op_div(total, count.into())?))
}

#[xpath_fn(
    "fn:max($arg as xs:anyAtomicType*, $collation as xs:string) as xs:anyAtomicType?",
    collation
)]
fn max(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
    collation: &str,
) -> error::Result<Option<Atomic>> {
    min_or_max(
        context,
        arg,
        collation,
        |atom, max, collation, default_offset| {
            OpGt::atomic_compare(
                atom.clone(),
                max.clone(),
                |a, b| collation.compare(a, b),
                default_offset,
            )
        },
    )
}

#[xpath_fn(
    "fn:min($arg as xs:anyAtomicType*, $collation as xs:string) as xs:anyAtomicType?",
    collation
)]
fn min(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
    collation: &str,
) -> error::Result<Option<Atomic>> {
    min_or_max(
        context,
        arg,
        collation,
        |atom, min, collation, default_offset| {
            OpLt::atomic_compare(
                atom.clone(),
                min.clone(),
                |a, b| collation.compare(a, b),
                default_offset,
            )
        },
    )
}

fn min_or_max<F>(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
    collation: &str,
    compare: F,
) -> error::Result<Option<Atomic>>
where
    F: Fn(Atomic, Atomic, &Collation, chrono::offset::FixedOffset) -> error::Result<bool>,
{
    let mut float_seen: bool = false;
    let mut double_seen: bool = false;
    let mut any_uri_seen: bool = false;
    let mut string_seen: bool = false;

    let mut arg = arg.map(|atom: error::Result<Atomic>| {
        let atom: Atomic = atom?;
        let r = if atom.is_untyped() {
            atom.cast_to_double()?
        } else {
            atom
        };
        match r {
            Atomic::Float(_) => float_seen = true,
            Atomic::Double(_) => double_seen = true,
            Atomic::String(StringType::AnyURI, _) => any_uri_seen = true,
            Atomic::String(StringType::String, _) => string_seen = true,
            _ => {}
        }
        Ok::<Atomic, error::Error>(r)
    });

    if let Some(atom) = arg.next() {
        let mut extreme = atom?;
        // if extreme is not comparable, then we fail
        if !extreme.is_comparable() {
            return Err(error::Error::new(error::ErrorCode::FORG0006));
        }

        let collation = context
            .static_context()
            .resolve_collation_str(Some(collation))?;
        let default_offset = context.implicit_timezone();

        // now compare the other arguments with extreme
        for atom in arg {
            let atom = atom?;

            // we want to handle NaN specifically; we do
            // want to record it so we can't bail out early,
            // as we need to see whether we need to cast in the end.
            // However, once a NaN has been found, further comparisons
            // should not take place.
            if (atom.is_nan()
                || compare(
                    atom.clone(),
                    extreme.clone(),
                    collation.as_ref(),
                    default_offset,
                )
                .map_err(|_| error::Error::new(error::ErrorCode::FORG0006))?)
                && !extreme.is_nan()
            {
                extreme = atom;
            }
        }
        if double_seen {
            Ok(Some(extreme.cast_to_double()?))
        } else if float_seen {
            Ok(Some(extreme.cast_to_float()?))
        } else if any_uri_seen && string_seen {
            // this will only cast any AnyURI max to string,
            // otherwise the max is already string
            Ok(Some(extreme.cast_to_string()))
        } else {
            Ok(Some(extreme))
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:sum($arg as xs:anyAtomicType*) as xs:anyAtomicType")]
fn sum1(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
) -> error::Result<Atomic> {
    let sum = sum_atoms(arg, context.implicit_timezone())?;
    if let Some((sum, _)) = sum {
        Ok(sum)
    } else {
        Ok(0.into())
    }
}

#[xpath_fn("fn:sum($arg as xs:anyAtomicType*, $zero as xs:anyAtomicType?) as xs:anyAtomicType?")]
fn sum2(
    context: &DynamicContext,
    arg: impl Iterator<Item = error::Result<Atomic>>,
    zero: Option<Atomic>,
) -> error::Result<Option<Atomic>> {
    let sum = sum_atoms(arg, context.implicit_timezone())?;
    if let Some((sum, _)) = sum {
        Ok(Some(sum))
    } else {
        Ok(zero)
    }
}

fn sum_atoms(
    atoms: impl Iterator<Item = error::Result<Atomic>>,
    default_offset: chrono::FixedOffset,
) -> error::Result<Option<(Atomic, usize)>> {
    let mut atoms = atoms.map(|atom| {
        let atom = atom?;
        if atom.is_untyped() {
            atom.cast_to_double()
        } else {
            Ok(atom)
        }
    });

    let total = atoms.next();
    if let Some(total) = total {
        // if the first atomic is not addable, bail out
        let mut total = total?;
        let mut atoms = atoms.peekable();
        if atoms.peek().is_none() && !total.is_addable() {
            return Err(error::Error::new(error::ErrorCode::FORG0006));
        }
        let mut count = 1;
        // now add all the further atoms
        for atom in atoms {
            let atom = atom?;
            count += 1;
            total = op_add(total, atom, default_offset)
                .map_err(|_| error::Error::new(error::ErrorCode::FORG0006))?;
        }
        Ok(Some((total, count)))
    } else {
        Ok(None)
    }
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(empty),
        wrap_xpath_fn!(exists),
        wrap_xpath_fn!(head),
        wrap_xpath_fn!(tail),
        wrap_xpath_fn!(insert_before),
        wrap_xpath_fn!(remove),
        wrap_xpath_fn!(reverse),
        wrap_xpath_fn!(subsequence2),
        wrap_xpath_fn!(subsequence3),
        wrap_xpath_fn!(unordered),
        wrap_xpath_fn!(distinct_values),
        wrap_xpath_fn!(index_of),
        wrap_xpath_fn!(deep_equal),
        wrap_xpath_fn!(zero_or_one),
        wrap_xpath_fn!(one_or_more),
        wrap_xpath_fn!(exactly_one),
        wrap_xpath_fn!(count),
        wrap_xpath_fn!(avg),
        wrap_xpath_fn!(max),
        wrap_xpath_fn!(min),
        wrap_xpath_fn!(sum1),
        wrap_xpath_fn!(sum2),
    ]
}
