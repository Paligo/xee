use std::rc::Rc;

use ibig::IBig;
use num_traits::Float;
use num_traits::Zero;
use ordered_float::OrderedFloat;
use rust_decimal::Decimal;

use crate::atomic;
use crate::error;

use super::cast_binary::cast_binary_arithmetic;
use super::cast_numeric::duration_i64;
use super::datetime::YearMonthDuration;

pub(crate) fn op_div(a: atomic::Atomic, b: atomic::Atomic) -> error::Result<atomic::Atomic> {
    use atomic::Atomic;

    let (a, b) = cast_binary_arithmetic(a, b)?;

    match (a, b) {
        (Atomic::Decimal(a), Atomic::Decimal(b)) => Ok(op_div_decimal(a, b)?.into()),
        (Atomic::Integer(_, a), Atomic::Integer(_, b)) => Ok(op_div_integer(a, b)?),
        (Atomic::Float(a), Atomic::Float(b)) => Ok(op_div_float(a, b).into()),
        (Atomic::Double(a), Atomic::Double(b)) => Ok(op_div_float(a, b).into()),
        // op:divide-yearMonthDuration(A, B) -> xs:yearMonthDuration
        (Atomic::YearMonthDuration(a), b @ Atomic::Decimal(_)) => {
            Ok(op_divide_year_month_duration_by_atomic(a, b)?)
        }
        (Atomic::YearMonthDuration(a), b @ Atomic::Integer(_, _)) => {
            Ok(op_divide_year_month_duration_by_atomic(a, b)?)
        }
        (Atomic::YearMonthDuration(a), b @ Atomic::Float(_)) => {
            Ok(op_divide_year_month_duration_by_atomic(a, b)?)
        }
        (Atomic::YearMonthDuration(a), Atomic::Double(OrderedFloat(b))) => {
            Ok(op_divide_year_month_duration_by_double(a, b)?)
        }
        // op:divide-dayTimeDuration(A, B) -> xs:dayTimeDuration
        (Atomic::DayTimeDuration(a), b @ Atomic::Decimal(_)) => {
            Ok(op_divide_day_time_duration_by_atomic(a, b)?)
        }
        (Atomic::DayTimeDuration(a), b @ Atomic::Integer(_, _)) => {
            Ok(op_divide_day_time_duration_by_atomic(a, b)?)
        }
        (Atomic::DayTimeDuration(a), b @ Atomic::Float(_)) => {
            Ok(op_divide_day_time_duration_by_atomic(a, b)?)
        }
        (Atomic::DayTimeDuration(a), Atomic::Double(OrderedFloat(b))) => {
            Ok(op_divide_day_time_duration_by_double(a, b)?)
        }
        // op:divide-yearMonthDuration-by-yearMonthDuration (A, B) -> xs:decimal
        (Atomic::YearMonthDuration(a), Atomic::YearMonthDuration(b)) => {
            Ok(op_divide_year_month_duration_by_year_month_duration(a, b)?)
        }
        // op:divide-dayTimeDuration-by-dayTimeDuration (A, B) -> xs:decimal
        (Atomic::DayTimeDuration(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_divide_day_time_duration_by_day_time_duration(a, b)?)
        }
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

pub(crate) fn op_div_decimal(a: Rc<Decimal>, b: Rc<Decimal>) -> error::Result<Decimal> {
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    a.checked_div(*b.as_ref())
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
}

fn op_div_integer(a: Rc<IBig>, b: Rc<IBig>) -> error::Result<atomic::Atomic> {
    // As a special case, if the types of both $arg1 and $arg2 are
    // xs:integer, then the return type is xs:decimal.
    let a: i128 = a
        .as_ref()
        .try_into()
        .map_err(|_| error::Error::new(error::ErrorCode::FOCA0001))?;
    let b: i128 = b
        .as_ref()
        .try_into()
        .map_err(|_| error::Error::new(error::ErrorCode::FOCA0001))?;
    let a: Decimal = a.into();
    let b: Decimal = b.into();
    let v = op_div_decimal(a.into(), b.into())?;
    Ok(v.into())
}

pub(crate) fn op_div_float<F>(a: F, b: F) -> F
where
    F: Float,
{
    a / b
}

fn op_divide_year_month_duration_by_atomic(
    a: YearMonthDuration,
    b: atomic::Atomic,
) -> error::Result<atomic::Atomic> {
    let b = b.cast_to_double()?;
    let b = match b {
        atomic::Atomic::Double(OrderedFloat(b)) => b,
        _ => unreachable!(),
    };
    op_divide_year_month_duration_by_double(a, b)
}

fn op_divide_year_month_duration_by_double(
    a: YearMonthDuration,
    b: f64,
) -> error::Result<atomic::Atomic> {
    if b.is_nan() {
        return Err(error::Error::new(error::ErrorCode::FOCA0005));
    }
    let total = duration_i64(a.months as f64 / b)?;
    Ok(YearMonthDuration::new(total).into())
}

fn op_divide_day_time_duration_by_atomic(
    a: Rc<chrono::Duration>,
    b: atomic::Atomic,
) -> error::Result<atomic::Atomic> {
    let b = b.cast_to_double()?;
    let b = match b {
        atomic::Atomic::Double(OrderedFloat(b)) => b,
        _ => unreachable!(),
    };
    op_divide_day_time_duration_by_double(a, b)
}

fn op_divide_day_time_duration_by_double(
    a: Rc<chrono::Duration>,
    b: f64,
) -> error::Result<atomic::Atomic> {
    if b.is_nan() {
        return Err(error::Error::new(error::ErrorCode::FOCA0005));
    }
    if b.is_zero() {
        return Err(error::Error::new(error::ErrorCode::FODT0001));
    }
    let a = a.num_milliseconds() as f64;
    let total = duration_i64(a / b)?;
    Ok(chrono::Duration::milliseconds(total).into())
}

fn op_divide_year_month_duration_by_year_month_duration(
    a: YearMonthDuration,
    b: YearMonthDuration,
) -> error::Result<atomic::Atomic> {
    if b.months == 0 {
        return Err(error::Error::new(error::ErrorCode::FOAR0001));
    }
    let a: Decimal = a.months.into();
    let b: Decimal = b.months.into();
    Ok((a / b).into())
}

fn op_divide_day_time_duration_by_day_time_duration(
    a: Rc<chrono::Duration>,
    b: Rc<chrono::Duration>,
) -> error::Result<atomic::Atomic> {
    let a = a.num_milliseconds();
    let b = b.num_milliseconds();
    if b == 0 {
        return Err(error::Error::new(error::ErrorCode::FODT0002));
    }
    let a: Decimal = a.into();
    let b: Decimal = b.into();
    Ok((a / b).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_decimal_macros::dec;

    #[test]
    fn test_integer_division_returns_decimal() {
        let a = 5i64.into();
        let b = 2i64.into();
        let result = op_div(a, b).unwrap();
        assert_eq!(result, dec!(2.5).into());
    }

    #[test]
    fn test_numeric_divide_both_integer_returns_decimal() {
        let a = 1i64.into();
        let b = 2i64.into();
        let result = op_div(a, b).unwrap();
        assert_eq!(result, dec!(0.5).into());
    }
}
