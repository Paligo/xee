use crate::stack::Value;
use crate::{atomic::IntegerType, error, sequence::Item};
use std::rc::Rc;
use std::sync::OnceLock;
use xee_xpath_macros::xpath_fn;

use crate::function::StaticFunctionDescription;
use crate::wrap_xpath_fn;
use crate::{
    atomic::{self, Atomic, BinaryType},
    context,
    function::{FunctionKind, Map, StaticFunctionId},
    interpreter::Interpreter,
    sequence::{self, Sequence},
};
use ordered_float::OrderedFloat;
use rand::{seq::SliceRandom, Rng};
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use rand_seeder::Seeder;

static RNG_NEXT_ID: OnceLock<StaticFunctionId> = OnceLock::new();
static RNG_PERMUTE_ID: OnceLock<StaticFunctionId> = OnceLock::new();
const NUMBER_KEY: &str = "number";
const NEXT_KEY: &str = "next";
const PERMUTE_KEY: &str = "permute";

trait Length {
    const LEN: usize;
}

impl<T, const LENGTH: usize> Length for [T; LENGTH] {
    const LEN: usize = LENGTH;
}

fn _random_number_generator(
    context: &context::DynamicContext,
    seed: <ChaCha20Rng as SeedableRng>::Seed,
    mut seed_loc: Option<Rc<[u8]>>,
    word_pos: u128,
) -> error::Result<Map> {
    let mut rng = create_rng(seed, word_pos);
    let next_id = RNG_NEXT_ID.get_or_init(|| {
        context
            .static_context()
            .function_id_by_private_name(RNG_NEXT_NAME)
            .unwrap()
    });
    let permute_id = RNG_PERMUTE_ID.get_or_init(|| {
        context
            .static_context()
            .function_id_by_private_name(RNG_PERMUTE_NAME)
            .unwrap()
    });
    let number = atomic::Atomic::Double(OrderedFloat(rng.random()));
    let new_word_pos = rng.get_word_pos();

    let closure_vars: Value = if let Some(seed_loc) = seed_loc.take() {
        to_rng_args_with_seed(seed_loc, new_word_pos).into()
    } else {
        to_rng_args(seed, new_word_pos).into()
    };

    let next_closure = Interpreter::create_static_closure(context, *next_id, || {
        Some(closure_vars.clone())
    })?;
    let permute_closure = Interpreter::create_static_closure(context, *permute_id, || {
        Some(closure_vars.clone())
    })?;
    return Map::new(vec![
        (NUMBER_KEY.into(), number.into()),
        (NEXT_KEY.into(), sequence::Item::from(next_closure).into()),
        (
            PERMUTE_KEY.into(),
            sequence::Item::from(permute_closure).into(),
        ),
    ]);
}

fn create_rng(seed: <ChaCha20Rng as SeedableRng>::Seed, word_pos: u128) -> ChaCha20Rng {
    let mut rng = ChaCha20Rng::from_seed(seed);
    rng.set_word_pos(word_pos);
    rng
}

#[xpath_fn("fn:random-number-generator($seed as xs:anyAtomicType?) as map(xs:string, item())")]
fn random_number_generator(
    context: &context::DynamicContext,
    seed: Option<atomic::Atomic>,
) -> error::Result<Map> {
    let seed = if let Some(seed) = seed {
        Seeder::from(seed).make_seed()
    } else {
        Seeder::from(context.current_datetime()).make_seed()
    };
    return _random_number_generator(context, seed, None, 0);
}

#[xpath_fn("fn:random-number-generator() as map(xs:string, item())")]
fn random_number_generator_empty(context: &context::DynamicContext) -> error::Result<Map> {
    _random_number_generator(
        context,
        Seeder::from(context.current_datetime()).make_seed(),
        None,
        0,
    )
}

const RNG_NEXT_NAME: &str = "rng_next";
macro_rules! rng_next_panic {
    () => {
        "Function called with bad context!"
    };
}

#[xpath_fn(
    "fn:function($seed as xs:anyAtomicType*) as map(xs:string, item())",
    context_last_optional
)]
fn random_number_generator_next<'a>(
    context: &context::DynamicContext,
    args: impl Iterator<Item = error::Result<Atomic>>,
) -> error::Result<Map> {
    let (seed, seed_loc, word_pos) = extract_rng_args_from_context(args)?;
    _random_number_generator(context, seed, Some(seed_loc), word_pos)
}

const RNG_PERMUTE_NAME: &str = "rng_permute";

#[xpath_fn(
    "fn:function($arg as item()*, $seed as xs:anyAtomicType*) as item()*",
    context_last_optional
)]
fn random_number_generator_permute<'a>(
    sequence: impl Iterator<Item = error::Result<Item>>,
    args: impl Iterator<Item = error::Result<Atomic>>,
) -> error::Result<Sequence> {
    let (seed, _, word_pos) = extract_rng_args_from_context(args)?;
    let mut rng = create_rng(seed, word_pos);
    let mut sequence: Vec<Item> = sequence.collect::<Result<_, _>>()?;
    sequence.shuffle(&mut rng);
    Ok(Sequence::new(sequence))
}

fn to_rng_args(seed: <ChaCha20Rng as SeedableRng>::Seed, word_pos: u128) -> Sequence {
    let seed = seed.to_vec().into();
    to_rng_args_with_seed(seed, word_pos)
}

fn to_rng_args_with_seed(seed: Rc<[u8]>, word_pos: u128) -> Sequence {
    Sequence::new(vec![
        Atomic::Binary(BinaryType::Hex, seed).into(),
        Atomic::Integer(IntegerType::Integer, std::rc::Rc::new(word_pos.into())).into(),
    ])
}

fn extract_rng_args_from_context(
    mut args: impl Iterator<Item = error::Result<Atomic>>,
) -> error::Result<(<ChaCha20Rng as SeedableRng>::Seed, Rc<[u8]>, u128)> {
    let Atomic::Binary(_, seed) = args.next().expect(rng_next_panic!())? else {
        panic!(rng_next_panic!())
    };
    let Atomic::Integer(_, word_pos) = args.next().expect(rng_next_panic!())? else {
        panic!(rng_next_panic!())
    };
    Ok((
        seed.as_ref().try_into().expect(rng_next_panic!()),
        seed,
        word_pos.as_ref().try_into().expect(rng_next_panic!()),
    ))
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(random_number_generator),
        wrap_xpath_fn!(random_number_generator_empty),
        StaticFunctionDescription::new_private(
            random_number_generator_next::WRAPPER,
            random_number_generator_next::SIGNATURE,
            RNG_NEXT_NAME,
            FunctionKind::parse(random_number_generator_next::KIND),
            &xee_name::Namespaces::default(),
        ),
        StaticFunctionDescription::new_private(
            random_number_generator_permute::WRAPPER,
            random_number_generator_permute::SIGNATURE,
            RNG_PERMUTE_NAME,
            FunctionKind::parse(random_number_generator_permute::KIND),
            &xee_name::Namespaces::default(),
        ),
    ]
}
