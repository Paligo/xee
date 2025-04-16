use std::rc::Rc;

use ibig::IBig;
use num_traits::Float;
use num_traits::ToPrimitive;
use num_traits::Zero;
use rust_decimal::Decimal;

use crate::atomic;
use crate::error;

use super::cast_binary::cast_binary_arithmetic;
use super::op_div::{op_div_decimal, op_div_float};

pub(crate) fn op_idiv(a: atomic::Atomic, b: atomic::Atomic) -> error::Result<atomic::Atomic> {
    use atomic::Atomic;

    let (a, b) = cast_binary_arithmetic(a, b)?;

    match (a, b) {
        (Atomic::Decimal(a), Atomic::Decimal(b)) => Ok(op_idiv_decimal(a, b)?),
        (Atomic::Integer(_, a), Atomic::Integer(_, b)) => Ok(op_idiv_integer(a, b)?),
        (Atomic::Float(a), Atomic::Float(b)) => Ok(op_idiv_float(a, b)?),
        (Atomic::Double(a), Atomic::Double(b)) => Ok(op_idiv_float(a, b)?),
        _ => Err(error::Error::new(error::ErrorCode::XPST0003)),
    }
}

fn op_idiv_decimal(a: Rc<Decimal>, b: Rc<Decimal>) -> error::Result<atomic::Atomic> {
    let v = op_div_decimal(a, b)?;
    let v: i128 = v
        .trunc()
        .to_i128()
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    let i: IBig = v.into();
    Ok(i.into())
}

fn op_idiv_integer(a: Rc<IBig>, b: Rc<IBig>) -> error::Result<atomic::Atomic> {
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    Ok((a.as_ref() / b.as_ref()).into())
}

fn op_idiv_float<F>(a: F, b: F) -> error::Result<atomic::Atomic>
where
    F: Float + Into<atomic::Atomic>,
{
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    if a.is_nan() || b.is_nan() || a.is_infinite() {
        return Err(error::Error::new(error::ErrorCode::FOAR0002));
    }

    let v = op_div_float(a, b);
    let v: i128 = v
        .trunc()
        .to_i128()
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    let i: IBig = v.into();
    Ok(i.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use ibig::ibig;

    #[test]
    fn test_numeric_integer_divide() {
        let a = 5i64.into();
        let b = 2i64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(2).into());
    }

    #[test]
    fn test_numeric_integer_divide_float() {
        let a = 5f64.into();
        let b = 2f64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(2).into());
    }

    #[test]
    fn test_numeric_integer_divide_10_by_3() {
        let a = 10i64.into();
        let b = 3i64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(3).into());
    }

    #[test]
    fn test_numeric_integer_divide_3_by_minus_2() {
        let a = 3i64.into();
        let b = (-2i64).into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, (ibig!(-1)).into());
    }

    #[test]
    fn test_numeric_integer_divide_minus_3_by_2() {
        let a = (-3i64).into();
        let b = 2i64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, (ibig!(-1)).into());
    }

    #[test]
    fn test_numeric_integer_divide_minus_3_by_minus_2() {
        let a = (-3i64).into();
        let b = (-2i64).into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(1).into());
    }

    #[test]
    fn test_numeric_integer_divide_9_point_0_by_3() {
        let a = 9.0f64.into();
        let b = 3i64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(3).into());
    }

    #[test]
    fn test_numeric_integer_divide_3_point_0_by_4() {
        let a = 3.0f32.into();
        let b = 4i64.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(0).into());
    }

    #[test]
    fn test_numeric_integer_divide_3_by_0() {
        let a = 3i64.into();
        let b = 0i64.into();
        let result = op_idiv(a, b);
        assert_eq!(result, Err(error::Error::new(error::ErrorCode::FOAR0001)));
    }

    #[test]
    fn test_numeric_integer_divide_3_point_0_by_0() {
        let a = 3.0f64.into();
        let b = 0i64.into();
        let result = op_idiv(a, b);
        assert_eq!(result, Err(error::Error::new(error::ErrorCode::FOAR0001)));
    }

    #[test]
    fn test_numeric_integer_divide_3_point_0_by_inf() {
        let a = 3.0f64.into();
        let b = f64::INFINITY.into();
        let result = op_idiv(a, b).unwrap();
        assert_eq!(result, ibig!(0).into());
    }
}
