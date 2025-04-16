use std::{cmp::Ordering, rc::Rc};

use ibig::{ops::Abs, IBig};
use num::Zero;
use ordered_float::OrderedFloat;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::{atomic, error};

// Normal round

pub(crate) fn round_atomic(arg: atomic::Atomic, precision: i32) -> error::Result<atomic::Atomic> {
    match arg {
        atomic::Atomic::Integer(_, i) => round_integer(i, precision),
        atomic::Atomic::Decimal(d) => round_decimal(*d, precision),
        // even though the spec claims we should cast to an infinite
        // precision decimal, we don't have such a thing, so we
        // make do with doing the operation directly on f32 and f64
        atomic::Atomic::Float(OrderedFloat(f)) => Ok(round_float(f, precision)?.into()),
        atomic::Atomic::Double(OrderedFloat(d)) => Ok(round_float(d, precision)?.into()),
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

fn round_integer(i: Rc<IBig>, precision: i32) -> Result<atomic::Atomic, error::Error> {
    if precision < 0 {
        Ok(round_integer_negative(
            i.as_ref().clone(),
            precision.unsigned_abs(),
        ))
    } else {
        Ok(i.into())
    }
}

fn round_integer_negative(arg: IBig, precision: u32) -> atomic::Atomic {
    // TODO: this is definitely not the most optimized way to
    // implement this.

    // The qt3 test suite doesn't seem to cover
    // the integer case very well either, so I wrote a few more tests.
    let d = 10u32.pow(precision);
    let mut divided = arg.clone() / d;
    let remainder = arg.clone() % d;
    if remainder.abs() > (d / 2).into() {
        if arg < 0.into() {
            divided -= 1;
        } else {
            divided += 1;
        }
    }
    (divided * d).into()
}

fn round_decimal(arg: Decimal, precision: i32) -> error::Result<atomic::Atomic> {
    let rounding_strategy = if arg >= Decimal::from(0) {
        RoundingStrategy::MidpointAwayFromZero
    } else {
        RoundingStrategy::MidpointTowardZero
    };
    match precision.cmp(&0) {
        Ordering::Equal | Ordering::Greater => Ok(arg
            .round_dp_with_strategy(precision as u32, rounding_strategy)
            .into()),
        Ordering::Less => {
            let d: Decimal = 10u32.pow(precision.unsigned_abs()).into();
            let arg = arg / d;
            let arg = arg.round_dp_with_strategy(0, rounding_strategy);
            let arg = arg * d;
            Ok(arg.into())
        }
    }
}

fn round_float<F: num_traits::Float>(arg: F, precision: i32) -> error::Result<F> {
    if arg.is_nan() || arg.is_infinite() || arg.is_zero() {
        return Ok(arg);
    }

    match precision.cmp(&0) {
        Ordering::Equal => Ok(round_float_ties_to_positive_infinity(arg)),
        Ordering::Greater => {
            let d = 10i32.pow(precision.unsigned_abs());
            let d = F::from(d);
            if let Some(d) = d {
                Ok(round_float_ties_to_positive_infinity(arg * d) / d)
            } else {
                Err(error::Error::new(error::ErrorCode::FOAR0001))
            }
        }
        Ordering::Less => {
            let d = 10i32.pow(precision.unsigned_abs());
            let d = F::from(d);
            if let Some(d) = d {
                Ok(round_float_ties_to_positive_infinity(arg / d) * d)
            } else {
                Err(error::Error::new(error::ErrorCode::FOAR0001))
            }
        }
    }
}

fn round_float_ties_to_positive_infinity<F: num_traits::Float>(x: F) -> F {
    let y = x.floor();
    if x == y {
        x
    } else {
        let z = ((x + x) - y).floor();
        z.copysign(x)
    }
}

// Round half to even

pub(crate) fn round_half_to_even_atomic(
    arg: atomic::Atomic,
    precision: i32,
) -> error::Result<atomic::Atomic> {
    match arg {
        atomic::Atomic::Integer(_, i) => round_half_to_even_integer(i, precision),
        atomic::Atomic::Decimal(d) => Ok(round_half_to_even_decimal(*d, precision).into()),
        // even though the spec claims we should cast to an infinite
        // precision decimal, we don't have such a thing, so we
        // make do with doing the operation directly on f32 and f64
        atomic::Atomic::Float(OrderedFloat(f)) => {
            if f.is_nan() || f.is_infinite() || f.is_zero() {
                return Ok(f.into());
            }
            // turn f into a Decimal
            // we have to retain the excess bits here, as that's what the spec
            // says
            let f = Decimal::from_f32_retain(f);
            if let Some(f) = f {
                let f = round_half_to_even_decimal(f, precision);
                // turn f back into a float
                let f: f32 = f
                    .try_into()
                    .map_err(|_| error::Error::new(error::ErrorCode::FOAR0001))?;
                Ok(f.into())
            } else {
                Err(error::Error::new(error::ErrorCode::FOCA0001))
            }
        }
        atomic::Atomic::Double(OrderedFloat(d)) => {
            if d.is_nan() || d.is_infinite() || d.is_zero() {
                return Ok(d.into());
            }
            // turn d into a Decimal
            // we have to retain the excess bits here, as that's what the spec
            // says
            let d = Decimal::from_f64_retain(d);
            if let Some(d) = d {
                let d = round_half_to_even_decimal(d, precision);
                // turn d back into a double
                let d: f64 = d
                    .try_into()
                    .map_err(|_| error::Error::new(error::ErrorCode::FOAR0001))?;
                Ok(d.into())
            } else {
                Err(error::Error::new(error::ErrorCode::FOCA0001))
            }
        }
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

fn round_half_to_even_integer(i: Rc<IBig>, precision: i32) -> Result<atomic::Atomic, error::Error> {
    if precision < 0 {
        Ok(round_half_to_even_integer_negative(
            i.as_ref().clone(),
            precision.unsigned_abs(),
        ))
    } else {
        Ok(i.into())
    }
}

fn round_half_to_even_integer_negative(arg: IBig, precision: u32) -> atomic::Atomic {
    let d = 10u32.pow(precision);
    let mut divided = arg.clone() / d;
    let remainder = arg.clone() % d;
    let halfway = d / 2;

    let remainder_abs = remainder.abs();
    if remainder_abs > halfway.into()
        || (remainder_abs == halfway.into() && divided.clone() % 2 != 0)
    {
        if arg < 0.into() {
            divided -= 1;
        } else {
            divided += 1;
        }
    }

    (divided * d).into()
}

// Round half to even (bankers' rounding) for decimal
// we also support negative precision
// in case of half-way, we go to the lowest even number
fn round_half_to_even_decimal(x: Decimal, precision: i32) -> Decimal {
    match precision.cmp(&0) {
        Ordering::Equal | Ordering::Greater => {
            x.round_dp_with_strategy(precision as u32, RoundingStrategy::MidpointNearestEven)
        }
        Ordering::Less => {
            // round-half-to-even(12450.00, -2) = 12400
            // round-half-to-even(12350.00, -2) = 12400
            let d = Decimal::new(10i64.pow(precision.unsigned_abs()), 0);
            let x = x / d;
            let x = x.round_dp_with_strategy(0, RoundingStrategy::MidpointNearestEven);
            x * d
        }
    }
}
