use chrono::{Datelike, Offset, TimeZone, Timelike};
use chumsky::prelude::*;
use chumsky::util::MaybeRef;
use rust_decimal::prelude::*;
use std::cmp::Ordering;
use std::rc::Rc;

use crate::atomic;
use crate::error;

use super::cast::whitespace_collapse;
use super::datetime::{
    Duration, GDay, GMonth, GMonthDay, GYear, GYearMonth, NaiveDateTimeWithOffset,
    NaiveDateWithOffset, NaiveTimeWithOffset, YearMonthDuration,
};

pub(crate) type BoxedParser<'a, 'b, T> = Boxed<'a, 'b, &'a str, T, extra::Default>;

impl atomic::Atomic {
    pub(crate) fn canonical_duration(duration: &Duration) -> String {
        // https://www.w3.org/TR/2012/REC-xmlschema11-2-20120405/datatypes.html#f-durationCanMap
        let mut s = String::new();
        let months = duration.year_month.months;
        let duration = duration.day_time;
        if months < 0 || duration.num_milliseconds() < 0 {
            s.push('-');
        }
        s.push('P');
        if months != 0 && duration.num_milliseconds() != 0 {
            Self::push_canonical_year_month_duration_fragment(&mut s, months);
            Self::push_canonical_day_time_duration_fragment(&mut s, &duration);
        } else if months != 0 {
            Self::push_canonical_year_month_duration_fragment(&mut s, months);
        } else {
            Self::push_canonical_day_time_duration_fragment(&mut s, &duration);
        }
        s
    }

    pub(crate) fn canonical_year_month_duration(year_month: YearMonthDuration) -> String {
        let mut s = String::new();
        let months = year_month.months;
        if months < 0 {
            s.push('-');
        }
        s.push('P');
        Self::push_canonical_year_month_duration_fragment(&mut s, months);
        s
    }

    pub(crate) fn canonical_day_time_duration(duration: &chrono::Duration) -> String {
        let mut s = String::new();
        if duration.num_milliseconds() < 0 {
            s.push('-');
        }
        s.push('P');
        Self::push_canonical_day_time_duration_fragment(&mut s, duration);
        s
    }

    pub(crate) fn canonical_date_time(date_time: &NaiveDateTimeWithOffset) -> String {
        let mut s = String::new();
        let offset = date_time.offset;
        let date_time = date_time.date_time;
        s.push_str(&date_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        let millis = date_time.timestamp_subsec_millis();
        Self::push_millis(&mut s, millis);
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_date_time_stamp(
        date_time: &chrono::DateTime<chrono::FixedOffset>,
    ) -> String {
        let mut s = String::new();
        s.push_str(&date_time.format("%Y-%m-%dT%H:%M:%S").to_string());
        let millis = date_time.timestamp_subsec_millis();
        Self::push_millis(&mut s, millis);
        let offset = date_time.offset();
        Self::push_canonical_time_zone_offset(&mut s, offset);
        s
    }

    pub(crate) fn canonical_time(time: &NaiveTimeWithOffset) -> String {
        let mut s = String::new();
        let offset = time.offset;
        let time = time.time;
        s.push_str(&time.format("%H:%M:%S").to_string());
        let millis = time.nanosecond() / 1_000_000;
        Self::push_millis(&mut s, millis);
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_date(date: &NaiveDateWithOffset) -> String {
        let mut s = String::new();
        let offset = date.offset;
        let date = date.date;
        s.push_str(&date.format("%Y-%m-%d").to_string());
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_g_year_month(g_year_month: &GYearMonth) -> String {
        let mut s = String::new();
        let offset = g_year_month.offset;
        let year = g_year_month.year;
        let month = g_year_month.month;
        if year >= 0 {
            s.push_str(&format!("{:04}", year));
        } else {
            s.push_str(&format!("-{:04}", year.abs()));
        }
        s.push_str(&format!("-{:02}", month));
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_g_year(g_year: &GYear) -> String {
        let mut s = String::new();
        let offset = g_year.offset;
        let year = g_year.year;
        if year >= 0 {
            s.push_str(&format!("{:04}", year));
        } else {
            s.push_str(&format!("-{:04}", year.abs()));
        }
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_g_month_day(g_month_day: &GMonthDay) -> String {
        let mut s = String::new();
        let offset = g_month_day.offset;
        let month = g_month_day.month;
        let day = g_month_day.day;
        s.push_str(&format!("--{:02}-{:02}", month, day));
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_g_day(g_day: &GDay) -> String {
        let mut s = String::new();
        let offset = g_day.offset;
        let day = g_day.day;
        s.push_str(&format!("---{:02}", day));
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    pub(crate) fn canonical_g_month(g_month: &GMonth) -> String {
        let mut s = String::new();
        let offset = g_month.offset;
        let month = g_month.month;
        s.push_str(&format!("--{:02}", month));
        if let Some(offset) = offset {
            Self::push_canonical_time_zone_offset(&mut s, &offset);
        }
        s
    }

    fn push_canonical_day_time_duration_fragment(v: &mut String, duration: &chrono::Duration) {
        // https://www.w3.org/TR/2012/REC-xmlschema11-2-20120405/datatypes.html#f-duDTCan
        let ss = duration.num_milliseconds().abs();
        let ss = (ss as f64) / 1000.0;
        if ss.is_zero() {
            v.push_str("T0S");
            return;
        }
        let d = (ss / 86400.0) as u64;
        let h = ((ss % 86400.0) / 3600.0) as u64;
        let m = ((ss % 3600.0) / 60.0) as u16;
        let s: Decimal = (ss % 60.0)
            .try_into()
            .unwrap_or(Decimal::from(0))
            .round_dp(3);

        if d != 0 {
            v.push_str(&format!("{}D", d));
        }
        if h != 0 || m != 0 || !s.is_zero() {
            v.push('T');
        }
        if h != 0 {
            v.push_str(&format!("{}H", h));
        }
        if m != 0 {
            v.push_str(&format!("{}M", m));
        }
        if s != Decimal::from(0) {
            v.push_str(&format!("{}S", s));
        }
    }

    fn push_canonical_time_zone_offset(s: &mut String, offset: &chrono::FixedOffset) {
        let seconds = offset.local_minus_utc();
        if seconds == 0 {
            s.push('Z');
            return;
        }
        let is_negative = seconds < 0;
        let seconds = seconds.abs();
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        if is_negative {
            s.push('-');
        } else {
            s.push('+');
        }
        s.push_str(&format!("{:02}:{:02}", hours, minutes));
    }

    fn push_canonical_year_month_duration_fragment(s: &mut String, months: i64) {
        // https://www.w3.org/TR/2012/REC-xmlschema11-2-20120405/datatypes.html#f-duYMCan
        let months = months.abs();
        let years = months / 12;
        let months = months % 12;
        if years != 0 && months != 0 {
            s.push_str(&format!("{}Y", years));
            s.push_str(&format!("{}M", months));
        } else if years != 0 {
            s.push_str(&format!("{}Y", years));
        } else {
            s.push_str(&format!("{}M", months));
        }
    }

    fn push_millis(s: &mut String, millis: u32) {
        if !millis.is_zero() {
            s.push_str(format!(".{:03}", millis).trim_end_matches('0'));
        }
    }

    // https://www.w3.org/TR/xpath-functions-31/#casting-to-durations

    pub(crate) fn cast_to_duration(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_duration(&s),
            atomic::Atomic::Duration(_) => Ok(self.clone()),
            atomic::Atomic::YearMonthDuration(year_month_duration) => Ok(atomic::Atomic::Duration(
                Rc::new(Duration::from_year_month(year_month_duration)),
            )),
            atomic::Atomic::DayTimeDuration(duration) => Ok(atomic::Atomic::Duration(Rc::new(
                Duration::from_day_time(*duration.as_ref()),
            ))),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_year_month_duration(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_year_month_duration(&s)
            }
            atomic::Atomic::Duration(duration) => Ok(atomic::Atomic::YearMonthDuration(
                duration.year_month.clone(),
            )),
            atomic::Atomic::YearMonthDuration(_) => Ok(self.clone()),
            atomic::Atomic::DayTimeDuration(_) => {
                Ok(atomic::Atomic::YearMonthDuration(YearMonthDuration::new(0)))
            }
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_day_time_duration(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_day_time_duration(&s)
            }
            atomic::Atomic::Duration(duration) => {
                Ok(atomic::Atomic::DayTimeDuration(Rc::new(duration.day_time)))
            }
            atomic::Atomic::YearMonthDuration(_) => Ok(atomic::Atomic::DayTimeDuration(Rc::new(
                chrono::Duration::zero(),
            ))),
            atomic::Atomic::DayTimeDuration(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_date_time(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_date_time(&s),
            atomic::Atomic::DateTime(_) => Ok(self.clone()),
            atomic::Atomic::DateTimeStamp(date_time) => Ok(atomic::Atomic::DateTime(Rc::new(
                NaiveDateTimeWithOffset::new(date_time.naive_utc(), Some(date_time.offset().fix())),
            ))),
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::DateTime(Rc::new(
                NaiveDateTimeWithOffset::new(
                    date.date
                        .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()),
                    date.offset,
                ),
            ))),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_date_time_stamp(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_date_time_stamp(&s)
            }
            atomic::Atomic::DateTime(date_time) => {
                if let Some(offset) = date_time.offset {
                    Ok(atomic::Atomic::DateTimeStamp(Rc::new(
                        chrono::DateTime::from_naive_utc_and_offset(date_time.date_time, offset),
                    )))
                } else {
                    Err(error::Error::Type)
                }
            }
            atomic::Atomic::DateTimeStamp(_) => Ok(self.clone()),

            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_time(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_time(&s),
            atomic::Atomic::DateTime(date_time) => Ok(atomic::Atomic::Time(Rc::new(
                NaiveTimeWithOffset::new(date_time.date_time.time(), date_time.offset),
            ))),
            atomic::Atomic::DateTimeStamp(date_time) => Ok(atomic::Atomic::Time(Rc::new(
                NaiveTimeWithOffset::new(date_time.time(), Some(date_time.offset().fix())),
            ))),
            atomic::Atomic::Time(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_date(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_date(&s),
            atomic::Atomic::DateTime(date_time) => Ok(atomic::Atomic::Date(Rc::new(
                NaiveDateWithOffset::new(date_time.date_time.date(), date_time.offset),
            ))),
            atomic::Atomic::DateTimeStamp(date_time) => {
                Ok(atomic::Atomic::Date(Rc::new(NaiveDateWithOffset::new(
                    date_time.naive_utc().date(),
                    Some(date_time.offset().fix()),
                ))))
            }
            atomic::Atomic::Date(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_g_year_month(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_g_year_month(&s)
            }
            atomic::Atomic::DateTime(date_time) => {
                Ok(atomic::Atomic::GYearMonth(Rc::new(GYearMonth::new(
                    date_time.date_time.year(),
                    date_time.date_time.month(),
                    date_time.offset,
                ))))
            }
            atomic::Atomic::DateTimeStamp(date_time) => {
                Ok(atomic::Atomic::GYearMonth(Rc::new(GYearMonth::new(
                    date_time.year(),
                    date_time.month(),
                    Some(date_time.offset().fix()),
                ))))
            }
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::GYearMonth(Rc::new(GYearMonth::new(
                date.date.year(),
                date.date.month(),
                date.offset,
            )))),
            atomic::Atomic::GYearMonth(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_g_year(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_g_year(&s),
            atomic::Atomic::DateTime(date_time) => Ok(atomic::Atomic::GYear(Rc::new(GYear::new(
                date_time.date_time.year(),
                date_time.offset,
            )))),
            atomic::Atomic::DateTimeStamp(date_time) => Ok(atomic::Atomic::GYear(Rc::new(
                GYear::new(date_time.year(), Some(date_time.offset().fix())),
            ))),
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::GYear(Rc::new(GYear::new(
                date.date.year(),
                date.offset,
            )))),
            atomic::Atomic::GYear(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_g_month_day(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_g_month_day(&s)
            }
            atomic::Atomic::DateTime(date_time) => {
                Ok(atomic::Atomic::GMonthDay(Rc::new(GMonthDay::new(
                    date_time.date_time.month(),
                    date_time.date_time.day(),
                    date_time.offset,
                ))))
            }
            atomic::Atomic::DateTimeStamp(date_time) => {
                Ok(atomic::Atomic::GMonthDay(Rc::new(GMonthDay::new(
                    date_time.month(),
                    date_time.day(),
                    Some(date_time.offset().fix()),
                ))))
            }
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::GMonthDay(Rc::new(GMonthDay::new(
                date.date.month(),
                date.date.day(),
                date.offset,
            )))),
            atomic::Atomic::GMonthDay(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_g_day(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_g_day(&s),
            atomic::Atomic::DateTime(date_time) => Ok(atomic::Atomic::GDay(Rc::new(GDay::new(
                date_time.date_time.day(),
                date_time.offset,
            )))),
            atomic::Atomic::DateTimeStamp(date_time) => Ok(atomic::Atomic::GDay(Rc::new(
                GDay::new(date_time.day(), Some(date_time.offset().fix())),
            ))),
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::GDay(Rc::new(GDay::new(
                date.date.day(),
                date.offset,
            )))),
            atomic::Atomic::GDay(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_g_month(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => Err(error::Error::Type),
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => Self::parse_g_month(&s),
            atomic::Atomic::DateTime(date_time) => Ok(atomic::Atomic::GMonth(Rc::new(
                GMonth::new(date_time.date_time.month(), date_time.offset),
            ))),
            atomic::Atomic::DateTimeStamp(date_time) => Ok(atomic::Atomic::GMonth(Rc::new(
                GMonth::new(date_time.month(), Some(date_time.offset().fix())),
            ))),
            atomic::Atomic::Date(date) => Ok(atomic::Atomic::GMonth(Rc::new(GMonth::new(
                date.date.month(),
                date.offset,
            )))),
            atomic::Atomic::GMonth(_) => Ok(self.clone()),
            _ => Err(error::Error::Type),
        }
    }

    fn parse<'a, T: Into<atomic::Atomic>>(
        parser: impl Parser<'a, &'a str, T, MyExtra>,
        s: &'a str,
    ) -> error::Result<atomic::Atomic> {
        match parser.parse(s).into_result() {
            Ok(value) => Ok(value.into()),
            Err(e) => Err(match &e[0] {
                ParserError::ExpectedFound { .. } => error::Error::FORG0001,
                ParserError::Error(e) => e.clone(),
            }),
        }
    }

    // TODO: these parse functions have overhead I'd like to avoid
    // https://github.com/zesterer/chumsky/issues/501

    fn parse_duration(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);

        // TODO: this needs to move into the static context, otherwise
        // caching is of no use
        let parser = chumsky::cache::Cache::new(DurationParserCache);

        // let parser = duration_parser();
        Self::parse(parser.get(), &s)
    }

    fn parse_year_month_duration(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = year_month_duration_parser();
        Self::parse(parser, &s)
    }

    fn parse_day_time_duration(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = day_time_duration_parser();
        Self::parse(parser, &s)
    }

    fn parse_date_time(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = date_time_parser();
        Self::parse(parser, &s)
    }

    fn parse_date_time_stamp(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = date_time_stamp_parser();
        Self::parse(parser, &s)
    }

    fn parse_time(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = time_parser();
        Self::parse(parser, &s)
    }

    fn parse_date(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = date_parser();
        Self::parse(parser, &s)
    }

    fn parse_g_year_month(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = g_year_month_parser();
        Self::parse(parser, &s)
    }

    fn parse_g_year(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = g_year_parser();
        Self::parse(parser, &s)
    }

    fn parse_g_month_day(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = g_month_day_parser();
        Self::parse(parser, &s)
    }

    fn parse_g_day(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = g_day_parser();
        Self::parse(parser, &s)
    }

    fn parse_g_month(s: &str) -> error::Result<atomic::Atomic> {
        let s = whitespace_collapse(s);
        let parser = g_month_parser();
        Self::parse(parser, &s)
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ParserError {
    ExpectedFound {
        span: SimpleSpan<usize>,
        expected: Vec<Option<char>>,
        found: Option<char>,
    },
    Error(error::Error),
}

impl From<error::Error> for ParserError {
    fn from(e: error::Error) -> Self {
        Self::Error(e)
    }
}

impl<'a> chumsky::error::Error<'a, &'a str> for ParserError {
    fn expected_found<E: IntoIterator<Item = Option<MaybeRef<'a, char>>>>(
        expected: E,
        found: Option<MaybeRef<'a, char>>,
        span: SimpleSpan<usize>,
    ) -> Self {
        Self::ExpectedFound {
            span,
            expected: expected
                .into_iter()
                .map(|e| e.as_deref().copied())
                .collect(),
            found: found.as_deref().copied(),
        }
    }

    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (ParserError::ExpectedFound { .. }, a) => a,
            (a, ParserError::ExpectedFound { .. }) => a,
            (a, _) => a,
        }
    }
}

type MyExtra = extra::Err<ParserError>;

fn digit_parser<'a>() -> impl Parser<'a, &'a str, char, MyExtra> {
    any::<&str, MyExtra>().filter(|c: &char| c.is_ascii_digit())
}

fn digits_parser<'a>() -> impl Parser<'a, &'a str, String, MyExtra> {
    let digit = digit_parser();
    digit.repeated().at_least(1).collect::<String>()
}

fn number_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    // failed parse may result in an overflow
    digits_parser().try_map(|s, _| s.parse().map_err(|_| error::Error::FODT0001.into()))
}

fn sign_parser<'a>() -> impl Parser<'a, &'a str, bool, MyExtra> {
    just('-').or_not().map(|sign| sign.is_some())
}

fn duration_second_parser<'a>() -> impl Parser<'a, &'a str, (u32, u32), MyExtra> {
    let seconds_digits = digits_parser().boxed();
    let ms_digits = digits_parser().boxed();
    seconds_digits
        .clone()
        .then(just('.').ignore_then(ms_digits).or_not())
        .try_map(|(a, b), _| {
            let b = b.unwrap_or("0".to_string());
            // ignore anything below milliseconds
            let b = if b.len() > 3 { &b[..3] } else { &b };
            let l = b.len();

            let a = a.parse::<u32>().map_err(|_| error::Error::FODT0002)?;
            let b = b.parse::<u32>().map_err(|_| error::Error::FODT0002)?;
            Ok((a, b * 10u32.pow(3 - l as u32)))
        })
}

fn time_second_parser<'a>() -> impl Parser<'a, &'a str, (u32, u32), MyExtra> {
    let seconds_digits = two_digit_parser().boxed();
    let ms_digits = digits_parser().boxed();
    seconds_digits
        .clone()
        .then(just('.').ignore_then(ms_digits).or_not())
        .try_map(|(a, b), _| {
            let b = b.unwrap_or("0".to_string());
            // ignore anything below milliseconds
            let b = if b.len() > 3 { &b[..3] } else { &b };
            let l = b.len();

            let b = b.parse::<u32>().map_err(|_| error::Error::FODT0001)?;
            Ok((a, b * 10u32.pow(3 - l as u32)))
        })
}

fn year_month_fragment_parser<'a>() -> impl Parser<'a, &'a str, i64, MyExtra> {
    let number = number_parser().boxed();
    let year_y = number.clone().then_ignore(just('Y')).boxed();
    let month_m = number.then_ignore(just('M')).boxed();
    (year_y
        .clone()
        .then(month_m.clone())
        .map(|(years, months)| (years, months)))
    .or(year_y.map(|years| (years, 0)))
    .or(month_m.map(|months| (0, months)))
    .map(|(years, months)| years as i64 * 12 + months as i64)
}

fn year_month_duration_parser<'a>() -> impl Parser<'a, &'a str, YearMonthDuration, MyExtra> {
    let year_month = year_month_fragment_parser().boxed();
    let sign = sign_parser();
    sign.then_ignore(just('P'))
        .then(year_month.clone())
        .then_ignore(end())
        .map(|(sign, months)| YearMonthDuration::new(if sign { -months } else { months }))
}

fn day_time_fragment_parser<'a>() -> impl Parser<'a, &'a str, chrono::Duration, MyExtra> {
    let number = number_parser().boxed();
    let day_d = number.clone().then_ignore(just('D')).boxed();
    let hour_h = number.clone().then_ignore(just('H')).boxed();
    let minute_m = number.clone().then_ignore(just('M')).boxed();
    let second_s = duration_second_parser().then_ignore(just('S')).boxed();

    let time = just('T')
        .ignore_then(hour_h.or_not())
        .then(minute_m.or_not())
        .then(second_s.or_not())
        .try_map(|((hours, minutes), s_ms), _| {
            if hours.is_none() && minutes.is_none() && s_ms.is_none() {
                return Err(error::Error::FORG0001.into());
            }
            let hours = hours.unwrap_or(0);
            let minutes = minutes.unwrap_or(0);
            let s_ms = s_ms.unwrap_or((0, 0));
            let (seconds, milliseconds) = s_ms;
            Ok(chrono::Duration::hours(hours as i64)
                + chrono::Duration::minutes(minutes as i64)
                + chrono::Duration::seconds(seconds as i64)
                + chrono::Duration::milliseconds(milliseconds as i64))
        })
        .boxed();

    let days = day_d.map(|days| chrono::Duration::days(days as i64));

    let days_then_time = days
        .then(time.clone().or_not())
        .map(|(days_duration, time_duration)| {
            let time_duration = time_duration.unwrap_or(chrono::Duration::seconds(0));
            days_duration + time_duration
        });
    days_then_time.or(time)
}

fn day_time_duration_parser<'a>() -> impl Parser<'a, &'a str, chrono::Duration, MyExtra> {
    let day_time = day_time_fragment_parser().boxed();
    let sign = sign_parser();
    sign.then_ignore(just('P'))
        .then(day_time.clone())
        .then_ignore(end())
        .map(|(sign, duration)| if sign { -duration } else { duration })
}

fn duration_parser<'a>() -> impl Parser<'a, &'a str, Duration, MyExtra> {
    let year_month = year_month_fragment_parser().boxed();
    let day_time = day_time_fragment_parser().boxed();
    let sign = sign_parser();
    sign.then_ignore(just('P'))
        .then(year_month.clone().or_not())
        .then(day_time.clone().or_not())
        .then_ignore(end())
        .try_map(|((sign, months), duration), _| {
            if months.is_none() && duration.is_none() {
                return Err(error::Error::FORG0001.into());
            }
            let months = months.unwrap_or(0);
            let duration = duration.unwrap_or(chrono::Duration::seconds(0));
            if sign {
                Ok(Duration::new(-months, -duration))
            } else {
                Ok(Duration::new(months, duration))
            }
        })
}

struct DurationParserCache;

impl chumsky::cache::Cached for DurationParserCache {
    type Parser<'src> = Boxed<'src, 'src, &'src str, Duration, MyExtra>;

    fn make_parser<'src>(self) -> Self::Parser<'src> {
        Parser::boxed(duration_parser())
    }
}

fn year_parser<'a>() -> impl Parser<'a, &'a str, i32, MyExtra> {
    let digits = digits_parser();
    let sign = sign_parser();

    // the year may have 0 prefixes, unless it's larger than 4, in
    // which case we don't allow any prefixes

    // we use validate here otherwise different parser paths eradicate
    // FODT0001
    // https://github.com/zesterer/chumsky/issues/530
    let year_digits = digits.validate(|digits, _, emitter| {
        match digits.len().cmp(&4) {
            Ordering::Greater => {
                // cannot have any 0 prefix
                if digits.starts_with('0') {
                    emitter.emit(error::Error::FORG0001.into());
                    0
                } else if let Ok(year) = digits.parse::<i32>() {
                    year
                } else {
                    emitter.emit(error::Error::FODT0001.into());
                    0
                }
            }
            Ordering::Equal => {
                if let Ok(year) = digits.parse::<i32>() {
                    year
                } else {
                    emitter.emit(error::Error::FODT0001.into());
                    0
                }
            }
            Ordering::Less => {
                emitter.emit(error::Error::FORG0001.into());
                0
            }
        }
    });

    sign.then(year_digits)
        .map(|(sign, year)| if sign { -year } else { year })
}

// HACK: a hacked version of year_parser which only returns FORG0001 using
// map_err on the output of year_parser does not work, so we have to resort to
// this duplication for now.
fn year_for_g_parser<'a>() -> impl Parser<'a, &'a str, i32, MyExtra> {
    let digits = digits_parser();
    let sign = sign_parser();

    let year_digits = digits.validate(|digits, _, emitter| {
        match digits.len().cmp(&4) {
            Ordering::Greater => {
                // cannot have any 0 prefix
                if digits.starts_with('0') {
                    emitter.emit(error::Error::FORG0001.into());
                    0
                } else if let Ok(year) = digits.parse::<i32>() {
                    year
                } else {
                    emitter.emit(error::Error::FORG0001.into());
                    0
                }
            }
            Ordering::Equal => {
                if let Ok(year) = digits.parse::<i32>() {
                    year
                } else {
                    emitter.emit(error::Error::FORG0001.into());
                    0
                }
            }
            Ordering::Less => {
                emitter.emit(error::Error::FORG0001.into());
                0
            }
        }
    });

    sign.then(year_digits)
        .map(|(sign, year)| if sign { -year } else { year })
}

fn two_digit_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    let digit = digit_parser().boxed();
    digit
        .clone()
        .then(digit)
        .map(|(a, b)| a.to_digit(10).unwrap() * 10 + b.to_digit(10).unwrap())
}

fn month_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    two_digit_parser().validate(|month, _, emitter| {
        if month == 0 || month > 12 {
            emitter.emit(error::Error::FORG0001.into());
            0
        } else {
            month
        }
    })
}

fn day_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    two_digit_parser().try_map(|day, _| {
        if day == 0 || day > 31 {
            Err(error::Error::FORG0001.into())
        } else {
            Ok(day)
        }
    })
}

fn date_fragment_parser<'a>() -> impl Parser<'a, &'a str, chrono::NaiveDate, MyExtra> {
    let year = year_parser().boxed();
    let month = month_parser().boxed();
    let day = day_parser().boxed();
    year.then_ignore(just('-'))
        .then(month)
        .then_ignore(just('-'))
        .then(day)
        .try_map(|((year, month), day), _| {
            chrono::NaiveDate::from_ymd_opt(year, month, day).ok_or(error::Error::FORG0001.into())
        })
}

fn date_parser<'a>() -> impl Parser<'a, &'a str, NaiveDateWithOffset, MyExtra> {
    let date = date_fragment_parser().boxed();
    let tz = tz_parser().boxed();
    date.then(tz.or_not())
        .then_ignore(end())
        .map(|(date, offset)| NaiveDateWithOffset::new(date, offset))
}

fn hour_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    two_digit_parser().try_map(|hour, _| {
        if hour > 24 {
            Err(error::Error::FORG0001.into())
        } else {
            Ok(hour)
        }
    })
}

fn minute_parser<'a>() -> impl Parser<'a, &'a str, u32, MyExtra> {
    two_digit_parser().try_map(|minute, _| {
        if minute > 59 {
            Err(error::Error::FORG0001.into())
        } else {
            Ok(minute)
        }
    })
}

fn time_fragment_parser<'a>() -> impl Parser<'a, &'a str, (chrono::NaiveTime, bool), MyExtra> {
    let hour = hour_parser().boxed();
    let minute = minute_parser().boxed();
    let second = time_second_parser().boxed();
    hour.then_ignore(just(':'))
        .then(minute)
        .then_ignore(just(':'))
        .then(second)
        .try_map(|((hour, minute), (second, millisecond)), _| {
            let (hour, twentyfour) = if hour == 24 && minute == 0 && second == 0 && millisecond == 0
            {
                (0, true)
            } else {
                (hour, false)
            };
            let time = chrono::NaiveTime::from_hms_milli_opt(hour, minute, second, millisecond)
                .ok_or(ParserError::Error(error::Error::FORG0001))?;
            Ok((time, twentyfour))
        })
}

fn time_parser<'a>() -> impl Parser<'a, &'a str, NaiveTimeWithOffset, MyExtra> {
    let time = time_fragment_parser().boxed();
    let tz = tz_parser().boxed();
    time.then(tz.or_not())
        .then_ignore(end())
        .map(|((time, _twentyfour), offset)| NaiveTimeWithOffset::new(time, offset))
}

fn date_time_fragment_parser<'a>() -> impl Parser<'a, &'a str, chrono::NaiveDateTime, MyExtra> {
    let date = date_fragment_parser().boxed();
    let time = time_fragment_parser().boxed();
    date.then_ignore(just('T'))
        .then(time)
        .map(|(date, (time, twentyfour))| {
            let dt = date.and_time(time);
            if twentyfour {
                // we're one day behind now, as we went to 0:00.
                // move into the next day
                dt + chrono::Duration::days(1)
            } else {
                dt
            }
        })
}

fn date_time_parser<'a>() -> impl Parser<'a, &'a str, NaiveDateTimeWithOffset, MyExtra> {
    let date_time = date_time_fragment_parser().boxed();
    let tz = tz_parser().boxed();
    date_time
        .then(tz.or_not())
        .map(|(date_time, offset)| NaiveDateTimeWithOffset::new(date_time, offset))
}

fn date_time_stamp_parser<'a>(
) -> impl Parser<'a, &'a str, chrono::DateTime<chrono::FixedOffset>, MyExtra> {
    let date_time = date_time_fragment_parser().boxed();
    let tz = tz_parser().boxed();
    date_time
        .then(tz)
        .map(|(date_time, tz)| tz.from_utc_datetime(&date_time))
}

fn offset_time_parser<'a>() -> impl Parser<'a, &'a str, i32, MyExtra> {
    let hour = hour_parser().boxed();
    let minute = minute_parser().boxed();
    hour.then_ignore(just(":"))
        .then(minute)
        .try_map(|(hour, minute), _| {
            if hour > 14 || hour == 14 && minute > 0 {
                Err(error::Error::FORG0001.into())
            } else {
                Ok(hour as i32 * 60 + minute as i32)
            }
        })
}

fn offset_parser<'a>() -> impl Parser<'a, &'a str, chrono::FixedOffset, MyExtra> {
    one_of("+-")
        .then(offset_time_parser())
        .map(|(sign, offset)| {
            // make it into seconds
            let offset = offset * 60;
            if sign == '+' {
                chrono::FixedOffset::east_opt(offset).unwrap()
            } else {
                chrono::FixedOffset::west_opt(offset).unwrap()
            }
        })
}

fn tz_parser<'a>() -> impl Parser<'a, &'a str, chrono::FixedOffset, MyExtra> {
    let offset = offset_parser();
    just('Z').to(chrono::offset::Utc.fix()).or(offset)
}

fn g_year_parser<'a>() -> impl Parser<'a, &'a str, GYear, MyExtra> {
    let year = year_for_g_parser().boxed();
    let tz = tz_parser().boxed();
    year.then(tz.or_not())
        .then_ignore(end())
        .map(|(year, tz)| GYear::new(year, tz))
}

fn g_month_parser<'a>() -> impl Parser<'a, &'a str, GMonth, MyExtra> {
    let month = month_parser().boxed();
    let tz = tz_parser().boxed();
    just('-')
        .ignore_then(just('-'))
        .ignore_then(month)
        .then(tz.or_not())
        .then_ignore(end())
        .map(|(month, tz)| GMonth::new(month, tz))
}

fn g_day_parser<'a>() -> impl Parser<'a, &'a str, GDay, MyExtra> {
    let day = day_parser().boxed();
    let tz = tz_parser().boxed();
    just('-')
        .ignore_then(just('-'))
        .ignore_then(just('-'))
        .ignore_then(day)
        .then(tz.or_not())
        .then_ignore(end())
        .map(|(day, tz)| GDay::new(day, tz))
}

fn g_month_day_parser<'a>() -> impl Parser<'a, &'a str, GMonthDay, MyExtra> {
    let month = month_parser().boxed();
    let day = day_parser().boxed();
    let tz = tz_parser().boxed();
    just('-')
        .ignore_then(just('-'))
        .ignore_then(month)
        .then_ignore(just('-'))
        .then(day)
        .then(tz.or_not())
        .then_ignore(end())
        .try_map(|((month, day), tz), _| {
            // pick leap year 2000
            let date = chrono::NaiveDate::from_ymd_opt(2000, month, day);
            if date.is_some() {
                Ok(GMonthDay::new(month, day, tz))
            } else {
                Err(error::Error::FORG0001.into())
            }
        })
}

fn g_year_month_parser<'a>() -> impl Parser<'a, &'a str, GYearMonth, MyExtra> {
    let year = year_for_g_parser().boxed();
    let month = month_parser().boxed();
    let tz = tz_parser().boxed();
    year.map_err(|_| error::Error::FORG0001.into())
        .then_ignore(just('-'))
        .then(month)
        .then(tz.or_not())
        .then_ignore(end())
        .try_map(|((year, month), tz), _| {
            let date = chrono::NaiveDate::from_ymd_opt(year, month, 1);
            if date.is_some() {
                Ok(GYearMonth::new(year, month, tz))
            } else {
                Err(error::Error::FODT0001.into())
            }
        })
}

#[cfg(test)]
mod tests {
    use crate::atomic::datetime::ToDateTimeStamp;

    use super::*;

    fn eq<T: ToDateTimeStamp>(a: &T, b: &T) -> bool {
        a.to_date_time_stamp(chrono::offset::Utc.fix())
            == b.to_date_time_stamp(chrono::offset::Utc.fix())
    }

    #[test]
    fn test_year_month_parser() {
        assert_eq!(year_month_fragment_parser().parse("1Y2M").unwrap(), 14);
    }

    #[test]
    fn test_year_month_parser_missing_year() {
        assert_eq!(year_month_fragment_parser().parse("2M").unwrap(), 2);
    }

    #[test]
    fn test_year_month_parser_missing_month() {
        assert_eq!(year_month_fragment_parser().parse("1Y").unwrap(), 12);
    }

    #[test]
    fn test_year_month_parser_zero_year() {
        assert_eq!(year_month_fragment_parser().parse("0Y2M").unwrap(), 2);
    }

    #[test]
    fn test_year_month_parser_leading_zero() {
        assert_eq!(year_month_fragment_parser().parse("01Y02M").unwrap(), 14);
    }

    #[test]
    fn test_year_month_duration_parser() {
        assert_eq!(
            year_month_duration_parser().parse("P1Y2M").unwrap(),
            YearMonthDuration::new(14)
        );
    }

    #[test]
    fn test_year_month_duration_parser_negative() {
        assert_eq!(
            year_month_duration_parser().parse("-P1Y2M").unwrap(),
            YearMonthDuration::new(-14)
        );
    }

    #[test]
    fn test_year_month_duration_parser_extra_rejected() {
        assert!(year_month_duration_parser().parse("P1Yflurb").has_errors());
    }

    #[test]
    fn test_day_time_parser() {
        assert_eq!(
            day_time_fragment_parser().parse("1DT2H3M4S").unwrap(),
            chrono::Duration::days(1)
                + chrono::Duration::hours(2)
                + chrono::Duration::minutes(3)
                + chrono::Duration::seconds(4)
        );
    }

    #[test]
    fn test_day_time_parser_with_fraction_seconds() {
        assert_eq!(
            day_time_fragment_parser().parse("1DT2H3M4.5S").unwrap(),
            chrono::Duration::days(1)
                + chrono::Duration::hours(2)
                + chrono::Duration::minutes(3)
                + chrono::Duration::seconds(4)
                + chrono::Duration::milliseconds(500)
        );
    }

    #[test]
    fn test_day_time_parser_with_fraction_seconds_long() {
        assert_eq!(
            day_time_fragment_parser().parse("1DT2H3M4.5678S").unwrap(),
            chrono::Duration::days(1)
                + chrono::Duration::hours(2)
                + chrono::Duration::minutes(3)
                + chrono::Duration::seconds(4)
                + chrono::Duration::milliseconds(567)
        );
    }

    #[test]
    fn test_day_time_parser_just_days() {
        assert_eq!(
            day_time_fragment_parser().parse("1D").unwrap(),
            chrono::Duration::days(1)
        );
    }

    #[test]
    fn test_day_time_parser_just_time() {
        assert_eq!(
            day_time_fragment_parser().parse("T2H3M4S").unwrap(),
            chrono::Duration::hours(2)
                + chrono::Duration::minutes(3)
                + chrono::Duration::seconds(4)
        );
    }

    #[test]
    fn test_day_time_parser_just_seconds() {
        assert_eq!(
            day_time_fragment_parser().parse("T4S").unwrap(),
            chrono::Duration::seconds(4)
        );
    }

    #[test]
    fn test_day_time_parser_empty_fails() {
        assert!(day_time_fragment_parser().parse("").has_errors());
    }

    #[test]
    fn test_day_time_parser_just_t_fails() {
        assert!(day_time_fragment_parser().parse("T").has_errors());
    }

    #[test]
    fn test_duration_parser() {
        assert_eq!(
            duration_parser().parse("P1Y2M3DT4H5M6S").unwrap(),
            Duration::new(
                14,
                chrono::Duration::days(3)
                    + chrono::Duration::hours(4)
                    + chrono::Duration::minutes(5)
                    + chrono::Duration::seconds(6)
            )
        );
    }

    #[test]
    fn test_duration_parser_just_months() {
        assert_eq!(
            duration_parser().parse("P1Y2M").unwrap(),
            Duration::new(14, chrono::Duration::seconds(0))
        );
    }

    #[test]
    fn test_duration_parser_just_days() {
        assert_eq!(
            duration_parser().parse("P1D").unwrap(),
            Duration::new(0, chrono::Duration::days(1))
        );
    }

    #[test]
    fn test_duration_parser_nothing() {
        assert!(duration_parser().parse("P").has_errors());
    }

    #[test]
    fn test_date_parser_4_digit_year() {
        assert_eq!(
            date_parser().parse("2020-01-02").unwrap(),
            NaiveDateWithOffset::new(chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(), None)
        );
    }

    #[test]
    fn test_date_parser_more_digits_year() {
        assert_eq!(
            date_parser().parse("20200-01-02").unwrap(),
            NaiveDateWithOffset::new(chrono::NaiveDate::from_ymd_opt(20200, 1, 2).unwrap(), None)
        );
    }

    #[test]
    fn test_date_parser_year_with_zeros() {
        assert_eq!(
            date_parser().parse("0120-01-02").unwrap(),
            NaiveDateWithOffset::new(chrono::NaiveDate::from_ymd_opt(120, 1, 2).unwrap(), None)
        );
    }

    #[test]
    fn test_date_parser_wrong_month() {
        assert!(date_parser().parse("2020-13-02").has_errors());
    }

    #[test]
    fn test_date_parser_wrong_day() {
        assert!(date_parser().parse("2020-01-32").has_errors());
    }

    #[test]
    fn test_date_parser_early_year_without_zeros_fails() {
        assert!(date_parser().parse("120-01-02").has_errors());
    }

    #[test]
    fn test_date_parser_long_year_leading_zeros_fails() {
        assert!(date_parser().parse("012020-01-02").has_errors());
    }

    #[test]
    fn test_date_parser_junk_fails() {
        assert!(date_parser().parse("2020-01-02flurb").has_errors());
    }

    #[test]
    fn test_date_parser_utc() {
        assert_eq!(
            date_parser().parse("2020-01-02Z").unwrap(),
            NaiveDateWithOffset::new(
                chrono::NaiveDate::from_ymd_opt(2020, 1, 2).unwrap(),
                Some(chrono::offset::Utc.fix())
            )
        );
    }

    #[test]
    fn test_tz_parser_utc() {
        assert_eq!(tz_parser().parse("Z").unwrap(), chrono::offset::Utc.fix());
    }

    #[test]
    fn test_tz_parser_naive() {
        assert_eq!(tz_parser().or_not().parse("").unwrap(), None);
    }

    #[test]
    fn test_tz_parser_offset_east() {
        assert_eq!(
            tz_parser().parse("+01:00").unwrap(),
            chrono::FixedOffset::east_opt(3600).unwrap()
        );
    }

    #[test]
    fn test_tz_parser_offset_west() {
        assert_eq!(
            tz_parser().parse("-01:00").unwrap(),
            chrono::FixedOffset::west_opt(3600).unwrap()
        );
    }

    #[test]
    fn test_tz_parser_offset_too_big_fails() {
        assert!(tz_parser().parse("+15:00").has_errors());
    }

    #[test]
    fn test_tz_parser_offset_max_range() {
        assert_eq!(
            tz_parser().parse("+14:00").unwrap(),
            chrono::FixedOffset::east_opt(50400).unwrap()
        );
    }

    #[test]
    fn test_tz_parser_offset_too_big2_fails() {
        assert!(tz_parser().parse("+14:01").has_errors());
    }

    #[test]
    fn test_tz_parser_offset_wrong_minutes_fails() {
        assert!(tz_parser().parse("+01:60").has_errors());
    }

    #[test]
    fn test_time_parser() {
        assert_eq!(
            time_parser().parse("01:02:03.456").unwrap(),
            NaiveTimeWithOffset::new(
                chrono::NaiveTime::from_hms_milli_opt(1, 2, 3, 456).unwrap(),
                None
            )
        );
    }

    #[test]
    fn test_time_parser_no_ms() {
        assert_eq!(
            time_parser().parse("01:02:03").unwrap(),
            NaiveTimeWithOffset::new(
                chrono::NaiveTime::from_hms_milli_opt(1, 2, 3, 0).unwrap(),
                None
            )
        );
    }

    #[test]
    fn test_time_parser_utc() {
        assert_eq!(
            time_parser().parse("01:02:03.456Z").unwrap(),
            NaiveTimeWithOffset::new(
                chrono::NaiveTime::from_hms_milli_opt(1, 2, 3, 456).unwrap(),
                Some(chrono::offset::Utc.fix())
            )
        );
    }

    #[test]
    fn test_time_parser_fails() {
        assert!(time_parser().parse("25:00:00").has_errors());
    }

    #[test]
    fn test_time_parser_junk_fails() {
        assert!(time_parser().parse("01:02:03.456flurb").has_errors());
    }

    #[test]
    fn test_date_time_parser() {
        assert_eq!(
            date_time_parser().parse("2020-01-02T01:02:03.456").unwrap(),
            NaiveDateTimeWithOffset::new(
                chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                    .unwrap()
                    .and_hms_milli_opt(1, 2, 3, 456)
                    .unwrap(),
                None
            )
        );
    }

    #[test]
    fn test_date_time_parser_utc() {
        assert_eq!(
            date_time_parser()
                .parse("2020-01-02T01:02:03.456Z")
                .unwrap(),
            NaiveDateTimeWithOffset::new(
                chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                    .unwrap()
                    .and_hms_milli_opt(1, 2, 3, 456)
                    .unwrap(),
                Some(chrono::offset::Utc.fix())
            )
        );
    }

    #[test]
    fn test_date_time_parser_offset() {
        assert_eq!(
            date_time_parser()
                .parse("2020-01-02T01:02:03.456+01:00")
                .unwrap(),
            NaiveDateTimeWithOffset::new(
                chrono::NaiveDate::from_ymd_opt(2020, 1, 2)
                    .unwrap()
                    .and_hms_milli_opt(1, 2, 3, 456)
                    .unwrap(),
                Some(chrono::FixedOffset::east_opt(3600).unwrap())
            )
        );
    }

    #[test]
    fn test_date_time_parser_junk_fails() {
        assert!(date_time_parser()
            .parse("2020-01-02T01:02:03.456flurb")
            .has_errors());
    }

    #[test]
    fn test_g_year_parser() {
        assert_eq!(
            g_year_parser().parse("2020").unwrap(),
            GYear::new(2020, None)
        );
    }

    #[test]
    fn test_g_year_parser_negative() {
        assert_eq!(
            g_year_parser().parse("-2020").unwrap(),
            GYear::new(-2020, None)
        );
    }

    #[test]
    fn test_g_year_parser_longer() {
        assert_eq!(
            g_year_parser().parse("20200").unwrap(),
            GYear::new(20200, None)
        );
    }

    #[test]
    fn test_g_year_parser_tz() {
        assert_eq!(
            g_year_parser().parse("2020Z").unwrap(),
            GYear::new(2020, Some(chrono::offset::Utc.fix()))
        );
    }

    #[test]
    fn test_g_month_parser() {
        assert_eq!(
            g_month_parser().parse("--01").unwrap(),
            GMonth::new(1, None)
        );
    }

    #[test]
    fn test_g_day_parser() {
        assert_eq!(g_day_parser().parse("---01").unwrap(), GDay::new(1, None));
    }

    #[test]
    fn test_g_month_day_parser() {
        assert_eq!(
            g_month_day_parser().parse("--04-12Z").unwrap(),
            GMonthDay::new(4, 12, Some(chrono::offset::Utc.fix()))
        );
    }

    #[test]
    fn test_g_year_month_parser() {
        assert_eq!(
            g_year_month_parser().parse("2020-04Z").unwrap(),
            GYearMonth::new(2020, 4, Some(chrono::offset::Utc.fix()))
        );
    }

    #[test]
    fn test_year_parser_error() {
        assert_eq!(
            year_parser()
                .parse("1000000000000000000")
                .errors()
                .collect::<Vec<_>>(),
            vec![&ParserError::Error(error::Error::FODT0001)]
        );
    }

    #[test]
    fn test_year_parser_error_negative() {
        assert_eq!(
            year_parser()
                .parse("-1000000000000000000")
                .errors()
                .collect::<Vec<_>>(),
            vec![&ParserError::Error(error::Error::FODT0001)]
        );
    }

    #[test]
    fn test_date_parser_error() {
        assert_eq!(
            date_parser()
                .parse("1000000000000000000-01-01")
                .errors()
                .collect::<Vec<_>>(),
            vec![&ParserError::Error(error::Error::FODT0001)]
        );
    }
}
