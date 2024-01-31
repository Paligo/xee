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
use crate::sequence;
use crate::string::Collation;
use crate::wrap_xpath_fn;

#[xpath_fn("fn:empty($arg as item()*) as xs:boolean")]
fn empty(arg: &[sequence::Item]) -> bool {
    arg.is_empty()
}

#[xpath_fn("fn:exists($arg as item()*) as xs:boolean")]
fn exists(arg: &[sequence::Item]) -> bool {
    !arg.is_empty()
}

#[xpath_fn("fn:head($arg as item()*) as item()?")]
fn head(arg: &[sequence::Item]) -> Option<sequence::Item> {
    arg.first().cloned()
}

#[xpath_fn("fn:tail($arg as item()*) as item()*")]
fn tail(arg: &[sequence::Item]) -> sequence::Sequence {
    if arg.is_empty() {
        return sequence::Sequence::empty();
    }
    arg[1..].to_vec().into()
}

#[xpath_fn(
    "fn:insert-before($target as item()*, $position as xs:integer, $inserts as item()*) as item()*"
)]
fn insert_before(
    target: &[sequence::Item],
    position: IBig,
    inserts: &[sequence::Item],
) -> error::Result<sequence::Sequence> {
    if target.is_empty() {
        return Ok(inserts.to_vec().into());
    }
    let position = if position < IBig::from(0) {
        IBig::from(0)
    } else {
        position
    };
    let position: usize = position.try_into().map_err(|_| error::Error::FOAR0002)?;
    let position = position.saturating_sub(1);
    let position = if position > target.len() {
        target.len()
    } else {
        position
    };

    let items = target[0..position]
        .iter()
        .chain(inserts)
        .chain(target[position..].iter())
        .cloned()
        .collect::<Vec<_>>();
    Ok(items.into())
}

#[xpath_fn("fn:remove($target as item()*, $position as xs:integer) as item()*")]
fn remove(target: &[sequence::Item], position: IBig) -> error::Result<sequence::Sequence> {
    let position = if position < IBig::from(0) {
        IBig::from(0)
    } else {
        position
    };
    let position: usize = position.try_into().map_err(|_| error::Error::FOAR0002)?;
    if position == 0 || position > target.len() {
        // TODO: unfortunate we can't just copy sequence
        return Ok(target.to_vec().into());
    }
    let mut target = target.to_vec();
    let position = position.saturating_sub(1);
    target.remove(position);
    Ok(target.into())
}

#[xpath_fn("fn:reverse($arg as item()*) as item()*")]
fn reverse(arg: &[sequence::Item]) -> sequence::Sequence {
    let mut items = arg.to_vec();
    items.reverse();
    items.into()
}

#[xpath_fn("fn:subsequence($sourceSeq as item()*, $startingLoc as xs:double) as item()*")]
fn subsequence2(source_seq: &[sequence::Item], starting_loc: f64) -> Vec<sequence::Item> {
    if starting_loc.is_nan() {
        return Vec::new();
    }
    let starting_loc = starting_loc - 1.0;
    let starting_loc = starting_loc.clamp(0.0, (source_seq.len()) as f64);
    let starting_loc = starting_loc as usize;
    source_seq[starting_loc..].to_vec()
}

#[xpath_fn(
    "fn:subsequence($sourceSeq as item()*, $startingLoc as xs:double, $length as xs:double) as item()*"
)]
fn subsequence3(
    source_seq: &[sequence::Item],
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

    source_seq[starting_loc..end].to_vec()
}

#[xpath_fn("fn:unordered($sourceSeq as item()*) as item()*")]
fn unordered(source_seq: &sequence::Sequence) -> sequence::Sequence {
    // TODO: annoying that a clone is needed there.
    // would be better if we could get an Rc of sequence so the clone is
    // much more cheap
    source_seq.clone()
}

#[xpath_fn(
    "fn:distinct-values($arg as xs:anyAtomicType*, $collation as xs:string) as xs:anyAtomicType*",
    collation
)]
fn distinct_values(
    context: &DynamicContext,
    arg: &[Atomic],
    collation: &str,
) -> error::Result<Vec<Atomic>> {
    if arg.is_empty() {
        return Ok(Vec::new());
    }
    let collation = context.static_context.collation(collation)?;
    let default_offset = context.implicit_timezone();
    // we use a HashMap first to remove items to compare. It removes easy
    // duplicates. It can't generate false positives as the default
    // string compare is in use. We store the order in the value.
    let distinct_set = arg
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, atom)| (atom, i))
        .collect::<HashMap<_, _>>();

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
    seq: &[Atomic],
    search: Atomic,
    collation: &str,
) -> error::Result<Vec<IBig>> {
    let collation = context.static_context.collation(collation)?;
    let default_offset = context.implicit_timezone();
    // TODO: annoying that we have to clone both atoms here
    let indices = seq.iter().enumerate().filter_map(|(i, atom)| {
        if atom.equal(&search, &collation, default_offset) {
            Some((i + 1).into())
        } else {
            None
        }
    });
    Ok(indices.collect::<Vec<_>>())
}

#[xpath_fn("fn:deep-equal($parameter1 as item()*, $parameter2 as item()*, $collation as xs:string) as xs:boolean", collation)]
fn deep_equal(
    context: &DynamicContext,
    parameter1: &sequence::Sequence,
    parameter2: &sequence::Sequence,
    collation: &str,
) -> error::Result<bool> {
    let collation = context.static_context.collation(collation)?;
    let default_offset = context.implicit_timezone();
    parameter1.deep_equal(parameter2, &collation, default_offset, context.xot())
}

#[xpath_fn("fn:zero-or-one($arg as item()*) as item()?")]
fn zero_or_one(arg: &[sequence::Item]) -> error::Result<Option<sequence::Item>> {
    match arg.len() {
        0 => Ok(None),
        1 => Ok(Some(arg[0].clone())),
        _ => Err(error::Error::FORG0003),
    }
}

#[xpath_fn("fn:one-or-more($arg as item()*) as item()+")]
fn one_or_more(arg: &[sequence::Item]) -> error::Result<sequence::Sequence> {
    if arg.is_empty() {
        Err(error::Error::FORG0004)
    } else {
        Ok(arg.to_vec().into())
    }
}

#[xpath_fn("fn:exactly-one($arg as item()*) as item()")]
fn exactly_one(arg: &[sequence::Item]) -> error::Result<sequence::Item> {
    if arg.len() == 1 {
        Ok(arg[0].clone())
    } else {
        Err(error::Error::FORG0005)
    }
}

#[xpath_fn("fn:count($arg as item()*) as xs:integer")]
fn count(arg: &[sequence::Item]) -> IBig {
    arg.len().into()
}

#[xpath_fn("fn:avg($arg as xs:anyAtomicType*) as xs:anyAtomicType?")]
fn avg(context: &DynamicContext, arg: &[Atomic]) -> error::Result<Option<Atomic>> {
    if arg.is_empty() {
        return Ok(None);
    }
    let total = sum_atoms(arg[0].clone(), &arg[1..], context.implicit_timezone())?;
    let count: IBig = arg.len().into();
    Ok(Some(op_div(total, count.into())?))
}

#[xpath_fn(
    "fn:max($arg as xs:anyAtomicType*, $collation as xs:string) as xs:anyAtomicType?",
    collation
)]
fn max(context: &DynamicContext, arg: &[Atomic], collation: &str) -> error::Result<Option<Atomic>> {
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
fn min(context: &DynamicContext, arg: &[Atomic], collation: &str) -> error::Result<Option<Atomic>> {
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
    arg: &[Atomic],
    collation: &str,
    compare: F,
) -> error::Result<Option<Atomic>>
where
    F: Fn(Atomic, Atomic, &Collation, chrono::offset::FixedOffset) -> error::Result<bool>,
{
    if !arg.is_empty() {
        let collation = context.static_context.collation(collation)?;
        let default_offset = context.implicit_timezone();
        let mut float_seen: bool = false;
        let mut double_seen: bool = false;
        let mut any_uri_seen: bool = false;
        let mut string_seen: bool = false;
        let mut arg_iter = arg.iter().map(|atom| {
            let atom = if atom.is_untyped() {
                atom.clone().cast_to_double()
            } else {
                Ok(atom.clone())
            };
            match atom {
                Ok(Atomic::Float(_)) => float_seen = true,
                Ok(Atomic::Double(_)) => double_seen = true,
                Ok(Atomic::String(StringType::AnyURI, _)) => any_uri_seen = true,
                Ok(Atomic::String(StringType::String, _)) => string_seen = true,
                _ => {}
            }
            atom
        });
        // unwrap is safe as we know it's not empty
        let mut extreme = arg_iter.next().unwrap()?;
        // if we know we don't have any more items, and max is
        // not comparable, then we fail
        if arg.len() == 1 && !extreme.is_comparable() {
            return Err(error::Error::FORG0006);
        }

        for atom in arg_iter {
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
                .map_err(|_| error::Error::FORG0006)?)
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
fn sum1(context: &DynamicContext, arg: &[Atomic]) -> error::Result<Atomic> {
    if arg.is_empty() {
        return Ok(0.into());
    }
    sum_atoms(arg[0].clone(), &arg[1..], context.implicit_timezone())
}

#[xpath_fn("fn:sum($arg as xs:anyAtomicType*, $zero as xs:anyAtomicType?) as xs:anyAtomicType")]
fn sum2(
    context: &DynamicContext,
    arg: &[Atomic],
    zero: Option<Atomic>,
) -> error::Result<Option<Atomic>> {
    if arg.is_empty() {
        return Ok(zero);
    }
    Ok(Some(sum_atoms(
        arg[0].clone(),
        &arg[1..],
        context.implicit_timezone(),
    )?))
}

fn sum_atoms(
    total: Atomic,
    arg: &[Atomic],
    default_offset: chrono::FixedOffset,
) -> error::Result<Atomic> {
    let mut total = if total.is_untyped() {
        total.clone().cast_to_double()?
    } else {
        total.clone()
    };
    if arg.is_empty() && !total.is_addable() {
        return Err(error::Error::FORG0006);
    }
    for atom in arg {
        let atom = if atom.is_untyped() {
            atom.clone()
                .cast_to_double()
                .map_err(|_| error::Error::FORG0006)?
        } else {
            atom.clone()
        };
        total = op_add(total, atom, default_offset).map_err(|_| error::Error::FORG0006)?;
    }
    Ok(total)
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
