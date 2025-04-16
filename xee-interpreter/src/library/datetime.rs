// https://www.w3.org/TR/xpath-functions-31/#dates-times

use chrono::{Datelike, Offset, SubsecRound, Timelike};
use ibig::IBig;
use rust_decimal::Decimal;
use xee_xpath_macros::xpath_fn;

use crate::atomic::ToDateTimeStamp;
use crate::function::StaticFunctionDescription;
use crate::{
    atomic::NaiveDateTimeWithOffset, atomic::NaiveDateWithOffset, atomic::NaiveTimeWithOffset,
    context::DynamicContext, error, wrap_xpath_fn,
};

#[xpath_fn("fn:dateTime($arg1 as xs:date?, $arg2 as xs:time?) as xs:dateTime?")]
fn date_time(
    arg1: Option<NaiveDateWithOffset>,
    arg2: Option<NaiveTimeWithOffset>,
) -> error::Result<Option<NaiveDateTimeWithOffset>> {
    match (arg1, arg2) {
        (Some(arg1), Some(arg2)) => {
            let offset = match (arg1.offset, arg2.offset) {
                (Some(arg1), Some(arg2)) => {
                    if arg1 == arg2 {
                        Some(arg1)
                    } else {
                        return Err(error::Error::new(error::ErrorCode::FORG0008));
                    }
                }
                (Some(arg1), None) => Some(arg1),
                (None, Some(arg2)) => Some(arg2),
                (None, None) => None,
            };
            Ok(Some(NaiveDateTimeWithOffset::new(
                arg1.date.and_time(arg2.time),
                offset,
            )))
        }
        (Some(_), None) => Ok(None),
        (None, Some(_)) => Ok(None),
        (None, None) => Ok(None),
    }
}

#[xpath_fn("fn:year-from-dateTime($arg as xs:dateTime?) as xs:integer?")]
fn year_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date_time.year().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:month-from-dateTime($arg as xs:dateTime?) as xs:integer?")]
fn month_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date_time.month().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:day-from-dateTime($arg as xs:dateTime?) as xs:integer?")]
fn day_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date_time.day().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:hours-from-dateTime($arg as xs:dateTime?) as xs:integer?")]
fn hours_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date_time.hour().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:minutes-from-dateTime($arg as xs:dateTime?) as xs:integer?")]
fn minutes_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date_time.minute().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:seconds-from-dateTime($arg as xs:dateTime?) as xs:decimal?")]
fn seconds_from_date_time(arg: Option<NaiveDateTimeWithOffset>) -> error::Result<Option<Decimal>> {
    match arg {
        Some(arg) => Ok(Some(seconds(arg.date_time))),
        None => Ok(None),
    }
}

#[xpath_fn("fn:timezone-from-dateTime($arg as xs:dateTime?) as xs:dayTimeDuration?")]
fn timezone_from_date_time(
    arg: Option<NaiveDateTimeWithOffset>,
) -> error::Result<Option<chrono::Duration>> {
    match arg {
        Some(arg) => Ok(offset_to_duration_option(arg.offset)),
        None => Ok(None),
    }
}

#[xpath_fn("fn:year-from-date($arg as xs:date?) as xs:integer?")]
fn year_from_date(arg: Option<NaiveDateWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date.year().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:month-from-date($arg as xs:date?) as xs:integer?")]
fn month_from_date(arg: Option<NaiveDateWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date.month().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:day-from-date($arg as xs:date?) as xs:integer?")]
fn day_from_date(arg: Option<NaiveDateWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.date.day().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:timezone-from-date($arg as xs:date?) as xs:time?")]
fn timezone_from_date(arg: Option<NaiveDateWithOffset>) -> error::Result<Option<chrono::Duration>> {
    match arg {
        Some(arg) => Ok(offset_to_duration_option(arg.offset)),
        None => Ok(None),
    }
}

#[xpath_fn("fn:hours-from-time($arg as xs:time?) as xs:integer?")]
fn hours_from_time(arg: Option<NaiveTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.time.hour().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:minutes-from-time($arg as xs:time?) as xs:integer?")]
fn minutes_from_time(arg: Option<NaiveTimeWithOffset>) -> error::Result<Option<IBig>> {
    match arg {
        Some(arg) => Ok(Some(arg.time.minute().into())),
        None => Ok(None),
    }
}

#[xpath_fn("fn:seconds-from-time($arg as xs:time?) as xs:decimal?")]
fn seconds_from_time(arg: Option<NaiveTimeWithOffset>) -> error::Result<Option<Decimal>> {
    match arg {
        Some(arg) => Ok(Some(seconds(arg.time))),
        None => Ok(None),
    }
}

#[xpath_fn("fn:timezone-from-time($arg as xs:time?) as xs:dayTimeDuration?")]
fn timezone_from_time(arg: Option<NaiveTimeWithOffset>) -> error::Result<Option<chrono::Duration>> {
    match arg {
        Some(arg) => Ok(offset_to_duration_option(arg.offset)),
        None => Ok(None),
    }
}

#[xpath_fn("fn:adjust-dateTime-to-timezone($arg as xs:dateTime?) as xs:dateTime?")]
fn adjust_date_time_to_timezone1(
    context: &DynamicContext,
    arg: Option<NaiveDateTimeWithOffset>,
) -> error::Result<Option<NaiveDateTimeWithOffset>> {
    adjust_date_time_to_timezone(arg, Some(context.implicit_timezone()))
}

#[xpath_fn("fn:adjust-dateTime-to-timezone($arg as xs:dateTime?, $timezone as xs:dayTimeDuration?) as xs:dateTime?")]
fn adjust_date_time_to_timezone2(
    arg: Option<NaiveDateTimeWithOffset>,
    timezone: Option<chrono::Duration>,
) -> error::Result<Option<NaiveDateTimeWithOffset>> {
    adjust_date_time_to_timezone(arg, duration_to_offset(timezone)?)
}

fn adjust_date_time_to_timezone(
    arg: Option<NaiveDateTimeWithOffset>,
    offset: Option<chrono::FixedOffset>,
) -> error::Result<Option<NaiveDateTimeWithOffset>> {
    match (arg, offset) {
        (Some(arg), Some(offset)) => {
            let date_time = if arg.offset.is_some() {
                // we need to first turn this into a date time stamp;
                // the default offset will be ignored as we know we have
                // an arg.offset
                let stamp = arg.to_date_time_stamp(chrono::offset::Utc.fix());
                // now we need to take the same time in this offset
                let stamp = stamp.with_timezone(&offset);
                // now we get the naive datetime local again
                stamp.naive_local()
            } else {
                arg.date_time
            };
            Ok(Some(NaiveDateTimeWithOffset::new(date_time, Some(offset))))
        }
        (Some(arg), None) => Ok(Some(NaiveDateTimeWithOffset::new(arg.date_time, None))),
        (None, _) => Ok(None),
    }
}

#[xpath_fn("fn:adjust-date-to-timezone($arg as xs:date?) as xs:date?")]
fn adjust_date_to_timezone1(
    context: &crate::context::DynamicContext,
    arg: Option<NaiveDateWithOffset>,
) -> error::Result<Option<NaiveDateWithOffset>> {
    adjust_date_to_timezone(arg, Some(context.implicit_timezone()))
}

#[xpath_fn(
    "fn:adjust-date-to-timezone($arg as xs:date?, $timezone as xs:dayTimeDuration?) as xs:date?"
)]
fn adjust_date_to_timezone2(
    arg: Option<NaiveDateWithOffset>,
    timezone: Option<chrono::Duration>,
) -> error::Result<Option<NaiveDateWithOffset>> {
    adjust_date_to_timezone(arg, duration_to_offset(timezone)?)
}

fn adjust_date_to_timezone(
    arg: Option<NaiveDateWithOffset>,
    offset: Option<chrono::FixedOffset>,
) -> error::Result<Option<NaiveDateWithOffset>> {
    match (arg, offset) {
        (Some(arg), Some(offset)) => {
            let stamp = arg.to_date_time_stamp(chrono::offset::Utc.fix());
            let stamp = if arg.offset.is_some() {
                stamp + offset
            } else {
                stamp
            };
            Ok(Some(NaiveDateWithOffset::new(
                stamp.naive_utc().date(),
                Some(offset),
            )))
        }
        (Some(arg), None) => Ok(Some(NaiveDateWithOffset::new(arg.date, None))),
        (None, _) => Ok(None),
    }
}

#[xpath_fn("fn:adjust-time-to-timezone($arg as xs:time?) as xs:time?")]
fn adjust_time_to_timezone1(
    context: &crate::context::DynamicContext,
    arg: Option<NaiveTimeWithOffset>,
) -> error::Result<Option<NaiveTimeWithOffset>> {
    adjust_time_to_timezone(arg, Some(context.implicit_timezone()))
}

#[xpath_fn(
    "fn:adjust-time-to-timezone($arg as xs:time?, $timezone as xs:dayTimeDuration?) as xs:time?"
)]
fn adjust_time_to_timezone2(
    arg: Option<NaiveTimeWithOffset>,
    timezone: Option<chrono::Duration>,
) -> error::Result<Option<NaiveTimeWithOffset>> {
    adjust_time_to_timezone(arg, duration_to_offset(timezone)?)
}

fn adjust_time_to_timezone(
    arg: Option<NaiveTimeWithOffset>,
    offset: Option<chrono::FixedOffset>,
) -> error::Result<Option<NaiveTimeWithOffset>> {
    match (arg, offset) {
        (Some(arg), Some(offset)) => {
            let stamp = arg.to_date_time_stamp(chrono::offset::Utc.fix());
            let stamp = if let Some(_arg_offset) = arg.offset {
                // the arg offset is already processed when we do
                // to_date_time_stamp, but the offset still needs to be
                // added in this case
                stamp + offset
            } else {
                stamp
            };
            Ok(Some(NaiveTimeWithOffset::new(
                stamp.naive_utc().time(),
                Some(offset),
            )))
        }
        (Some(arg), None) => Ok(Some(NaiveTimeWithOffset::new(arg.time, None))),
        (None, _) => Ok(None),
    }
}

fn seconds(time: impl Timelike + SubsecRound + Copy) -> Decimal {
    let nanoseconds: Decimal = time.round_subsecs(3).nanosecond().into();
    let seconds: Decimal = time.second().into();
    seconds + (nanoseconds / Decimal::from(1_000_000_000))
}

fn offset_to_duration_option(offset: Option<chrono::FixedOffset>) -> Option<chrono::Duration> {
    offset.map(offset_to_duration)
}

pub(crate) fn offset_to_duration(offset: chrono::FixedOffset) -> chrono::Duration {
    chrono::Duration::seconds(offset.local_minus_utc() as i64)
}

fn duration_to_offset(
    duration: Option<chrono::Duration>,
) -> error::Result<Option<chrono::FixedOffset>> {
    if let Some(duration) = duration {
        if duration > chrono::Duration::hours(14)
            || duration < chrono::Duration::hours(-14)
            || duration.num_seconds() % (60 * 60) != 0
        {
            return Err(error::Error::new(error::ErrorCode::FODT0003));
        }
        Ok(Some(
            chrono::FixedOffset::east_opt(
                duration
                    .num_seconds()
                    .try_into()
                    .expect("too many seconds to convert"),
            )
            .unwrap(),
        ))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:parse-ietf-date($value as xs:string?) as xs:dateTime?")]
fn parse_ietf_date(value: Option<&str>) -> error::Result<Option<NaiveDateTimeWithOffset>> {
    if let Some(value) = value {
        match chrono::DateTime::parse_from_rfc2822(value.trim()) {
            Ok(date_time) => Ok(Some(date_time.into())),
            Err(_) => Err(error::Error::new(error::ErrorCode::FORG0010)),
        }
    } else {
        Ok(None)
    }
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(date_time),
        wrap_xpath_fn!(year_from_date_time),
        wrap_xpath_fn!(month_from_date_time),
        wrap_xpath_fn!(day_from_date_time),
        wrap_xpath_fn!(hours_from_date_time),
        wrap_xpath_fn!(minutes_from_date_time),
        wrap_xpath_fn!(seconds_from_date_time),
        wrap_xpath_fn!(timezone_from_date_time),
        wrap_xpath_fn!(year_from_date),
        wrap_xpath_fn!(month_from_date),
        wrap_xpath_fn!(day_from_date),
        wrap_xpath_fn!(timezone_from_date),
        wrap_xpath_fn!(hours_from_time),
        wrap_xpath_fn!(minutes_from_time),
        wrap_xpath_fn!(seconds_from_time),
        wrap_xpath_fn!(timezone_from_time),
        wrap_xpath_fn!(adjust_date_time_to_timezone1),
        wrap_xpath_fn!(adjust_date_time_to_timezone2),
        wrap_xpath_fn!(adjust_date_to_timezone1),
        wrap_xpath_fn!(adjust_date_to_timezone2),
        wrap_xpath_fn!(adjust_time_to_timezone1),
        wrap_xpath_fn!(adjust_time_to_timezone2),
        wrap_xpath_fn!(parse_ietf_date),
    ]
}
