use std::cmp::Ordering;

use ordered_float::OrderedFloat;

use crate::error;

use super::cast_binary::cast_binary_compare;
use super::compare::AtomicCompare;
use super::datetime::EqWithDefaultOffset;
use super::{Atomic, AtomicCompareValue, BinaryType};

pub(crate) struct OpEq;

impl AtomicCompare for OpEq {
    fn atomic_compare<F>(
        a: Atomic,
        b: Atomic,
        string_compare: F,
        default_offset: chrono::FixedOffset,
    ) -> error::Result<bool>
    where
        F: Fn(&str, &str) -> Ordering,
    {
        let (a, b) = cast_binary_compare(a, b)?;

        use Atomic::*;

        // cast guarantees both atomic types are the same concrete atomic
        match (a, b) {
            (Decimal(a), Decimal(b)) => Ok(a == b),
            (Integer(_, a), Integer(_, b)) => Ok(a == b),
            (Float(OrderedFloat(a)), Float(OrderedFloat(b))) => Ok(a == b),
            (Double(OrderedFloat(a)), Double(OrderedFloat(b))) => Ok(a == b),
            (Boolean(a), Boolean(b)) => Ok(a == b),
            (String(_, a), String(_, b)) => Ok(string_compare(a.as_ref(), b.as_ref()).is_eq()),
            (Date(a), Date(b)) => Ok(a
                .as_ref()
                .eq_with_default_offset(b.as_ref(), default_offset)),
            (Time(a), Time(b)) => Ok(a
                .as_ref()
                .eq_with_default_offset(b.as_ref(), default_offset)),
            (DateTime(a), DateTime(b)) => Ok(a
                .as_ref()
                .eq_with_default_offset(b.as_ref(), default_offset)),
            (DateTimeStamp(a), DateTimeStamp(b)) => Ok(a == b),
            (Duration(a), Duration(b)) => Ok(a == b),
            (YearMonthDuration(a), YearMonthDuration(b)) => Ok(a == b),
            (DayTimeDuration(a), DayTimeDuration(b)) => Ok(a == b),
            (GYearMonth(a), GYearMonth(b)) => Ok(a == b),
            (GYear(a), GYear(b)) => Ok(a == b),
            (GMonthDay(a), GMonthDay(b)) => Ok(a == b),
            (GDay(a), GDay(b)) => Ok(a == b),
            (GMonth(a), GMonth(b)) => Ok(a == b),
            (Binary(BinaryType::Hex, a), Binary(BinaryType::Hex, b)) => Ok(a == b),
            (Binary(BinaryType::Base64, a), Binary(BinaryType::Base64, b)) => Ok(a == b),
            (QName(a), QName(b)) => Ok(a == b),
            _ => Err(error::Error::XPTY0004),
        }
    }

    fn arguments_inverted() -> impl AtomicCompare {
        Self
    }

    fn value() -> AtomicCompareValue {
        AtomicCompareValue::Eq
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::Offset;
    use rust_decimal_macros::dec;

    use crate::atomic;
    use crate::atomic::OpNe;

    fn default_offset() -> chrono::FixedOffset {
        chrono::offset::Utc.fix()
    }

    #[test]
    fn test_compare_bytes() {
        let a: atomic::Atomic = 1i8.into();
        let b: atomic::Atomic = 2i8.into();

        assert!(!OpEq::atomic_compare(a.clone(), b.clone(), str::cmp, default_offset()).unwrap());
        assert!(OpNe::atomic_compare(a, b, str::cmp, default_offset()).unwrap());
    }

    #[test]
    fn test_compare_cast_untyped() {
        let a: atomic::Atomic = "foo".into();
        let b: atomic::Atomic = atomic::Atomic::Untyped("foo".to_string().into());

        assert!(OpEq::atomic_compare(a.clone(), b.clone(), str::cmp, default_offset()).unwrap());
        assert!(!OpNe::atomic_compare(a, b, str::cmp, default_offset()).unwrap());
    }

    #[test]
    fn test_compare_cast_decimal_to_double() {
        let a: atomic::Atomic = dec!(1.5).into();
        let b: atomic::Atomic = 1.5f64.into();

        assert!(OpEq::atomic_compare(a.clone(), b.clone(), str::cmp, default_offset()).unwrap());
        assert!(!OpNe::atomic_compare(a, b, str::cmp, default_offset()).unwrap());
    }

    #[test]
    fn test_compare_byte_and_integer() {
        let a: atomic::Atomic = 1i8.into();
        let b: atomic::Atomic = 1i64.into();

        assert!(OpEq::atomic_compare(a.clone(), b.clone(), str::cmp, default_offset()).unwrap());
        assert!(!OpNe::atomic_compare(a, b, str::cmp, default_offset()).unwrap());
    }

    #[test]
    fn test_compare_integer_and_integer() {
        let a: atomic::Atomic = 1i64.into();
        let b: atomic::Atomic = 1i64.into();

        assert!(OpEq::atomic_compare(a.clone(), b.clone(), str::cmp, default_offset()).unwrap());
        assert!(!OpNe::atomic_compare(a, b, str::cmp, default_offset()).unwrap());
    }
}
