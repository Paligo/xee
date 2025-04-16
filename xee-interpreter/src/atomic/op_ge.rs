use std::cmp::Ordering;

use crate::error;

use super::cast_binary::cast_binary_compare;
use super::datetime::OrdWithDefaultOffset;
use super::{Atomic, AtomicCompare, AtomicCompareValue, BinaryType};

pub(crate) struct OpGe;

impl AtomicCompare for OpGe {
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

        match (a, b) {
            (Decimal(a), Decimal(b)) => Ok(a >= b),
            (Integer(_, a), Integer(_, b)) => Ok(a >= b),
            (Float(a), Float(b)) => Ok(a >= b),
            (Double(a), Double(b)) => Ok(a >= b),
            (Boolean(a), Boolean(b)) => Ok(a >= b),
            (String(_, a), String(_, b)) => Ok(string_compare(a.as_ref(), b.as_ref()).is_ge()),
            (Date(a), Date(b)) => Ok(a
                .as_ref()
                .cmp_with_default_offset(b.as_ref(), default_offset)
                .is_ge()),
            (Time(a), Time(b)) => Ok(a
                .as_ref()
                .cmp_with_default_offset(b.as_ref(), default_offset)
                .is_ge()),
            (DateTime(a), DateTime(b)) => Ok(a
                .as_ref()
                .cmp_with_default_offset(b.as_ref(), default_offset)
                .is_ge()),
            (DateTimeStamp(a), DateTimeStamp(b)) => Ok(a >= b),
            (YearMonthDuration(a), YearMonthDuration(b)) => Ok(a >= b),
            (DayTimeDuration(a), DayTimeDuration(b)) => Ok(a >= b),
            (Binary(BinaryType::Hex, a), Binary(BinaryType::Hex, b)) => Ok(a >= b),
            (Binary(BinaryType::Base64, a), Binary(BinaryType::Base64, b)) => Ok(a >= b),
            _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
        }
    }

    fn arguments_inverted() -> impl AtomicCompare {
        super::OpLe
    }

    fn value() -> AtomicCompareValue {
        AtomicCompareValue::Ge
    }
}
