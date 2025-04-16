use std::rc::Rc;

use chrono::Offset;
use ibig::IBig;
use rust_decimal::Decimal;

use crate::atomic;
use crate::error;

use super::cast_binary::cast_binary_arithmetic;
use super::datetime::ToDateTimeStamp;
use super::datetime::{
    NaiveDateTimeWithOffset, NaiveDateWithOffset, NaiveTimeWithOffset, YearMonthDuration,
};

pub(crate) fn op_subtract(
    a: atomic::Atomic,
    b: atomic::Atomic,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    use atomic::Atomic;

    let (a, b) = cast_binary_arithmetic(a, b)?;

    match (a, b) {
        (Atomic::Decimal(a), Atomic::Decimal(b)) => Ok(op_substract_decimal(a, b)?),
        (Atomic::Integer(_, a), Atomic::Integer(_, b)) => Ok(op_substract_integer(a, b)),
        (Atomic::Float(a), Atomic::Float(b)) => Ok((a - b).into()),
        (Atomic::Double(a), Atomic::Double(b)) => Ok((a - b).into()),
        // op:subtract-dates(A, B) -> xs:date
        (Atomic::Date(a), Atomic::Date(b)) => Ok(op_subtract_dates(a, b, default_offset)?),
        // op:subtract-yearMonthDuration-from-date(A, B) -> xs:date
        (Atomic::Date(a), Atomic::YearMonthDuration(b)) => {
            Ok(op_subtract_year_month_duration_from_date(a, b)?)
        }
        // op:subtract-dayTimeDuration-from-date(A, B) -> xs:date
        (Atomic::Date(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_subtract_day_time_duration_from_date(a, *b)?)
        }
        // op:subtract-times(A, B) -> xs:dayTimeDuration
        (Atomic::Time(a), Atomic::Time(b)) => Ok(op_subtract_times(a, b, default_offset)?),
        // op:subtract-dayTimeDuration-from-time(A, B) -> xs:time
        (Atomic::Time(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_subtract_day_time_duration_from_time(a, *b)?)
        }
        // op:subtract_dateTimes(A, B) -> xs:dayTimeDuration
        (Atomic::DateTime(a), Atomic::DateTime(b)) => {
            Ok(op_subtract_date_times(a, b, default_offset)?)
        }
        (Atomic::DateTimeStamp(a), Atomic::DateTimeStamp(b)) => {
            Ok(op_subtract_date_time_stamps(*a.as_ref(), *b.as_ref())?)
        }
        (Atomic::DateTimeStamp(a), Atomic::DateTime(b)) => Ok(
            op_subtract_date_time_from_date_time_stamp(a, b, default_offset)?,
        ),
        (Atomic::DateTime(a), Atomic::DateTimeStamp(b)) => Ok(
            op_subtract_date_time_stamp_from_date_time(a, b, default_offset)?,
        ),
        // op:subtract-yearMonthDuration-from-dateTime(A, B) -> xs:dateTime
        (Atomic::DateTime(a), Atomic::YearMonthDuration(b)) => {
            Ok(op_subtract_year_month_duration_from_date_time(a, b)?)
        }
        // op:subtract-yearMonthDuration-from-dateTimeStamp(A, B) -> xs:dateTimeStamp
        (Atomic::DateTimeStamp(a), Atomic::YearMonthDuration(b)) => {
            Ok(op_subtract_year_month_duration_from_date_time_stamp(a, b)?)
        }
        // op:subtract-dayTimeDuration-from-dateTime(A, B) -> xs:dateTime
        (Atomic::DateTime(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_subtract_day_time_duration_from_date_time(a, *b)?)
        }
        // op:subtract-dayTimeDuration-from-dateTimeStamp(A, B) -> xs:dateTimeStamp
        (Atomic::DateTimeStamp(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_subtract_day_time_duration_from_date_time_stamp(a, *b)?)
        }
        // op:subtract-year-monthDurations(A, B) -> xs:yearMonthDuration
        (Atomic::YearMonthDuration(a), Atomic::YearMonthDuration(b)) => {
            Ok(op_subtract_year_month_durations(a, b)?)
        }
        // op:subtract-dayTimeDurations(A, B) -> xs:dayTimeDuration
        (Atomic::DayTimeDuration(a), Atomic::DayTimeDuration(b)) => {
            Ok(op_subtract_day_time_durations(a, b)?)
        }
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

fn op_substract_decimal(a: Rc<Decimal>, b: Rc<Decimal>) -> error::Result<atomic::Atomic> {
    Ok(a.as_ref()
        .checked_sub(*b.as_ref())
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?
        .into())
}

fn op_substract_integer(a: Rc<IBig>, b: Rc<IBig>) -> atomic::Atomic {
    (a.as_ref() - b.as_ref()).into()
}

fn op_subtract_dates(
    a: Rc<NaiveDateWithOffset>,
    b: Rc<NaiveDateWithOffset>,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    let a = a.to_date_time_stamp(default_offset);
    let b = b.to_date_time_stamp(default_offset);
    op_subtract_date_time_stamps(a, b)
}

fn op_subtract_year_month_duration_from_date(
    a: Rc<NaiveDateWithOffset>,
    b: YearMonthDuration,
) -> error::Result<atomic::Atomic> {
    let a = a.as_ref();
    let date = a.date;
    let new_date = if b.months >= 0 {
        date.checked_sub_months(chrono::Months::new(b.months as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    } else {
        date.checked_add_months(chrono::Months::new(b.months.unsigned_abs() as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    }?;

    Ok(NaiveDateWithOffset::new(new_date, a.offset).into())
}

fn op_subtract_day_time_duration_from_date(
    a: Rc<NaiveDateWithOffset>,
    b: chrono::Duration,
) -> error::Result<atomic::Atomic> {
    let offset = a.as_ref().offset;
    let a = a.to_date_time_stamp(chrono::offset::Utc.fix());
    let a = a
        .checked_sub_signed(b)
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    let new_date = a.date_naive();
    Ok(NaiveDateWithOffset::new(new_date, offset).into())
}

fn op_subtract_times(
    a: Rc<NaiveTimeWithOffset>,
    b: Rc<NaiveTimeWithOffset>,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    let a = a.to_date_time_stamp(default_offset);
    let b = b.to_date_time_stamp(default_offset);
    op_subtract_date_time_stamps(a, b)
}

fn op_subtract_day_time_duration_from_time(
    a: Rc<NaiveTimeWithOffset>,
    b: chrono::Duration,
) -> error::Result<atomic::Atomic> {
    // this never fails, but wraps around
    let new_time = a.as_ref().time - b;
    Ok(NaiveTimeWithOffset::new(new_time, a.as_ref().offset).into())
}

fn op_subtract_date_times(
    a: Rc<NaiveDateTimeWithOffset>,
    b: Rc<NaiveDateTimeWithOffset>,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    let a = a.to_date_time_stamp(default_offset);
    let b = b.to_date_time_stamp(default_offset);
    op_subtract_date_time_stamps(a, b)
}

fn op_subtract_date_time_stamps(
    a: chrono::DateTime<chrono::FixedOffset>,
    b: chrono::DateTime<chrono::FixedOffset>,
) -> error::Result<atomic::Atomic> {
    Ok((a - b).into())
}

fn op_subtract_date_time_stamp_from_date_time(
    a: Rc<NaiveDateTimeWithOffset>,
    b: Rc<chrono::DateTime<chrono::FixedOffset>>,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    let a = a.to_date_time_stamp(default_offset);
    op_subtract_date_time_stamps(a, *b.as_ref())
}

fn op_subtract_date_time_from_date_time_stamp(
    a: Rc<chrono::DateTime<chrono::FixedOffset>>,
    b: Rc<NaiveDateTimeWithOffset>,
    default_offset: chrono::FixedOffset,
) -> error::Result<atomic::Atomic> {
    let b = b.to_date_time_stamp(default_offset);
    op_subtract_date_time_stamps(*a.as_ref(), b)
}

fn op_subtract_year_month_duration_from_date_time(
    a: Rc<NaiveDateTimeWithOffset>,
    b: YearMonthDuration,
) -> error::Result<atomic::Atomic> {
    let a = a.as_ref();
    let date_time = a.date_time;
    let new_date_time = if b.months >= 0 {
        date_time
            .checked_sub_months(chrono::Months::new(b.months as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    } else {
        date_time
            .checked_add_months(chrono::Months::new(b.months.unsigned_abs() as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    }?;

    Ok(NaiveDateTimeWithOffset::new(new_date_time, a.offset).into())
}

fn op_subtract_year_month_duration_from_date_time_stamp(
    a: Rc<chrono::DateTime<chrono::FixedOffset>>,
    b: YearMonthDuration,
) -> error::Result<atomic::Atomic> {
    let a = a.as_ref();
    let date_time = *a;
    let new_date_time = if b.months >= 0 {
        date_time
            .checked_sub_months(chrono::Months::new(b.months as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    } else {
        date_time
            .checked_add_months(chrono::Months::new(b.months.unsigned_abs() as u32))
            .ok_or(error::Error::new(error::ErrorCode::FOAR0002))
    }?;

    Ok(new_date_time.into())
}

fn op_subtract_day_time_duration_from_date_time(
    a: Rc<NaiveDateTimeWithOffset>,
    b: chrono::Duration,
) -> error::Result<atomic::Atomic> {
    let new_date_time = a
        .as_ref()
        .date_time
        .checked_sub_signed(b)
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    Ok(NaiveDateTimeWithOffset::new(new_date_time, a.as_ref().offset).into())
}

fn op_subtract_day_time_duration_from_date_time_stamp(
    a: Rc<chrono::DateTime<chrono::FixedOffset>>,
    b: chrono::Duration,
) -> error::Result<atomic::Atomic> {
    let new_date_time = (*a.as_ref())
        .checked_sub_signed(b)
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    Ok(new_date_time.into())
}

fn op_subtract_year_month_durations(
    a: YearMonthDuration,
    b: YearMonthDuration,
) -> error::Result<atomic::Atomic> {
    let new_months = a
        .months
        .checked_sub(b.months)
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;
    Ok(YearMonthDuration { months: new_months }.into())
}

fn op_subtract_day_time_durations(
    a: Rc<chrono::Duration>,
    b: Rc<chrono::Duration>,
) -> error::Result<atomic::Atomic> {
    let new_duration = (*a.as_ref())
        .checked_sub(b.as_ref())
        .ok_or(error::Error::new(error::ErrorCode::FOAR0002))?;

    Ok(new_duration.into())
}
