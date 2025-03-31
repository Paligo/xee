use std::rc::Rc;

use chrono::Offset;
use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::Decimal;

use xee_name::Name;

use crate::error;

use super::{
    Atomic, BinaryType, Duration, GDay, GMonth, GMonthDay, GYear, GYearMonth, ToDateTimeStamp,
};

// A map key is constructed according to the rules in
// https://www.w3.org/TR/xpath-functions-31/#func-same-key
// We can use the MapKey as a key in a HashMap so we can implement
// XPath Map

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapKey {
    String(Rc<String>),
    PositiveInfinity,
    NegativeInfinity,
    NaN,
    Integer(Rc<IBig>),
    Decimal(Rc<Decimal>),
    Duration(Rc<Duration>),
    // datetime with timezone don't hash the same, so we convert
    // into a naive datetime
    Date(chrono::NaiveDateTime),
    NaiveDate(chrono::NaiveDate),
    Time(chrono::NaiveDateTime),
    NaiveTime(chrono::NaiveTime),
    DateTime(chrono::NaiveDateTime),
    NaiveDateTime(chrono::NaiveDateTime),
    GYear(Rc<GYear>),
    GYearMonth(Rc<GYearMonth>),
    GMonth(Rc<GMonth>),
    GMonthDay(Rc<GMonthDay>),
    GDay(Rc<GDay>),
    Boolean(bool),
    Binary(BinaryType, Rc<Vec<u8>>),
    QName(Rc<Name>),
}

#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(MapKey, [u8; 16]);

impl MapKey {
    pub(crate) fn new(atomic: Atomic) -> error::Result<Self> {
        match &atomic {
            // string types (including AnyURI) and untyped are stored as the same key
            Atomic::String(_, s) | Atomic::Untyped(s) => Ok(Self::String(s.to_string().into())),
            // floats and doubles are have special handling for NaN and infinity.
            // Otherwise they are stored as decimals
            Atomic::Float(OrderedFloat(f)) => {
                if f.is_nan() {
                    Ok(Self::NaN)
                } else if f.is_infinite() {
                    if f.is_sign_positive() {
                        Ok(Self::PositiveInfinity)
                    } else {
                        Ok(Self::NegativeInfinity)
                    }
                } else {
                    Self::new(atomic.cast_to_decimal()?)
                }
            }
            Atomic::Double(OrderedFloat(f)) => {
                if f.is_nan() {
                    Ok(Self::NaN)
                } else if f.is_infinite() {
                    if f.is_sign_positive() {
                        Ok(Self::PositiveInfinity)
                    } else {
                        Ok(Self::NegativeInfinity)
                    }
                } else {
                    Self::new(atomic.cast_to_decimal()?)
                }
            }
            Atomic::Decimal(d) => {
                // we ensure that any decimals that can be stored
                // as an integer are stored that way, so they have
                // the same hash
                if d.is_integer() {
                    Self::new(atomic.cast_to_integer()?)
                } else {
                    Ok(Self::Decimal(d.clone()))
                }
            }
            Atomic::Integer(_, i) => Ok(Self::Integer(i.clone())),

            // All types of duration as stored the same way, so they
            // can have the same key
            Atomic::Duration(d) => Ok(Self::Duration(d.clone())),
            Atomic::YearMonthDuration(d) => Ok(Self::Duration(
                Duration::from_year_month(d.clone()).into(),
            )),
            Atomic::DayTimeDuration(d) => Ok(Self::Duration(
                Duration::from_day_time(*d.as_ref()).into(),
            )),
            // date times with a timezone are stored as a chrono datetime,
            // or they are stored as a naive datetime
            Atomic::DateTime(d) => {
                if d.offset.is_some() {
                    Ok(Self::DateTime(
                        d.to_naive_date_time(chrono::offset::Utc.fix()),
                    ))
                } else {
                    Ok(Self::NaiveDateTime(d.date_time))
                }
            }
            Atomic::DateTimeStamp(d) => Ok(Self::DateTime(d.naive_local())),
            // times and dates with a timezone are stored as a chrono
            // datetime (but separately), or they are stored as a naive
            // time or date
            Atomic::Time(t) => {
                if t.offset.is_some() {
                    Ok(Self::Time(
                        t.to_naive_date_time(chrono::offset::Utc.fix()),
                    ))
                } else {
                    Ok(Self::NaiveTime(t.time))
                }
            }
            Atomic::Date(d) => {
                if d.offset.is_some() {
                    Ok(Self::Date(
                        d.to_naive_date_time(chrono::offset::Utc.fix()),
                    ))
                } else {
                    Ok(Self::NaiveDate(d.date))
                }
            }
            // gregorian objects have hashes that are already okay
            Atomic::GYearMonth(g) => Ok(Self::GYearMonth(g.clone())),
            Atomic::GYear(g) => Ok(Self::GYear(g.clone())),
            Atomic::GMonthDay(g) => Ok(Self::GMonthDay(g.clone())),
            Atomic::GDay(g) => Ok(Self::GDay(g.clone())),
            Atomic::GMonth(g) => Ok(Self::GMonth(g.clone())),
            // booleans are stored as themselves
            Atomic::Boolean(b) => Ok(Self::Boolean(*b)),
            // binary types are stored as themselves
            Atomic::Binary(t, b) => Ok(Self::Binary(*t, b.to_vec().into())),
            // qnames are stored as themselves
            Atomic::QName(q) => Ok(Self::QName(q.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::atomic::NaiveDateTimeWithOffset;

    use super::*;

    use ibig::ibig;
    use rust_decimal_macros::*;

    #[test]
    fn test_float_and_decimal() {
        let a: Atomic = dec!(1.5).into();
        let b: Atomic = (1.5f32).into();
        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_float_and_decimal_that_are_integers() {
        let a: Atomic = dec!(1.0).into();
        let b: Atomic = (1.0f32).into();
        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_float_and_integer() {
        let a: Atomic = dec!(1.0).into();
        let b: Atomic = ibig!(1).into();
        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_decimal_and_integer() {
        let a: Atomic = dec!(1.0).into();
        let b: Atomic = ibig!(1).into();
        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_integer_and_bool() {
        let a: Atomic = ibig!(1).into();
        let b: Atomic = true.into();
        assert_ne!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_string_and_untyped() {
        let a: Atomic = "foo".into();
        let b: Atomic = Atomic::Untyped("foo".into());
        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_datetimes_with_timezones() {
        let a_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(1, 2, 3, 456)
                .unwrap(),
            Some(chrono::offset::Utc.fix()),
        );
        // put it at the same time, but in timezone one hour ahead
        let b_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(2, 2, 3, 456)
                .unwrap(),
            Some(chrono::FixedOffset::east_opt(60 * 60).unwrap()),
        );

        let a: Atomic = Atomic::DateTime(a_date_time.into());
        let b: Atomic = Atomic::DateTime(b_date_time.into());

        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_datetimes_without_timezones() {
        let a_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(3, 2, 3, 456)
                .unwrap(),
            None,
        );
        let b_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(3, 2, 3, 456)
                .unwrap(),
            None,
        );

        let a: Atomic = Atomic::DateTime(a_date_time.into());
        let b: Atomic = Atomic::DateTime(b_date_time.into());

        assert_eq!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }

    #[test]
    fn test_datetimes_with_and_without_timezones() {
        let a_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(3, 2, 3, 456)
                .unwrap(),
            Some(chrono::offset::Utc.fix()),
        );
        let b_date_time = NaiveDateTimeWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                .unwrap()
                .and_hms_milli_opt(3, 2, 3, 456)
                .unwrap(),
            None,
        );

        let a: Atomic = Atomic::DateTime(a_date_time.into());
        let b: Atomic = Atomic::DateTime(b_date_time.into());

        assert_ne!(MapKey::new(a).unwrap(), MapKey::new(b).unwrap());
    }
}
