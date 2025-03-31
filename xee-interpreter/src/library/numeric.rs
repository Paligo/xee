// https://www.w3.org/TR/xpath-functions-31/#numeric-functions
use ahash::random_state::RandomState;
use ibig::ops::Abs;
use ibig::IBig;
use num_traits::Float;
use rand::prelude::*;
use rand_xoshiro::SplitMix64;

use xee_name::{Name, FN_NAMESPACE};
use xee_xpath_macros::xpath_fn;

use crate::atomic::round_atomic;
use crate::atomic::round_half_to_even_atomic;
use crate::atomic::Atomic;
use crate::context;
use crate::error;
use crate::function;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::sequence;
use crate::wrap_xpath_fn;

#[xpath_fn("fn:abs($arg as xs:numeric?) as xs:numeric?")]
fn abs(arg: Option<Atomic>) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        match arg {
            Atomic::Integer(_, i) => Ok(Some(i.as_ref().abs().into())),
            Atomic::Decimal(d) => Ok(Some(d.abs().into())),
            Atomic::Float(f) => Ok(Some(f.abs().into())),
            Atomic::Double(d) => Ok(Some(d.abs().into())),
            _ => Err(error::Error::XPTY0004),
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:ceiling($arg as xs:numeric?) as xs:numeric?")]
fn ceiling(arg: Option<Atomic>) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        match arg {
            Atomic::Integer(_, _) => Ok(Some(arg.clone())),
            Atomic::Decimal(d) => Ok(Some(d.ceil().into())),
            Atomic::Float(f) => Ok(Some(f.ceil().into())),
            Atomic::Double(d) => Ok(Some(d.ceil().into())),
            _ => Err(error::Error::XPTY0004),
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:floor($arg as xs:numeric?) as xs:numeric?")]
fn floor(arg: Option<Atomic>) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        match arg {
            Atomic::Integer(_, _) => Ok(Some(arg.clone())),
            Atomic::Decimal(d) => Ok(Some(d.floor().into())),
            Atomic::Float(f) => Ok(Some(f.floor().into())),
            Atomic::Double(d) => Ok(Some(d.floor().into())),
            _ => Err(error::Error::XPTY0004),
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:round($arg as xs:numeric?) as xs:numeric?")]
fn round1(arg: Option<Atomic>) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        round_atomic(arg, 0).map(Some)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:round($arg as xs:numeric?, $precision as xs:integer) as xs:numeric?")]
fn round2(arg: Option<Atomic>, precision: IBig) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        let precision: i32 = precision.try_into().map_err(|_| error::Error::FOAR0002)?;
        round_atomic(arg, precision).map(Some)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:round-half-to-even($arg as xs:numeric?) as xs:numeric?")]
fn round_half_to_even1(arg: Option<Atomic>) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        round_half_to_even_atomic(arg, 0).map(Some)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:round-half-to-even($arg as xs:numeric?, $precision as xs:integer) as xs:numeric?")]
fn round_half_to_even2(arg: Option<Atomic>, precision: IBig) -> error::Result<Option<Atomic>> {
    if let Some(arg) = arg {
        let precision: i32 = precision.try_into().map_err(|_| error::Error::FOAR0002)?;
        round_half_to_even_atomic(arg, precision).map(Some)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:number($arg as xs:anyAtomicType?) as xs:double", context_first)]
fn number(arg: Option<Atomic>) -> error::Result<Atomic> {
    if let Some(arg) = arg {
        match arg.cast_to_double() {
            Ok(d) => Ok(d),
            Err(_) => Ok(f64::NAN.into()),
        }
    } else {
        Ok(f64::NAN.into())
    }
}

#[xpath_fn("fn:random-number-generator() as map(xs:string, item())")]
fn random_number_generator0(context: &context::DynamicContext) -> error::Result<function::Map> {
    random_number_generator1(context, None)
}

#[xpath_fn("fn:random-number-generator($seed as xs:anyAtomicType?) as map(xs:string, item())")]
fn random_number_generator1(
    context: &context::DynamicContext,
    seed: Option<Atomic>,
) -> error::Result<function::Map> {
    // use a hash function with a fixed seed
    let random_state = RandomState::with_seeds(0, 0, 0, 0);
    let seed = if let Some(seed) = seed {
        random_state.hash_one(seed)
    } else {
        random_state.hash_one(context.current_datetime())
    };
    rng_object(context, seed)
}

fn rng_object(context: &context::DynamicContext, seed: u64) -> error::Result<function::Map> {
    // XPath 3.1 states that all xs:double values in [0.0, 1.0) SHOULD be
    // equally likely, but a mathematically uniform distribution is clearly
    // the intent.
    let number = SplitMix64::seed_from_u64(seed).gen_range(0.0..1.0);
    let static_context = context.static_context();
    let next_name = Name::new(
        "_rng-next".to_string(),
        FN_NAMESPACE.to_string(),
        String::new(),
    );
    let next_id = static_context
        .function_id_by_internal_name(&next_name, 0)
        .unwrap();
    let next = Interpreter::create_static_closure(context, next_id, || Some(seed.into()))?;
    let permute_name = Name::new(
        "_rng-permute".to_string(),
        FN_NAMESPACE.to_string(),
        String::new(),
    );
    let permute_id = static_context
        .function_id_by_internal_name(&permute_name, 1)
        .unwrap();
    let permute = Interpreter::create_static_closure(context, permute_id, || Some(seed.into()))?;
    function::Map::new(vec![
        ("number".into(), number.into()),
        ("next".into(), sequence::Item::from(next).into()),
        ("permute".into(), sequence::Item::from(permute).into()),
    ])
}

#[xpath_fn(
    "fn:_rng-next($seed as xs:unsignedLong) as map(xs:string, item())",
    anonymous_closure
)]
fn rng_next(context: &context::DynamicContext, seed: u64) -> error::Result<function::Map> {
    // this code has the same effect as calling next_u64() on a SplitMix64
    // generator then extracting its state afterward. See the original
    // implementation at:
    // https://github.com/rust-random/rngs/blob/rand_xoshiro-0.6.0/rand_xoshiro/src/splitmix64.rs#L47-L53
    const PHI: u64 = 0x9e3779b97f4a7c15;
    rng_object(context, seed.wrapping_add(PHI))
}

#[xpath_fn(
    "fn:_rng-permute($arg as item()*, $seed as xs:unsignedLong) as item()*",
    anonymous_closure
)]
fn rng_permute(arg: &sequence::Sequence, seed: u64) -> sequence::Sequence {
    // don't use the seed directly, since rejection sampling can cause
    // consecutive seeds in a next() sequence to produce the same shuffle.
    // Adding a level of indirection breaks up this correlation.
    // TODO: mix an argument-based hash into the seed for better randomness.
    let shuffle_seed = SplitMix64::seed_from_u64(seed).next_u64();
    let mut items = arg.iter().collect::<Vec<_>>();
    items.shuffle(&mut SplitMix64::seed_from_u64(shuffle_seed));
    items.into()
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(abs),
        wrap_xpath_fn!(ceiling),
        wrap_xpath_fn!(floor),
        wrap_xpath_fn!(round1),
        wrap_xpath_fn!(round2),
        wrap_xpath_fn!(round_half_to_even1),
        wrap_xpath_fn!(round_half_to_even2),
        wrap_xpath_fn!(number),
        wrap_xpath_fn!(random_number_generator0),
        wrap_xpath_fn!(random_number_generator1),
        wrap_xpath_fn!(rng_next),
        wrap_xpath_fn!(rng_permute),
    ]
}
