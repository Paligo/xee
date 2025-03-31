use chrono::{Offset, TimeZone};

use crate::{atomic::Atomic, error};

pub(crate) trait EqWithDefaultOffset: ToDateTimeStamp {
    fn eq_with_default_offset(&self, other: &Self, default_offset: chrono::FixedOffset) -> bool {
        let self_date_time_stamp = self.to_date_time_stamp(default_offset);
        let other_date_time_stamp = other.to_date_time_stamp(default_offset);
        self_date_time_stamp == other_date_time_stamp
    }
}

pub(crate) trait OrdWithDefaultOffset: ToDateTimeStamp {
    fn cmp_with_default_offset(
        &self,
        other: &Self,
        default_offset: chrono::FixedOffset,
    ) -> std::cmp::Ordering {
        let self_date_time_stamp = self.to_date_time_stamp(default_offset);
        let other_date_time_stamp = other.to_date_time_stamp(default_offset);
        self_date_time_stamp.cmp(&other_date_time_stamp)
    }
}

pub(crate) trait ToDateTimeStamp {
    fn to_date_time_stamp(
        &self,
        default_offset: chrono::FixedOffset,
    ) -> chrono::DateTime<chrono::FixedOffset>;

    fn to_naive_date_time(&self, default_offset: chrono::FixedOffset) -> chrono::NaiveDateTime {
        self.to_date_time_stamp(default_offset).naive_utc()
    }
}

impl<T> EqWithDefaultOffset for T where T: ToDateTimeStamp {}
impl<T> OrdWithDefaultOffset for T where T: ToDateTimeStamp {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonthDuration {
    pub(crate) months: i64,
}

impl YearMonthDuration {
    pub(crate) fn new(months: i64) -> Self {
        Self { months }
    }

    pub(crate) fn years(&self) -> i64 {
        self.months / 12
    }

    pub(crate) fn months(&self) -> i64 {
        self.months % 12
    }
}

impl From<YearMonthDuration> for Atomic {
    fn from(year_month_duration: YearMonthDuration) -> Self {
        Self::YearMonthDuration(year_month_duration)
    }
}

/// A Duration is a combination of a [`YearMonthDuration`]` and
/// [`chrono::Duration`].
///
/// It represents `xs:duration`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Duration {
    pub(crate) year_month: YearMonthDuration,
    pub(crate) day_time: chrono::Duration,
}

impl Duration {
    pub(crate) fn new(months: i64, day_time: chrono::Duration) -> Self {
        Self {
            year_month: YearMonthDuration { months },
            day_time,
        }
    }

    pub(crate) fn from_year_month(year_month_duration: YearMonthDuration) -> Self {
        Self {
            year_month: year_month_duration,
            day_time: chrono::Duration::zero(),
        }
    }

    pub(crate) fn from_day_time(duration: chrono::Duration) -> Self {
        Self {
            year_month: YearMonthDuration { months: 0 },
            day_time: duration,
        }
    }
}

impl From<Duration> for Atomic {
    fn from(duration: Duration) -> Self {
        Self::Duration(duration.into())
    }
}

impl TryFrom<Atomic> for Duration {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Duration(d) => Ok(d.as_ref().clone()),
            Atomic::YearMonthDuration(d) => Ok(Self::from_year_month(d)),
            Atomic::DayTimeDuration(d) => Ok(Self::from_day_time(*d)),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl TryFrom<Atomic> for chrono::Duration {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::DayTimeDuration(d) => Ok(*d.as_ref()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

/// A `NaiveDateTimeWithOffset` is a combination of a [`chrono::NaiveDateTime`] and
/// an optional [`chrono::FixedOffset`].
///
/// It represents `xs:dateTime`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NaiveDateTimeWithOffset {
    pub(crate) date_time: chrono::NaiveDateTime,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl From<NaiveDateTimeWithOffset> for chrono::DateTime<chrono::FixedOffset> {
    fn from(naive_date_time_with_offset: NaiveDateTimeWithOffset) -> Self {
        let offset = naive_date_time_with_offset
            .offset
            .unwrap_or_else(|| chrono::offset::Utc.fix());
        Self::from_naive_utc_and_offset(naive_date_time_with_offset.date_time, offset)
    }
}

impl From<chrono::DateTime<chrono::FixedOffset>> for NaiveDateTimeWithOffset {
    fn from(date_time: chrono::DateTime<chrono::FixedOffset>) -> Self {
        Self::new(date_time.naive_local(), Some(*date_time.offset()))
    }
}

impl From<NaiveDateTimeWithOffset> for Atomic {
    fn from(date_time: NaiveDateTimeWithOffset) -> Self {
        Self::DateTime(date_time.into())
    }
}

impl TryFrom<Atomic> for NaiveDateTimeWithOffset {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::DateTime(d) => Ok(d.as_ref().clone()),
            Atomic::DateTimeStamp(d) => Ok((*d.as_ref()).into()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl ToDateTimeStamp for NaiveDateTimeWithOffset {
    fn to_date_time_stamp(
        &self,
        default_offset: chrono::FixedOffset,
    ) -> chrono::DateTime<chrono::FixedOffset> {
        let offset = self.offset.unwrap_or(default_offset);
        offset.from_local_datetime(&self.date_time).unwrap()
    }
}

impl NaiveDateTimeWithOffset {
    pub(crate) fn new(
        date_time: chrono::NaiveDateTime,
        offset: Option<chrono::FixedOffset>,
    ) -> Self {
        Self { date_time, offset }
    }
}

/// A `NaiveTimeWithOffset` is a combination of a [`chrono::NaiveTime`] and
/// an optional [`chrono::FixedOffset`].
///
/// It represents `xs:time`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NaiveTimeWithOffset {
    pub(crate) time: chrono::NaiveTime,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl TryFrom<Atomic> for NaiveTimeWithOffset {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Time(d) => Ok(d.as_ref().clone()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl NaiveTimeWithOffset {
    pub(crate) fn new(time: chrono::NaiveTime, offset: Option<chrono::FixedOffset>) -> Self {
        Self { time, offset }
    }
}

impl ToDateTimeStamp for NaiveTimeWithOffset {
    fn to_date_time_stamp(
        &self,
        default_offset: chrono::FixedOffset,
    ) -> chrono::DateTime<chrono::FixedOffset> {
        let offset = self.offset.unwrap_or(default_offset);
        // https://www.w3.org/TR/xpath-functions-31/#func-subtract-times
        let date_time = chrono::NaiveDate::from_ymd_opt(1972, 12, 31)
            .unwrap()
            .and_time(self.time);
        offset.from_local_datetime(&date_time).unwrap()
    }
}

impl From<NaiveTimeWithOffset> for Atomic {
    fn from(time: NaiveTimeWithOffset) -> Self {
        Self::Time(time.into())
    }
}

/// A `NaiveDateWithOffset` is a combination of a [`chrono::NaiveDate`] and
/// an optional [`chrono::FixedOffset`].
///
/// It represents `xs:date`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NaiveDateWithOffset {
    pub(crate) date: chrono::NaiveDate,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl TryFrom<Atomic> for NaiveDateWithOffset {
    type Error = error::Error;

    fn try_from(a: Atomic) -> Result<Self, Self::Error> {
        match a {
            Atomic::Date(d) => Ok(d.as_ref().clone()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl NaiveDateWithOffset {
    pub(crate) fn new(date: chrono::NaiveDate, offset: Option<chrono::FixedOffset>) -> Self {
        Self { date, offset }
    }
}

impl ToDateTimeStamp for NaiveDateWithOffset {
    fn to_date_time_stamp(
        &self,
        default_offset: chrono::FixedOffset,
    ) -> chrono::DateTime<chrono::FixedOffset> {
        let offset = self.offset.unwrap_or(default_offset);
        let date_time = self.date.and_hms_opt(0, 0, 0).unwrap();
        offset.from_local_datetime(&date_time).unwrap()
    }
}

impl From<NaiveDateWithOffset> for Atomic {
    fn from(date: NaiveDateWithOffset) -> Self {
        Self::Date(date.into())
    }
}

/// A `GYearMonth` is a combination of a year and a month, and an optional
/// [`chrono::FixedOffset`].
///
/// It represents `xs:gYearMonth`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GYearMonth {
    pub(crate) year: i32,
    pub(crate) month: u32,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl GYearMonth {
    pub(crate) fn new(year: i32, month: u32, offset: Option<chrono::FixedOffset>) -> Self {
        Self {
            year,
            month,
            offset,
        }
    }
}

impl From<GYearMonth> for Atomic {
    fn from(g_year_month: GYearMonth) -> Self {
        Self::GYearMonth(g_year_month.into())
    }
}

/// A `GYear` is a combination of a year and an optional
/// [`chrono::FixedOffset`].
///
/// It represents `xs:gYear`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GYear {
    pub(crate) year: i32,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl GYear {
    pub(crate) fn new(year: i32, offset: Option<chrono::FixedOffset>) -> Self {
        Self { year, offset }
    }
}

impl From<GYear> for Atomic {
    fn from(g_year: GYear) -> Self {
        Self::GYear(g_year.into())
    }
}

/// A `GMonthDay` is a combination of a month and a day, and an optional
/// [`chrono::FixedOffset`].
///
/// It represents `xs:gMonthDay`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GMonthDay {
    pub(crate) month: u32,
    pub(crate) day: u32,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl GMonthDay {
    pub(crate) fn new(month: u32, day: u32, offset: Option<chrono::FixedOffset>) -> Self {
        Self { month, day, offset }
    }
}

impl From<GMonthDay> for Atomic {
    fn from(g_month_day: GMonthDay) -> Self {
        Self::GMonthDay(g_month_day.into())
    }
}

/// A `GDay` is a combination of a day and an optional
/// [`chrono::FixedOffset`].
///
/// It represents `xs:gDay`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GDay {
    pub(crate) day: u32,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl GDay {
    pub(crate) fn new(day: u32, offset: Option<chrono::FixedOffset>) -> Self {
        Self { day, offset }
    }
}

impl From<GDay> for Atomic {
    fn from(g_day: GDay) -> Self {
        Self::GDay(g_day.into())
    }
}

/// A `GMonth` is a combination of a month and an optional
/// [`chrono::FixedOffset`].
///
/// It represents `xs:gMonth`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GMonth {
    pub(crate) month: u32,
    pub(crate) offset: Option<chrono::FixedOffset>,
}

impl GMonth {
    pub(crate) fn new(month: u32, offset: Option<chrono::FixedOffset>) -> Self {
        Self { month, offset }
    }
}

impl From<GMonth> for Atomic {
    fn from(g_month: GMonth) -> Self {
        Self::GMonth(g_month.into())
    }
}

impl From<chrono::Duration> for Atomic {
    fn from(duration: chrono::Duration) -> Self {
        Self::DayTimeDuration(duration.into())
    }
}

impl From<chrono::DateTime<chrono::FixedOffset>> for Atomic {
    fn from(date_time: chrono::DateTime<chrono::FixedOffset>) -> Self {
        Self::DateTimeStamp(date_time.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::atomic::{AtomicCompare, OpGt};

    use super::*;

    #[test]
    fn test_compare_dates() {
        let a_date = NaiveDateWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2004, 12, 25).unwrap(),
            Some(chrono::offset::Utc.fix()),
        );
        let b_date = NaiveDateWithOffset::new(
            chrono::NaiveDate::from_ymd_opt(2004, 12, 25).unwrap(),
            Some(chrono::FixedOffset::east_opt(60 * 60 * 7).unwrap()),
        );

        let a: Atomic = Atomic::Date(a_date.into());
        let b: Atomic = Atomic::Date(b_date.into());

        assert!(
            OpGt::atomic_compare(a.clone(), b.clone(), str::cmp, chrono::offset::Utc.fix())
                .unwrap()
        );
    }
}
