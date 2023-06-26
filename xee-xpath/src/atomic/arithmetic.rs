// atomized the operand
// if atomized is empty, result is empty
// if atomized is greater than one, type error
// if atomized xs:untypedAtomic, cast to xs:double
// also type substitution, promotion

// so this is like a function that looks ike:

// fn:op($arg1 as xs:numeric, $arg2 as xs:numeric) as xs:numeric

// with an additional untypedAtomic casting rule

// function conversion rules:
// atomization
// xs:untypedAtomic cast to expected type
// numeric items promoted to expectd atomic type
// xs:anyURI promoted too
// also type substitution

use num_traits::{Float, PrimInt};
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;

use crate::atomic;
use crate::error;

// TODO:
// type check to see whether it conforms to signature, get out atomized,
// like in the function signature. This takes care of subtype
// relations as that's just a check

// TODO:
// if untypedAtomic, passed through typecheck and cast to double happens
// now do type promotion, conforming the arguments to each other

pub(crate) fn numeric_arithmetic_op<O>(
    a: atomic::Atomic,
    b: atomic::Atomic,
) -> error::Result<atomic::Atomic>
where
    O: ArithmeticOp,
{
    // we need to extract the values and pass them along now
    match (a, b) {
        (atomic::Atomic::Decimal(a), atomic::Atomic::Decimal(b)) => {
            <O as ArithmeticOp>::decimal_atomic(a, b)
        }
        (atomic::Atomic::Integer(a), atomic::Atomic::Integer(b)) => {
            <O as ArithmeticOp>::integer_atomic::<i64>(a, b)
        }
        (atomic::Atomic::Int(a), atomic::Atomic::Int(b)) => {
            <O as ArithmeticOp>::integer_atomic::<i32>(a, b)
        }
        (atomic::Atomic::Short(a), atomic::Atomic::Short(b)) => {
            <O as ArithmeticOp>::integer_atomic::<i16>(a, b)
        }
        (atomic::Atomic::Byte(a), atomic::Atomic::Byte(b)) => {
            <O as ArithmeticOp>::integer_atomic::<i8>(a, b)
        }
        (atomic::Atomic::UnsignedLong(a), atomic::Atomic::UnsignedLong(b)) => {
            <O as ArithmeticOp>::integer_atomic::<u64>(a, b)
        }
        (atomic::Atomic::UnsignedInt(a), atomic::Atomic::UnsignedInt(b)) => {
            <O as ArithmeticOp>::integer_atomic::<u32>(a, b)
        }
        (atomic::Atomic::UnsignedShort(a), atomic::Atomic::UnsignedShort(b)) => {
            <O as ArithmeticOp>::integer_atomic::<u16>(a, b)
        }
        (atomic::Atomic::UnsignedByte(a), atomic::Atomic::UnsignedByte(b)) => {
            <O as ArithmeticOp>::integer_atomic::<u8>(a, b)
        }
        (atomic::Atomic::Float(OrderedFloat(a)), atomic::Atomic::Float(OrderedFloat(b))) => {
            <O as ArithmeticOp>::float_atomic::<f32>(a, b)
        }
        (atomic::Atomic::Double(OrderedFloat(a)), atomic::Atomic::Double(OrderedFloat(b))) => {
            <O as ArithmeticOp>::float_atomic::<f64>(a, b)
        }
        _ => unreachable!("Both the atomics not the same type"),
    }
}

pub(crate) trait ArithmeticOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt;
    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal>;
    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float;

    fn integer_atomic<I>(a: I, b: I) -> error::Result<atomic::Atomic>
    where
        I: PrimInt + Into<atomic::Atomic> + Into<Decimal>,
    {
        let v = <Self as ArithmeticOp>::integer(a, b)?;
        Ok(v.into())
    }

    fn decimal_atomic(a: Decimal, b: Decimal) -> error::Result<atomic::Atomic> {
        let v = <Self as ArithmeticOp>::decimal(a, b)?;
        Ok(v.into())
    }

    fn float_atomic<F>(a: F, b: F) -> error::Result<atomic::Atomic>
    where
        F: Float + Into<atomic::Atomic>,
    {
        let v = <Self as ArithmeticOp>::float(a, b)?;
        Ok(v.into())
    }
}

pub(crate) struct AddOp;

impl ArithmeticOp for AddOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        a.checked_add(&b).ok_or(error::Error::Overflow)
    }

    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal> {
        a.checked_add(b).ok_or(error::Error::Overflow)
    }

    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float,
    {
        Ok(a + b)
    }
}

pub(crate) struct SubtractOp;

impl ArithmeticOp for SubtractOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        a.checked_sub(&b).ok_or(error::Error::Overflow)
    }

    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal> {
        a.checked_sub(b).ok_or(error::Error::Overflow)
    }

    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float,
    {
        Ok(a - b)
    }
}

pub(crate) struct MultiplyOp;

impl ArithmeticOp for MultiplyOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        a.checked_mul(&b).ok_or(error::Error::Overflow)
    }

    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal> {
        a.checked_mul(b).ok_or(error::Error::Overflow)
    }

    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float,
    {
        Ok(a * b)
    }
}

pub(crate) struct DivideOp;

impl ArithmeticOp for DivideOp {
    fn integer_atomic<I>(a: I, b: I) -> error::Result<atomic::Atomic>
    where
        I: PrimInt + Into<atomic::Atomic> + Into<Decimal>,
    {
        // As a special case, if the types of both $arg1 and $arg2 are
        // xs:integer, then the return type is xs:decimal.
        let a: Decimal = a.into();
        let b: Decimal = b.into();
        let v = <Self as ArithmeticOp>::decimal(a, b)?;
        Ok(v.into())
    }

    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        if b.is_zero() {
            return Err(error::Error::DivisionByZero);
        }
        a.checked_div(&b).ok_or(error::Error::Overflow)
    }

    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal> {
        if b.is_zero() {
            return Err(error::Error::DivisionByZero);
        }
        a.checked_div(b).ok_or(error::Error::Overflow)
    }

    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float,
    {
        Ok(a / b)
    }
}

pub(crate) struct IntegerDivideOp;

impl ArithmeticOp for IntegerDivideOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        if b.is_zero() {
            return Err(error::Error::DivisionByZero);
        }
        a.checked_div(&b).ok_or(error::Error::Overflow)
    }

    fn decimal_atomic(a: Decimal, b: Decimal) -> error::Result<atomic::Atomic> {
        let v = <DivideOp as ArithmeticOp>::decimal(a, b)?;

        Ok(v.trunc().to_i64().ok_or(error::Error::Overflow)?.into())
    }

    fn decimal(_a: Decimal, _b: Decimal) -> error::Result<Decimal> {
        unreachable!();
    }

    fn float_atomic<F>(a: F, b: F) -> error::Result<atomic::Atomic>
    where
        F: Float + Into<atomic::Atomic>,
    {
        let v = <DivideOp as ArithmeticOp>::float(a, b)?;
        Ok(v.trunc().to_i64().ok_or(error::Error::Overflow)?.into())
    }

    fn float<F>(_a: F, _b: F) -> error::Result<F>
    where
        F: Float,
    {
        unreachable!();
    }
}

pub(crate) struct ModOp;

impl ArithmeticOp for ModOp {
    fn integer<I>(a: I, b: I) -> error::Result<I>
    where
        I: PrimInt,
    {
        if b.is_zero() {
            return Err(error::Error::DivisionByZero);
        }
        Ok(a % b)
    }

    fn decimal(a: Decimal, b: Decimal) -> error::Result<Decimal> {
        if b.is_zero() {
            return Err(error::Error::DivisionByZero);
        }
        Ok(a % b)
    }

    fn float<F>(a: F, b: F) -> error::Result<F>
    where
        F: Float,
    {
        Ok(a % b)
    }
}

pub(crate) fn numeric_unary_plus(atomic: atomic::Atomic) -> error::Result<atomic::Atomic> {
    if atomic.is_numeric() {
        Ok(atomic)
    } else {
        Err(error::Error::Type)
    }
}

pub(crate) fn numeric_unary_minus(atomic: atomic::Atomic) -> error::Result<atomic::Atomic> {
    if atomic.is_numeric() {
        match atomic {
            atomic::Atomic::Decimal(v) => Ok(atomic::Atomic::Decimal(-v)),
            atomic::Atomic::Integer(v) => Ok(atomic::Atomic::Integer(-v)),
            atomic::Atomic::Int(v) => Ok(atomic::Atomic::Int(-v)),
            atomic::Atomic::Short(v) => Ok(atomic::Atomic::Short(-v)),
            atomic::Atomic::Byte(v) => Ok(atomic::Atomic::Byte(-v)),
            atomic::Atomic::Float(v) => Ok(atomic::Atomic::Float(-v)),
            atomic::Atomic::Double(v) => Ok(atomic::Atomic::Double(-v)),
            // what is the correct behavior for unsigned types? We could return
            // a signed integer of the same type with overflow behavior if
            // that's not possible, but for now we just refuse to do it.
            atomic::Atomic::UnsignedLong(_) => Err(error::Error::Type),
            atomic::Atomic::UnsignedInt(_) => Err(error::Error::Type),
            atomic::Atomic::UnsignedShort(_) => Err(error::Error::Type),
            atomic::Atomic::UnsignedByte(_) => Err(error::Error::Type),
            _ => unreachable!(),
        }
    } else {
        Err(error::Error::Type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rust_decimal_macros::dec;

    #[test]
    fn test_add_ints() {
        let a = 1i64.into();
        let b = 2i64.into();
        let result = numeric_arithmetic_op::<AddOp>(a, b).unwrap();
        assert_eq!(result, 3i64.into());
    }

    #[test]
    fn test_integer_division_returns_decimal() {
        let a = 5i64.into();
        let b = 2i64.into();
        let result = numeric_arithmetic_op::<DivideOp>(a, b).unwrap();
        assert_eq!(result, dec!(2.5).into());
    }

    #[test]
    fn test_numeric_integer_divide() {
        let a = 5i64.into();
        let b = 2i64.into();
        let result = numeric_arithmetic_op::<IntegerDivideOp>(a, b).unwrap();
        assert_eq!(result, 2i64.into());
    }

    #[test]
    fn test_numeric_integer_divide_float() {
        let a = 5f64.into();
        let b = 2f64.into();
        let result = numeric_arithmetic_op::<IntegerDivideOp>(a, b).unwrap();
        assert_eq!(result, 2i64.into());
    }
}
