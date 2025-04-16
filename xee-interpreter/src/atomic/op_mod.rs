use std::rc::Rc;

use ibig::IBig;
use num_traits::Float;
use num_traits::Zero;
use rust_decimal::Decimal;

use crate::atomic;
use crate::error;

use super::cast_binary::cast_binary_arithmetic;

pub(crate) fn op_mod(a: atomic::Atomic, b: atomic::Atomic) -> error::Result<atomic::Atomic> {
    use atomic::Atomic;

    let (a, b) = cast_binary_arithmetic(a, b)?;

    match (a, b) {
        (Atomic::Decimal(a), Atomic::Decimal(b)) => Ok(op_mod_decimal(a, b)?),
        (Atomic::Integer(_, a), Atomic::Integer(_, b)) => Ok(op_mod_integer(a, b)?),
        (Atomic::Float(a), Atomic::Float(b)) => Ok(op_mod_float(a, b)?),
        (Atomic::Double(a), Atomic::Double(b)) => Ok(op_mod_float(a, b)?),
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

fn op_mod_decimal(a: Rc<Decimal>, b: Rc<Decimal>) -> error::Result<atomic::Atomic> {
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    Ok((a.as_ref() % b.as_ref()).into())
}

fn op_mod_integer(a: Rc<IBig>, b: Rc<IBig>) -> error::Result<atomic::Atomic> {
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    Ok((a.as_ref() % b.as_ref()).into())
}

fn op_mod_float<F>(a: F, b: F) -> error::Result<atomic::Atomic>
where
    F: Float + Into<atomic::Atomic>,
{
    Ok((a % b).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numeric_mod_nan_nan() {
        let a = f64::NAN.into();
        let b = f64::NAN.into();
        let result = op_mod(a, b).unwrap();
        assert!(result.is_nan());
    }
}
