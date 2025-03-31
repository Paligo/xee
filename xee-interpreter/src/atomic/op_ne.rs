use std::cmp::Ordering;

use crate::error;

use super::op_eq::OpEq;
use super::{Atomic, AtomicCompare, AtomicCompareValue};

pub(crate) struct OpNe;

impl AtomicCompare for OpNe {
    fn atomic_compare<F>(
        a: Atomic,
        b: Atomic,
        string_compare: F,
        default_offset: chrono::FixedOffset,
    ) -> error::Result<bool>
    where
        F: Fn(&str, &str) -> Ordering,
    {
        OpEq::atomic_compare(a, b, string_compare, default_offset).map(|eq| !eq)
    }

    fn arguments_inverted() -> impl AtomicCompare {
        Self
    }

    fn value() -> AtomicCompareValue {
        AtomicCompareValue::Ne
    }
}
