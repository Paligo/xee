use chrono::Offset;
use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;
use xee_xpath_ast::ast;

use xee_schema_type::Xs;

use crate::atomic::types::{BinaryType, IntegerType, StringType};
use crate::error;
use crate::string::Collation;

use super::datetime::{
    Duration, GDay, GMonth, GMonthDay, GYear, GYearMonth, NaiveDateTimeWithOffset,
    NaiveDateWithOffset, NaiveTimeWithOffset, YearMonthDuration,
};
use super::{op_unary, OpEq};
use super::{AtomicCompare, OpGt};

// We try to maintain this struct as size 16 as it's cloned a lot during normal
// operation. Anything bigger we stuff in an Rc

// https://www.w3.org/TR/xpath-datamodel-31/#xs-types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atomic {
    Untyped(Rc<String>),
    String(StringType, Rc<String>),
    Float(OrderedFloat<f32>),
    Double(OrderedFloat<f64>),
    Decimal(Rc<Decimal>),
    Integer(IntegerType, Rc<IBig>),
    Duration(Rc<Duration>),
    YearMonthDuration(YearMonthDuration),
    DayTimeDuration(Rc<chrono::Duration>),
    DateTime(Rc<NaiveDateTimeWithOffset>),
    DateTimeStamp(Rc<chrono::DateTime<chrono::FixedOffset>>),
    Time(Rc<NaiveTimeWithOffset>),
    Date(Rc<NaiveDateWithOffset>),
    GYearMonth(Rc<GYearMonth>),
    GYear(Rc<GYear>),
    GMonthDay(Rc<GMonthDay>),
    GDay(Rc<GDay>),
    GMonth(Rc<GMonth>),
    Boolean(bool),
    Binary(BinaryType, Rc<Vec<u8>>),
    QName(Rc<ast::Name>),
}

impl Atomic {
    pub(crate) fn effective_boolean_value(&self) -> error::Result<bool> {
        match self {
            Atomic::Boolean(b) => Ok(*b),
            // https://www.w3.org/TR/xpath-31/#id-ebv
            // point 4
            Atomic::String(_, s) => Ok(!s.is_empty()),
            Atomic::Untyped(s) => Ok(!s.is_empty()),
            // point 5
            Atomic::Integer(_, i) => Ok(!i.is_zero()),
            Atomic::Decimal(d) => Ok(!d.is_zero()),
            // NaN also counts as false
            Atomic::Float(f) => Ok(!f.is_zero() && !f.is_nan()),
            Atomic::Double(d) => Ok(!d.is_zero() && !d.is_nan()),
            // point 6
            _ => Err(error::Error::FORG0006),
        }
    }

    // XXX is this named right? It's consistent with  to_double, to_bool, etc,
    // but inconsistent with the to_string Rust convention
    pub(crate) fn to_str(&self) -> error::Result<&str> {
        match self {
            Atomic::String(_, s) => Ok(s),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn to_string(&self) -> error::Result<String> {
        Ok(self.to_str()?.to_string())
    }

    pub(crate) fn string_value(&self) -> error::Result<String> {
        Ok(self.clone().into_canonical())
    }

    pub(crate) fn is_nan(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_nan(),
            Atomic::Double(d) => d.is_nan(),
            _ => false,
        }
    }

    pub(crate) fn is_infinite(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_infinite(),
            Atomic::Double(d) => d.is_infinite(),
            _ => false,
        }
    }

    pub(crate) fn is_zero(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_zero(),
            Atomic::Double(d) => d.is_zero(),
            Atomic::Decimal(d) => d.is_zero(),
            Atomic::Integer(_, i) => i.is_zero(),
            _ => false,
        }
    }

    pub(crate) fn is_numeric(&self) -> bool {
        matches!(
            self,
            Atomic::Float(_) | Atomic::Double(_) | Atomic::Decimal(_) | Atomic::Integer(_, _)
        )
    }

    pub(crate) fn is_addable(&self) -> bool {
        matches!(
            self,
            Atomic::Float(_)
                | Atomic::Double(_)
                | Atomic::Decimal(_)
                | Atomic::Integer(_, _)
                | Atomic::DayTimeDuration(_)
                | Atomic::YearMonthDuration(_)
        )
    }

    pub(crate) fn is_comparable(&self) -> bool {
        matches!(
            self,
            Atomic::String(_, _)
                | Atomic::Float(_)
                | Atomic::Double(_)
                | Atomic::Decimal(_)
                | Atomic::Integer(_, _)
                | Atomic::YearMonthDuration(_)
                | Atomic::DayTimeDuration(_)
                | Atomic::DateTime(_)
                | Atomic::DateTimeStamp(_)
                | Atomic::Time(_)
                | Atomic::Date(_)
                | Atomic::Boolean(_)
                | Atomic::Binary(_, _)
        )
    }

    pub(crate) fn is_true(&self) -> bool {
        if let Atomic::Boolean(b) = self {
            *b
        } else {
            false
        }
    }

    pub(crate) fn is_untyped(&self) -> bool {
        matches!(self, Atomic::Untyped(_))
    }

    pub(crate) fn schema_type(&self) -> Xs {
        match self {
            Atomic::String(string_type, _) => string_type.schema_type(),
            Atomic::Untyped(_) => Xs::UntypedAtomic,
            Atomic::Boolean(_) => Xs::Boolean,
            Atomic::Decimal(_) => Xs::Decimal,
            Atomic::Integer(integer_type, _) => integer_type.schema_type(),
            Atomic::Float(_) => Xs::Float,
            Atomic::Double(_) => Xs::Double,
            Atomic::QName(_) => Xs::QName,
            Atomic::Binary(binary_type, _) => binary_type.schema_type(),
            Atomic::Duration(_) => Xs::Duration,
            Atomic::YearMonthDuration(_) => Xs::YearMonthDuration,
            Atomic::DayTimeDuration(_) => Xs::DayTimeDuration,
            Atomic::Time(_) => Xs::Time,
            Atomic::Date(_) => Xs::Date,
            Atomic::DateTime(_) => Xs::DateTime,
            Atomic::DateTimeStamp(_) => Xs::DateTimeStamp,
            Atomic::GYearMonth(_) => Xs::GYearMonth,
            Atomic::GYear(_) => Xs::GYear,
            Atomic::GMonthDay(_) => Xs::GMonthDay,
            Atomic::GMonth(_) => Xs::GMonth,
            Atomic::GDay(_) => Xs::GDay,
        }
    }

    pub(crate) fn type_name(&self) -> ast::Name {
        let schema_type = self.schema_type();
        ast::Name::new(
            schema_type.local_name().to_string(),
            Some(Xs::namespace().to_string()),
            Some("xs".to_string()),
        )
    }

    pub(crate) fn ensure_base_schema_type(&self, xs: Xs) -> error::Result<()> {
        if self.schema_type().derives_from(xs) {
            Ok(())
        } else {
            Err(error::Error::Type)
        }
    }

    pub(crate) fn derives_from(&self, other: &Atomic) -> bool {
        self.schema_type().derives_from(other.schema_type())
    }

    pub(crate) fn has_same_schema_type(&self, other: &Atomic) -> bool {
        self.schema_type() == other.schema_type()
    }

    pub(crate) fn plus(self) -> error::Result<Atomic> {
        op_unary::unary_plus(self)
    }

    pub(crate) fn minus(self) -> error::Result<Atomic> {
        op_unary::unary_minus(self)
    }

    /// Compare atoms using XPath rules.
    ///
    /// This means for instance that an integer can compare the same as a
    /// decimal. This is different from the Eq implemented for the atom itself,
    /// which compares the actual data, and different types are always distinct
    /// in that case.
    ///
    /// simple equal uses a comparison with the codepoint collation UTC as the
    /// timezone.
    pub fn simple_equal(&self, other: &Atomic) -> bool {
        self.equal(other, &Collation::CodePoint, chrono::offset::Utc.fix())
    }

    /// Compare atoms using XPath rules, with explicit collation and offset.
    pub fn equal(
        &self,
        other: &Atomic,
        collation: &Collation,
        default_offset: chrono::FixedOffset,
    ) -> bool {
        // TODO: clone is annoying
        let equal = OpEq::atomic_compare(
            self.clone(),
            other.clone(),
            |a, b| collation.compare(a, b),
            default_offset,
        );
        if let Ok(equal) = equal {
            equal
        } else {
            false
        }
    }

    /// Deep-equal comparison.
    ///
    /// This is like equal, but NaN compare equal as well
    pub(crate) fn deep_equal(
        &self,
        other: &Atomic,
        collation: &Collation,
        default_offset: chrono::FixedOffset,
    ) -> bool {
        if self.is_nan() && other.is_nan() {
            return true;
        }
        self.equal(other, collation, default_offset)
    }

    pub(crate) fn fallible_compare(
        &self,
        other: &Atomic,
        collation: &Collation,
        default_offset: chrono::FixedOffset,
    ) -> error::Result<Ordering> {
        if !self.is_comparable() || !other.is_comparable() {
            return Err(error::Error::Type);
        }
        let is_equal = OpEq::atomic_compare(
            self.clone(),
            other.clone(),
            |a, b| collation.compare(a, b),
            default_offset,
        )?;

        if is_equal {
            Ok(Ordering::Equal)
        } else {
            let is_greater = OpGt::atomic_compare(
                self.clone(),
                other.clone(),
                |a, b| collation.compare(a, b),
                default_offset,
            )?;
            if is_greater {
                Ok(Ordering::Greater)
            } else {
                Ok(Ordering::Less)
            }
        }
    }

    /// This function is intended to be used by sort_by_key
    /// Since comparison is fallible, we sort all error cases as
    /// less than all non-error cases, and then we detect them later.
    /// This requires an additional pass to determine that for each pair a, b
    /// comparison doesn't fail.
    pub(crate) fn compare(
        &self,
        other: &Atomic,
        collation: &Collation,
        default_offset: chrono::FixedOffset,
    ) -> Ordering {
        self.fallible_compare(other, collation, default_offset)
            .unwrap_or(Ordering::Less)
    }
}

impl fmt::Display for Atomic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} {}",
            self.schema_type(),
            self.clone().into_canonical()
        )
    }
}

// strings

impl From<String> for Atomic {
    fn from(s: String) -> Self {
        Atomic::String(StringType::String, Rc::new(s))
    }
}

impl From<&str> for Atomic {
    fn from(s: &str) -> Self {
        Atomic::String(StringType::String, Rc::new(s.to_string()))
    }
}

impl From<&String> for Atomic {
    fn from(s: &String) -> Self {
        Atomic::String(StringType::String, Rc::new(s.clone()))
    }
}

impl TryFrom<Atomic> for String {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::String(_, s) => Ok(s.as_ref().clone()),
            _ => Err(error::Error::Type),
        }
    }
}

// bool

impl From<bool> for Atomic {
    fn from(b: bool) -> Self {
        Atomic::Boolean(b)
    }
}

impl TryFrom<Atomic> for bool {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Boolean(b) => Ok(b),
            _ => Err(error::Error::Type),
        }
    }
}

// decimal

impl From<Decimal> for Atomic {
    fn from(d: Decimal) -> Self {
        Atomic::Decimal(Rc::new(d))
    }
}

impl TryFrom<Atomic> for Decimal {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Decimal(d) => Ok(*d.as_ref()),
            _ => Err(error::Error::Type),
        }
    }
}

// integers

impl From<IBig> for Atomic {
    fn from(i: IBig) -> Self {
        Atomic::Integer(IntegerType::Integer, Rc::new(i))
    }
}

impl From<Rc<IBig>> for Atomic {
    fn from(i: Rc<IBig>) -> Self {
        Atomic::Integer(IntegerType::Integer, i)
    }
}

impl TryFrom<Atomic> for Rc<IBig> {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(_, i) => Ok(i),
            _ => Err(error::Error::Type),
        }
    }
}

impl TryFrom<Atomic> for IBig {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(_, i) => Ok(i.as_ref().clone()),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<i64> for Atomic {
    fn from(i: i64) -> Self {
        Atomic::Integer(IntegerType::Long, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for i64 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Long, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<i32> for Atomic {
    fn from(i: i32) -> Self {
        Atomic::Integer(IntegerType::Int, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for i32 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Int, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<i16> for Atomic {
    fn from(i: i16) -> Self {
        Atomic::Integer(IntegerType::Short, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for i16 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Short, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<i8> for Atomic {
    fn from(i: i8) -> Self {
        Atomic::Integer(IntegerType::Byte, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for i8 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Byte, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<u64> for Atomic {
    fn from(i: u64) -> Self {
        Atomic::Integer(IntegerType::UnsignedLong, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for u64 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedLong, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<u32> for Atomic {
    fn from(i: u32) -> Self {
        Atomic::Integer(IntegerType::UnsignedInt, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for u32 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedInt, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<u16> for Atomic {
    fn from(i: u16) -> Self {
        Atomic::Integer(IntegerType::UnsignedShort, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for u16 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedShort, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

impl From<u8> for Atomic {
    fn from(i: u8) -> Self {
        Atomic::Integer(IntegerType::UnsignedByte, Rc::new(i.into()))
    }
}

impl TryFrom<Atomic> for u8 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedByte, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::Type),
        }
    }
}

// floats

impl From<f32> for Atomic {
    fn from(f: f32) -> Self {
        Atomic::Float(OrderedFloat(f))
    }
}

impl From<OrderedFloat<f32>> for Atomic {
    fn from(f: OrderedFloat<f32>) -> Self {
        Atomic::Float(f)
    }
}

impl TryFrom<Atomic> for f32 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Float(f) => Ok(f.into_inner()),
            // type promotion
            Atomic::Decimal(_) | Atomic::Integer(_, _) => {
                let f: f32 = a.cast_to_float()?.try_into()?;
                Ok(f)
            }
            _ => Err(error::Error::Type),
        }
    }
}

impl From<f64> for Atomic {
    fn from(f: f64) -> Self {
        Atomic::Double(OrderedFloat(f))
    }
}

impl From<OrderedFloat<f64>> for Atomic {
    fn from(f: OrderedFloat<f64>) -> Self {
        Atomic::Double(f)
    }
}

impl TryFrom<Atomic> for f64 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Double(f) => Ok(f.into_inner()),
            // type promotion
            Atomic::Float(f) => Ok(f.into_inner() as f64),
            Atomic::Decimal(_) | Atomic::Integer(_, _) => {
                let f: f64 = a.cast_to_double()?.try_into()?;
                Ok(f)
            }
            _ => Err(error::Error::Type),
        }
    }
}

impl From<ast::Name> for Atomic {
    fn from(n: ast::Name) -> Self {
        Atomic::QName(Rc::new(n))
    }
}

impl TryFrom<Atomic> for ast::Name {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::QName(n) => Ok(n.as_ref().clone()),
            _ => Err(error::Error::Type),
        }
    }
}
