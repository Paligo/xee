use chrono::Offset;
use ibig::IBig;
use iri_string::types::{IriReferenceStr, IriReferenceString, IriString};
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;
use xee_xpath_ast::ast::Name;

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

/// An atomic value.
///
/// These are designated with a `xs:` namespace prefix and are described by the
/// [XPath data model](https://www.w3.org/TR/xpath-datamodel-31/#xs-types).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Atomic {
    /// xs:untypedAtomic
    Untyped(Rc<str>),
    /// a string type such as xs:string, xs:token, etc
    String(StringType, Rc<str>),
    /// xs:float
    ///
    /// This is an [`ordered_float::OrderedFloat`]
    Float(OrderedFloat<f32>),
    /// xs:double
    ///
    /// This is an [`ordered_float::OrderedFloat`]
    Double(OrderedFloat<f64>),
    /// xs:decimal
    ///
    /// This is a [`rust_decimal::Decimal`]
    Decimal(Rc<Decimal>),
    /// xs integer types (xs:integer, xs:long, xs:int, etc)
    ///
    /// This is an [`ibig::IBig`]
    Integer(IntegerType, Rc<IBig>),
    /// xs:duration
    Duration(Rc<Duration>),
    /// xs:yearMonthDuration
    YearMonthDuration(YearMonthDuration),
    /// xs:dayTimeDuration
    ///
    /// This is a [`chrono::Duration`]
    DayTimeDuration(Rc<chrono::Duration>),
    /// xs:dateTime
    DateTime(Rc<NaiveDateTimeWithOffset>),
    /// xs:dateTimeStamp
    ///
    /// This is a [`chrono::DateTime`] with a fixed offset
    DateTimeStamp(Rc<chrono::DateTime<chrono::FixedOffset>>),
    /// xs:time
    Time(Rc<NaiveTimeWithOffset>),
    /// xs:date
    Date(Rc<NaiveDateWithOffset>),
    /// xs:gYearMonth
    GYearMonth(Rc<GYearMonth>),
    /// xs:gYear
    GYear(Rc<GYear>),
    /// xs:gMonthDay
    GMonthDay(Rc<GMonthDay>),
    /// xs:gMonth
    GDay(Rc<GDay>),
    /// xs:gDay
    GMonth(Rc<GMonth>),
    /// xs:boolean
    Boolean(bool),
    /// xs binary types (xs:hexBinary, xs:base64Binary)
    Binary(BinaryType, Rc<[u8]>),
    /// xs:QName
    QName(Rc<Name>),
}

// This takes 24 bytes to store. Atomic is the largest part of Item. We could
// try to make it smaller by using Rc<String> and Rc<Vec<u8>> instead of
// Rc<str> and Rc<[u8]>, but that would pack it so tightly that item, which
// uses atomic, would need 24 bytes anyway (as it does already), and we'd have
// more indirections. Since we have no clear evidence that would help, we leave
// it at 24 for now.
#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(Atomic, [u8; 24]);

impl Atomic {
    /// The [effective boolean
    /// value](https://www.w3.org/TR/xpath-functions-31/#func-boolean) of an
    /// atomic value.
    ///
    /// - xs:boolean are taken as is.
    ///
    /// - xs:string is false if empty, otherwise true
    /// - xs:untypedAtomic is false if empty, otherwise true
    /// - any xs integer values are false if zero, otherwise true
    /// - xs:decimal is false if zero, otherwise true
    /// - xs:float is false if zero or NaN, otherwise true
    /// - xs:double is false if zero or NaN, otherwise true
    ///
    /// All other types are not convertible to a boolean.
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
            _ => Err(error::Error::XPTY0004),
        }
    }

    /// Get the string if this atomic value is a xs:string
    pub fn to_string(&self) -> error::Result<String> {
        Ok(self.to_str()?.to_string())
    }

    /// Get the string value of the atomic value.
    ///
    /// This is the canonical representation of the atomic value
    /// according to xs:schema.
    pub(crate) fn string_value(&self) -> String {
        self.clone().into_canonical()
    }

    /// Check whether this value is the NaN value.
    ///  
    /// Only xs:float and xs:double can be NaN.
    pub fn is_nan(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_nan(),
            Atomic::Double(d) => d.is_nan(),
            _ => false,
        }
    }

    /// Check whether this value is infinite.
    ///
    /// Only xs:float and xs:double can be infinite.
    pub fn is_infinite(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_infinite(),
            Atomic::Double(d) => d.is_infinite(),
            _ => false,
        }
    }

    /// Check whether this value is zero.
    ///
    /// Only numeric types can be zero.
    pub fn is_zero(&self) -> bool {
        match self {
            Atomic::Float(f) => f.is_zero(),
            Atomic::Double(d) => d.is_zero(),
            Atomic::Decimal(d) => d.is_zero(),
            Atomic::Integer(_, i) => i.is_zero(),
            _ => false,
        }
    }

    /// Check whether this is a numeric value.
    ///
    /// That is, xs:float, xs:double, xs:decimal, xs:integer and any
    /// types derived from xs:integer such as xs:int, xs:long, etc.
    pub fn is_numeric(&self) -> bool {
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

    pub(crate) fn ensure_base_schema_type(&self, xs: Xs) -> error::Result<()> {
        if self.schema_type().derives_from(xs) {
            Ok(())
        } else {
            Err(error::Error::XPTY0004)
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
    /// Simple equal uses a comparison with the codepoint collation, and UTC as
    /// the timezone.
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
        equal.unwrap_or_default()
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
            return Err(error::Error::XPTY0004);
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
        Atomic::String(StringType::String, s.into())
    }
}

impl From<&str> for Atomic {
    fn from(s: &str) -> Self {
        Atomic::String(StringType::String, s.into())
    }
}

impl From<&String> for Atomic {
    fn from(s: &String) -> Self {
        Atomic::String(StringType::String, s.clone().into())
    }
}

impl TryFrom<Atomic> for String {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::String(_, s) => Ok(s.to_string()),
            _ => Err(error::Error::XPTY0004),
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
            _ => Err(error::Error::XPTY0004),
        }
    }
}

// decimal

impl From<Decimal> for Atomic {
    fn from(d: Decimal) -> Self {
        Atomic::Decimal(d.into())
    }
}

impl TryFrom<Atomic> for Decimal {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Decimal(d) => Ok(*d.as_ref()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

// URL

impl From<IriString> for Atomic {
    fn from(u: IriString) -> Self {
        Atomic::String(StringType::AnyURI, u.to_string().into())
    }
}

impl From<IriReferenceString> for Atomic {
    fn from(u: IriReferenceString) -> Self {
        Atomic::String(StringType::AnyURI, u.to_string().into())
    }
}

impl From<&IriReferenceStr> for Atomic {
    fn from(u: &IriReferenceStr) -> Self {
        Atomic::String(StringType::AnyURI, u.to_string().into())
    }
}

impl TryFrom<Atomic> for IriReferenceString {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, error::Error> {
        match a {
            Atomic::String(_, s) => {
                Ok(s.as_ref().try_into().map_err(|_| error::Error::FORG0002)?)
            }
            _ => Err(error::Error::XPTY0004),
        }
    }
}

// integers

impl From<IBig> for Atomic {
    fn from(i: IBig) -> Self {
        Atomic::Integer(IntegerType::Integer, i.into())
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
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl TryFrom<Atomic> for IBig {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(_, i) => Ok(i.as_ref().clone()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<i64> for Atomic {
    fn from(i: i64) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::Long, i.into())
    }
}

impl TryFrom<Atomic> for i64 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Long, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<i32> for Atomic {
    fn from(i: i32) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::Int, i.into())
    }
}

impl TryFrom<Atomic> for i32 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Int, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<i16> for Atomic {
    fn from(i: i16) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::Short, i.into())
    }
}

impl TryFrom<Atomic> for i16 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Short, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<i8> for Atomic {
    fn from(i: i8) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::Byte, i.into())
    }
}

impl TryFrom<Atomic> for i8 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::Byte, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<u64> for Atomic {
    fn from(i: u64) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::UnsignedLong, i.into())
    }
}

impl TryFrom<Atomic> for u64 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedLong, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<u32> for Atomic {
    fn from(i: u32) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::UnsignedInt, i.into())
    }
}

impl TryFrom<Atomic> for u32 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedInt, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<u16> for Atomic {
    fn from(i: u16) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::UnsignedShort, i.into())
    }
}

impl TryFrom<Atomic> for u16 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedShort, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<u8> for Atomic {
    fn from(i: u8) -> Self {
        let i: IBig = i.into();
        Atomic::Integer(IntegerType::UnsignedByte, i.into())
    }
}

impl TryFrom<Atomic> for u8 {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Integer(IntegerType::UnsignedByte, i) => Ok(i.as_ref().clone().try_into()?),
            _ => Err(error::Error::XPTY0004),
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
            _ => Err(error::Error::XPTY0004),
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
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl From<Name> for Atomic {
    fn from(n: Name) -> Self {
        Atomic::QName(n.into())
    }
}

impl TryFrom<Atomic> for Name {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::QName(n) => Ok(n.as_ref().clone()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

// binary

impl TryFrom<Atomic> for Rc<[u8]> {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Binary(_, n) => Ok(n),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl<'a> TryFrom<&'a Atomic> for Rc<[u8]> {
    type Error = error::Error;

    fn try_from(a: &'a Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Binary(_, n) => Ok(Rc::clone(n)),
            _ => Err(error::Error::XPTY0004),
        }
    }
}
