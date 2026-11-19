// Parser for fn:parse-ietf-date
// https://www.w3.org/TR/xpath-functions-31/#func-parse-ietf-date
//
// Hand-written per the direction in
// https://github.com/Paligo/xee/issues/36: the interpreter's existing
// chumsky dependency is slated for removal, so this deliberately adds no
// parser-library usage. The grammar is the one in the spec: the three RFC
// 2616 (HTTP) date formats, extended with liberal whitespace, single-digit
// fields, and lowercase input.

use crate::atomic::NaiveDateTimeWithOffset;
use crate::error;

// full names before abbreviations so that "Tuesday" is not read as "Tue" + "sday"
const DAYNAMES: [&str; 14] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
    "Mon",
    "Tue",
    "Wed",
    "Thu",
    "Fri",
    "Sat",
    "Sun",
];

const MONTHNAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

// timezone names with their offset in minutes; "UTC" before "UT" so the
// longest match wins
const TZNAMES: [(&str, i32); 11] = [
    ("UTC", 0),
    ("UT", 0),
    ("GMT", 0),
    ("EST", -5 * 60),
    ("EDT", -4 * 60),
    ("CST", -6 * 60),
    ("CDT", -5 * 60),
    ("MST", -7 * 60),
    ("MDT", -6 * 60),
    ("PST", -8 * 60),
    ("PDT", -7 * 60),
];

const MAX_OFFSET_MINUTES: u32 = 14 * 60;

struct Time {
    hours: u32,
    minutes: u32,
    seconds: u32,
    nanoseconds: u32,
    // whether any fractional digit was nonzero, even beyond the nanosecond
    // precision that survives in nanoseconds
    nonzero_fraction: bool,
    offset_minutes: Option<i32>,
}

pub(crate) fn parse(input: &str) -> error::Result<NaiveDateTimeWithOffset> {
    Parser {
        input: input.as_bytes(),
        pos: 0,
    }
    .parse()
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    // input ::= S? (dayname ","? S)? ((datespec S time) | asctime) S?
    fn parse(mut self) -> error::Result<NaiveDateTimeWithOffset> {
        self.skip_whitespace();
        if self.accept_dayname() {
            self.accept(b',');
            self.require_whitespace()?;
        }
        let (date, time) = if matches!(self.peek(), Some(b'0'..=b'9')) {
            // datespec ::= daynum dsep monthname dsep year
            let day = self.daynum()?;
            self.date_separator()?;
            let month = self.monthname()?;
            self.date_separator()?;
            let year = self.year()?;
            self.require_whitespace()?;
            let time = self.time()?;
            ((year, month, day), time)
        } else {
            // asctime ::= monthname dsep daynum S time S year
            let month = self.monthname()?;
            self.date_separator()?;
            let day = self.daynum()?;
            self.require_whitespace()?;
            let time = self.time()?;
            self.require_whitespace()?;
            let year = self.year()?;
            ((year, month, day), time)
        };
        self.skip_whitespace();
        if self.pos != self.input.len() {
            return Err(error::Error::FORG0010);
        }
        build_date_time(date, time)
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn accept(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    // a case-insensitive match of the whole keyword
    fn accept_keyword(&mut self, keyword: &str) -> bool {
        let bytes = keyword.as_bytes();
        match self.input.get(self.pos..self.pos + bytes.len()) {
            Some(candidate) if candidate.eq_ignore_ascii_case(bytes) => {
                self.pos += bytes.len();
                true
            }
            _ => false,
        }
    }

    // S ::= ( x09 | x0A | x0D | x20 )+
    fn skip_whitespace(&mut self) -> bool {
        let start = self.pos;
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
        self.pos > start
    }

    fn require_whitespace(&mut self) -> error::Result<()> {
        if self.skip_whitespace() {
            Ok(())
        } else {
            Err(error::Error::FORG0010)
        }
    }

    fn digits(&mut self) -> &'a [u8] {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        &self.input[start..self.pos]
    }

    fn accept_dayname(&mut self) -> bool {
        DAYNAMES.iter().any(|dayname| self.accept_keyword(dayname))
    }

    fn monthname(&mut self) -> error::Result<u32> {
        for (index, monthname) in MONTHNAMES.iter().enumerate() {
            if self.accept_keyword(monthname) {
                return Ok(index as u32 + 1);
            }
        }
        Err(error::Error::FORG0010)
    }

    // dsep ::= S | (S? "-" S?)
    fn date_separator(&mut self) -> error::Result<()> {
        let seen_whitespace = self.skip_whitespace();
        if self.accept(b'-') {
            self.skip_whitespace();
            Ok(())
        } else if seen_whitespace {
            Ok(())
        } else {
            Err(error::Error::FORG0010)
        }
    }

    // daynum ::= digit digit?
    fn daynum(&mut self) -> error::Result<u32> {
        match self.digits() {
            digits @ ([_] | [_, _]) => Ok(to_number(digits)),
            _ => Err(error::Error::FORG0010),
        }
    }

    // year ::= digit digit (digit digit)?
    // A two-digit year must have 1900 added to it.
    fn year(&mut self) -> error::Result<i32> {
        match self.digits() {
            digits @ [_, _] => Ok(to_number(digits) as i32 + 1900),
            digits @ [_, _, _, _] => Ok(to_number(digits) as i32),
            _ => Err(error::Error::FORG0010),
        }
    }

    // time ::= hours ":" minutes (":" seconds)? (S? timezone)?
    fn time(&mut self) -> error::Result<Time> {
        let hours = match self.digits() {
            digits @ ([_] | [_, _]) => to_number(digits),
            _ => return Err(error::Error::FORG0010),
        };
        if !self.accept(b':') {
            return Err(error::Error::FORG0010);
        }
        let minutes = match self.digits() {
            digits @ [_, _] => to_number(digits),
            _ => return Err(error::Error::FORG0010),
        };
        // seconds ::= digit digit ("." digit+)?
        let (seconds, nanoseconds, nonzero_fraction) = if self.accept(b':') {
            let seconds = match self.digits() {
                digits @ [_, _] => to_number(digits),
                _ => return Err(error::Error::FORG0010),
            };
            let (nanoseconds, nonzero_fraction) = if self.accept(b'.') {
                match self.digits() {
                    [] => return Err(error::Error::FORG0010),
                    digits => (
                        fraction_to_nanoseconds(digits),
                        digits.iter().any(|&digit| digit != b'0'),
                    ),
                }
            } else {
                (0, false)
            };
            (seconds, nanoseconds, nonzero_fraction)
        } else {
            (0, 0, false)
        };
        // the timezone is optional, so restore the position if the
        // whitespace we consumed turns out not to be followed by one
        let mark = self.pos;
        self.skip_whitespace();
        let offset_minutes = match self.timezone()? {
            Some(offset_minutes) => Some(offset_minutes),
            None => {
                self.pos = mark;
                None
            }
        };
        Ok(Time {
            hours,
            minutes,
            seconds,
            nanoseconds,
            nonzero_fraction,
            offset_minutes,
        })
    }

    // timezone ::= tzname | tzoffset (S? "(" S? tzname S? ")")?
    //
    // Returns Ok(None) if no timezone starts at the current position;
    // a malformed timezone that does start here is an error, as nothing
    // else in the grammar can follow the time.
    fn timezone(&mut self) -> error::Result<Option<i32>> {
        if matches!(self.peek(), Some(b'+' | b'-')) {
            let offset_minutes = self.timezone_offset()?;
            // a parenthesized tzname after the offset is ignored, but
            // must still be valid
            let mark = self.pos;
            self.skip_whitespace();
            if self.accept(b'(') {
                self.skip_whitespace();
                if self.tzname().is_none() {
                    return Err(error::Error::FORG0010);
                }
                self.skip_whitespace();
                if !self.accept(b')') {
                    return Err(error::Error::FORG0010);
                }
            } else {
                self.pos = mark;
            }
            Ok(Some(offset_minutes))
        } else {
            Ok(self.tzname())
        }
    }

    fn tzname(&mut self) -> Option<i32> {
        TZNAMES
            .iter()
            .find(|(tzname, _)| self.accept_keyword(tzname))
            .map(|&(_, offset_minutes)| offset_minutes)
    }

    // tzoffset ::= ("+"|"-") hours ":"? minutes?
    fn timezone_offset(&mut self) -> error::Result<i32> {
        let negative = match self.peek() {
            Some(b'+') => false,
            Some(b'-') => true,
            _ => return Err(error::Error::FORG0010),
        };
        self.pos += 1;
        let digits = self.digits();
        let (hours, minutes) = if digits.len() <= 2 && !digits.is_empty() && self.accept(b':') {
            let hours = to_number(digits);
            let minutes = match self.digits() {
                [] => 0,
                minutes_digits @ [_, _] => to_number(minutes_digits),
                _ => return Err(error::Error::FORG0010),
            };
            (hours, minutes)
        } else {
            // without a colon the digits are read as H, HH, HMM or HHMM
            match digits {
                digits @ ([_] | [_, _]) => (to_number(digits), 0),
                [h, m @ ..] if m.len() == 2 => ((h - b'0') as u32, to_number(m)),
                [h1, h2, m @ ..] if m.len() == 2 => (to_number(&[*h1, *h2]), to_number(m)),
                _ => return Err(error::Error::FORG0010),
            }
        };
        if minutes > 59 {
            return Err(error::Error::FORG0010);
        }
        let total = hours * 60 + minutes;
        if total > MAX_OFFSET_MINUTES {
            return Err(error::Error::FORG0010);
        }
        Ok(if negative {
            -(total as i32)
        } else {
            total as i32
        })
    }
}

fn to_number(digits: &[u8]) -> u32 {
    digits
        .iter()
        .fold(0, |number, digit| number * 10 + (digit - b'0') as u32)
}

fn fraction_to_nanoseconds(digits: &[u8]) -> u32 {
    let mut nanoseconds = 0;
    for &digit in digits.iter().take(9) {
        nanoseconds = nanoseconds * 10 + (digit - b'0') as u32;
    }
    for _ in digits.len().min(9)..9 {
        nanoseconds *= 10;
    }
    nanoseconds
}

fn build_date_time(
    (year, month, day): (i32, u32, u32),
    time: Time,
) -> error::Result<NaiveDateTimeWithOffset> {
    let mut date =
        chrono::NaiveDate::from_ymd_opt(year, month, day).ok_or(error::Error::FORG0010)?;
    let Time {
        mut hours,
        minutes,
        seconds,
        nanoseconds,
        nonzero_fraction,
        offset_minutes,
    } = time;
    // 24:00:00 is midnight at the end of the day
    if hours == 24 {
        if minutes != 0 || seconds != 0 || nonzero_fraction {
            return Err(error::Error::FORG0010);
        }
        date = date.succ_opt().ok_or(error::Error::FORG0010)?;
        hours = 0;
    }
    let time = chrono::NaiveTime::from_hms_nano_opt(hours, minutes, seconds, nanoseconds)
        .ok_or(error::Error::FORG0010)?;
    // if no timezone is given, an offset of 00:00 is assumed
    let offset = chrono::FixedOffset::east_opt(offset_minutes.unwrap_or(0) * 60)
        .ok_or(error::Error::FORG0010)?;
    Ok(NaiveDateTimeWithOffset::new(
        date.and_time(time),
        Some(offset),
    ))
}

#[cfg(test)]
mod tests {
    use chrono::{FixedOffset, NaiveDate};

    use super::*;

    fn dt(
        date: (i32, u32, u32),
        time: (u32, u32, u32),
        nanos: u32,
        offset_hours: i32,
    ) -> NaiveDateTimeWithOffset {
        let (y, mo, d) = date;
        let (h, mi, s) = time;
        NaiveDateTimeWithOffset::new(
            NaiveDate::from_ymd_opt(y, mo, d)
                .unwrap()
                .and_hms_nano_opt(h, mi, s, nanos)
                .unwrap(),
            Some(FixedOffset::east_opt(offset_hours * 3600).unwrap()),
        )
    }

    #[track_caller]
    fn ok(input: &str, expected: NaiveDateTimeWithOffset) {
        assert_eq!(parse(input), Ok(expected), "input: {:?}", input);
    }

    #[track_caller]
    fn err(input: &str) {
        assert_eq!(
            parse(input),
            Err(error::Error::FORG0010),
            "input: {:?}",
            input
        );
    }

    #[test]
    fn spec_examples() {
        ok(
            "Wed, 06 Jun 1994 07:29:35 GMT",
            dt((1994, 6, 6), (7, 29, 35), 0, 0),
        );
        ok(
            "Wed, 6 Jun 94 07:29:35 GMT",
            dt((1994, 6, 6), (7, 29, 35), 0, 0),
        );
        ok(
            "Wed Jun 06 11:54:45 EST 2013",
            dt((2013, 6, 6), (11, 54, 45), 0, -5),
        );
        ok(
            "Sunday, 06-Nov-94 08:49:37 GMT",
            dt((1994, 11, 6), (8, 49, 37), 0, 0),
        );
        ok(
            "Wed, 6 Jun 94 07:29:35 +0500",
            dt((1994, 6, 6), (7, 29, 35), 0, 5),
        );
    }

    #[test]
    fn basic_datespec() {
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        ok("Wed, 20 Aug 2014 19:36:01 GMT", base.clone());
        // whitespace at the start and end is ignored
        ok("  Wed, 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("Wed, 20 Aug 2014 19:36:01 GMT    ", base.clone());
        // single-digit day and hour
        ok(
            "Tue, 9 Sep 2014 19:36:01 GMT",
            dt((2014, 9, 9), (19, 36, 1), 0, 0),
        );
        ok(
            "Tue,  9 Sep 2014 19:36:01 GMT",
            dt((2014, 9, 9), (19, 36, 1), 0, 0),
        );
        ok(
            "Tue, 09 Sep 2014 19:36:01 GMT",
            dt((2014, 9, 9), (19, 36, 1), 0, 0),
        );
        ok(
            "Wed, 20 Aug 2014 9:36:01 GMT",
            dt((2014, 8, 20), (9, 36, 1), 0, 0),
        );
    }

    #[test]
    fn daynames_optional_and_ignored() {
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        // no comma after dayname
        ok("Wed 20 Aug 2014 19:36:01 GMT", base.clone());
        // full dayname
        ok("Wednesday 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("Tuesday, 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("Thursday, 20 Aug 2014 19:36:01 GMT", base.clone());
        // no dayname at all
        ok(" 20 Aug 2014 19:36:01 GMT", base.clone());
        ok(
            "Mon 8 Sep 2014 19:36:01 GMT",
            dt((2014, 9, 8), (19, 36, 1), 0, 0),
        );
        // the dayname is not checked against the date (2014-08-20 was a Wednesday)
        ok("Sun, 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("Mon, 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("FRI, 20 Aug 2014 19:36:01 GMT", base.clone());
        ok("SAT, 20 Aug 2014 19:36:01 GMT", base);
    }

    #[test]
    fn case_insensitive() {
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        ok("wed, 20 aug 2014 19:36:01 gmt", base.clone());
        ok("WED, 20 AUG 2014 19:36:01 GMT", base);
    }

    #[test]
    fn fractional_seconds() {
        ok(
            "Wed, 20 Aug 2014 19:36:01.25 GMT",
            dt((2014, 8, 20), (19, 36, 1), 250_000_000, 0),
        );
        ok(
            "Wed, 20 Aug 2014 19:36:01.299 GMT",
            dt((2014, 8, 20), (19, 36, 1), 299_000_000, 0),
        );
    }

    #[test]
    fn missing_seconds_and_timezone() {
        let no_seconds = dt((2014, 8, 20), (19, 36, 0), 0, 0);
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        ok("Wed, 20 Aug 2014 19:36 GMT", no_seconds.clone());
        // absent timezone means an offset of 00:00
        ok("WED, 20 AUG 2014 19:36:01", base.clone());
        ok("Wed, 20 Aug 2014 19:36", no_seconds.clone());
        // timezone attached without whitespace
        ok("Wed, 20 Aug 2014 19:36:01GMT", base);
        ok("Wed, 20 Aug 2014 19:36GMT", no_seconds);
    }

    #[test]
    fn hyphen_date_separators() {
        let no_seconds = dt((2014, 8, 20), (19, 36, 0), 0, 0);
        ok("Wed, 20 - Aug - 2014 19:36", no_seconds.clone());
        ok("Wed, 20- Aug- 2014 19:36", no_seconds.clone());
        ok("Wed, 20 -Aug -2014 19:36", no_seconds);
    }

    #[test]
    fn named_timezones() {
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        ok("Wed, 20 Aug 2014 19:36:01 UT", base.clone());
        ok("Wed, 20 Aug 2014 19:36:01 UTC", base);
        ok(
            "Wed, 20 Aug 2014 14:36:01 EST",
            dt((2014, 8, 20), (14, 36, 1), 0, -5),
        );
        ok(
            "Wed, 20 Aug 2014 15:36:01 EDT",
            dt((2014, 8, 20), (15, 36, 1), 0, -4),
        );
        ok(
            "Wed, 20 Aug 2014 13:36:01 CST",
            dt((2014, 8, 20), (13, 36, 1), 0, -6),
        );
        ok(
            "Wed, 20 Aug 2014 14:36:01 CDT",
            dt((2014, 8, 20), (14, 36, 1), 0, -5),
        );
        ok(
            "Wed, 20 Aug 2014 12:36:01 MST",
            dt((2014, 8, 20), (12, 36, 1), 0, -7),
        );
        ok(
            "Wed, 20 Aug 2014 13:36:01 MDT",
            dt((2014, 8, 20), (13, 36, 1), 0, -6),
        );
        ok(
            "Wed, 20 Aug 2014 11:36:01 PST",
            dt((2014, 8, 20), (11, 36, 1), 0, -8),
        );
        ok(
            "Wed, 20 Aug 2014 12:36:01 PDT",
            dt((2014, 8, 20), (12, 36, 1), 0, -7),
        );
    }

    #[test]
    fn offset_timezones() {
        let minus5 = dt((2014, 8, 20), (14, 36, 1), 0, -5);
        let plus5 = dt((2014, 8, 20), (14, 36, 1), 0, 5);
        ok("Wed, 20 Aug 2014 14:36:01-05:00", minus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01-05", minus5.clone());
        // trailing colon with no minutes
        ok("Wed, 20 Aug 2014 14:36:01-05:", minus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01 -05", minus5.clone());
        // colonless digits are H, HH, HMM or HHMM
        ok("Wed, 20 Aug 2014 14:36:01 -0500", minus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01 +0500", plus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01+05", plus5);
        // extreme but valid offset
        ok(
            "Wed, 20 Aug 2014 14:36:01 +14:00",
            dt((2014, 8, 20), (14, 36, 1), 0, 14),
        );
    }

    #[test]
    fn offset_with_parenthesized_name() {
        let minus5 = dt((2014, 8, 20), (14, 36, 1), 0, -5);
        let plus5 = dt((2014, 8, 20), (14, 36, 1), 0, 5);
        // the tzname is ignored when a tzoffset is given
        ok("Wed, 20 Aug 2014 14:36:01 -05:00(EST)", minus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01 -05:00(GMT)", minus5.clone());
        ok("Wed, 20 Aug 2014 14:36:01 -05:00  (  EST  )", minus5);
        ok("Wed, 20 Aug 2014 14:36:01 +0500(EST)", plus5);
    }

    #[test]
    fn asctime() {
        let no_seconds = dt((2014, 8, 20), (19, 36, 0), 0, 0);
        let base = dt((2014, 8, 20), (19, 36, 1), 0, 0);
        let minus5 = dt((2014, 8, 20), (14, 36, 1), 0, -5);
        ok("Aug-20 19:36 2014", no_seconds.clone());
        ok("Aug 20 19:36 2014", no_seconds);
        ok("Wed Aug 20 19:36:01 2014", base);
        ok("Aug 20 14:36:01 -05 2014", minus5.clone());
        ok("Aug 20 14:36:01 -05:00 2014", minus5.clone());
        ok("Aug 20 14:36:01 -05:00(EST) 2014", minus5.clone());
        ok("Wed, Aug 20 14:36:01EST 2014", minus5.clone());
        ok("Wed Aug 20 14:36:01-05:00 2014", minus5);
        ok(
            "Aug 20 19:36:01 -5 2014",
            dt((2014, 8, 20), (19, 36, 1), 0, -5),
        );
        ok(
            "Aug 20 4:36:01 -5:00 2014",
            dt((2014, 8, 20), (4, 36, 1), 0, -5),
        );
        ok(
            "Aug 20 4:36:01 -500 2014",
            dt((2014, 8, 20), (4, 36, 1), 0, -5),
        );
        ok("Feb-02 02:02-02: 02", dt((1902, 2, 2), (2, 2, 0), 0, -2));
    }

    #[test]
    fn years() {
        // a two-digit year has 1900 added to it
        ok("20 Aug 14 19:36:01", dt((1914, 8, 20), (19, 36, 1), 0, 0));
        ok("Aug 20 19:36:01 14", dt((1914, 8, 20), (19, 36, 1), 0, 0));
        ok(
            "Aug-20 14:36:01-05(EST) 14",
            dt((1914, 8, 20), (14, 36, 1), 0, -5),
        );
        // a four-digit year is taken as given
        ok("20 Aug 0070 19:36:01", dt((70, 8, 20), (19, 36, 1), 0, 0));
    }

    #[test]
    fn hour_24() {
        let next_day = dt((2014, 8, 21), (0, 0, 0), 0, 0);
        ok("Aug 20 24:00:00 2014", next_day.clone());
        ok("Aug 20 24:00 2014", next_day.clone());
        ok("Aug 20 24:00:00.000 2014", next_day);
        err("Aug 20 24:00:01 2014");
        err("Aug 20 24:01:00 2014");
        err("Aug 20 24:00:00.5 2014");
        // a nonzero fraction beyond nanosecond precision is still nonzero
        err("Aug 20 24:00:00.0000000001 2014");
        // off the 24:00 path the same tail simply truncates
        ok(
            "Aug 20 12:00:00.0000000001 2014",
            dt((2014, 8, 20), (12, 0, 0), 0, 0),
        );
    }

    #[test]
    fn offset_with_minutes() {
        // half-hour zones exercise the minutes part of every offset form
        let time = NaiveDate::from_ymd_opt(2014, 8, 20)
            .unwrap()
            .and_hms_nano_opt(14, 36, 1, 0)
            .unwrap();
        let plus = NaiveDateTimeWithOffset::new(
            time,
            Some(FixedOffset::east_opt((5 * 60 + 30) * 60).unwrap()),
        );
        ok("Aug 20 14:36:01 +05:30 2014", plus.clone());
        ok("Aug 20 14:36:01 +0530 2014", plus.clone());
        ok("Aug 20 14:36:01 +530 2014", plus);
        let minus = NaiveDateTimeWithOffset::new(
            time,
            Some(FixedOffset::east_opt(-(5 * 60 + 30) * 60).unwrap()),
        );
        ok("Aug 20 14:36:01 -05:30 2014", minus.clone());
        ok("Aug 20 14:36:01 -530 2014", minus);
    }

    #[test]
    fn boundary_years() {
        // the grammar caps years at four digits, so chrono's range is
        // unreachable; the lexical extremes must still work, including a
        // 24:00 rollover across the maximum year (year 10000 is a valid
        // xs:dateTime value; that the serializer prints five-digit years
        // with a leading "+" is a separate pre-existing issue, also
        // reachable via xs:dateTime("10000-01-01T00:00:00Z"))
        ok("Jan 1 00:00 0001", dt((1, 1, 1), (0, 0, 0), 0, 0));
        ok("Dec 31 24:00 9999", dt((10000, 1, 1), (0, 0, 0), 0, 0));
    }

    #[test]
    fn fuzz_smoke_never_panics() {
        // deterministic mutation fuzz: whatever bytes arrive, the parser
        // must return Ok or Err, never panic
        let seeds: &[&str] = &[
            "Wed, 06 Jun 1994 07:29:35 GMT",
            "Sunday, 06-Nov-94 08:49:37 GMT",
            "Wed Jun 06 11:54:45 EST 2013",
            "Aug-20 14:36:01-05(EST) 14",
            "Feb-02 02:02-02: 02",
            "Aug 20 24:00:00.000 2014",
            "Wed, 20 Aug 2014 14:36:01 -05:00  (  EST  )",
            "Wed, 20 Aug 2014 19:36:01.25 GMT",
        ];
        let mut state: u64 = 0x853c49e6748fea9b;
        let mut next = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (state >> 33) as u32
        };
        for round in 0..50_000 {
            let seed = seeds[round % seeds.len()];
            let mut bytes = seed.as_bytes().to_vec();
            for _ in 0..(next() % 4 + 1) {
                match next() % 3 {
                    0 if !bytes.is_empty() => {
                        let i = (next() as usize) % bytes.len();
                        bytes[i] = (next() % 128) as u8;
                    }
                    1 => {
                        let i = (next() as usize) % (bytes.len() + 1);
                        bytes.insert(i, (next() % 128) as u8);
                    }
                    _ if !bytes.is_empty() => {
                        let i = (next() as usize) % bytes.len();
                        bytes.remove(i);
                    }
                    _ => {}
                }
            }
            if let Ok(s) = std::str::from_utf8(&bytes) {
                let _ = parse(s);
            }
        }
    }

    #[test]
    fn whitespace_forms() {
        // tab, CR and LF count as whitespace
        ok(
            "Wed,\t20 Aug 2014\r\n19:36:01 GMT",
            dt((2014, 8, 20), (19, 36, 1), 0, 0),
        );
        // other whitespace-like characters do not, at the edges either:
        // stricter than the previous implementation, which used a Unicode
        // trim, but the grammar's S production is exactly these four bytes
        err("Wed,\u{00A0}20 Aug 2014 19:36:01 GMT");
        err("Wed,\u{000B}20 Aug 2014 19:36:01 GMT");
        err("\u{00A0}Wed, 20 Aug 2014 19:36:01 GMT");
        err("Wed, 20 Aug 2014 19:36:01 GMT\u{00A0}");
        err("Wed, 20 Aug 2014 19:36:01 GMT\u{2028}");
    }

    #[test]
    fn invalid_syntax() {
        err("");
        err("2014-08-20T19:36:01Z");
        // three-digit day
        err("Wed, 020 Aug 2014 19:36:01 GMT");
        // full month name
        err("Wed, 20 August 2014 19:36:01 GMT");
        // three-digit year
        err("Wed, 20 Aug 114 19:36:01 GMT");
        err("Aug 20 19:36:01GMT 014");
        // not a dayname
        err("Boy 20 Aug 2014 19:36:01 GMT");
        err("Manchester, 20 Aug 2014 19:36:01 GMT");
        // comma in the wrong place / missing whitespace
        err("Aug,20 19:36:01 2014");
        err("Aug20 19:36:01 2014");
        err("Wed,20 Aug 2014 19:36:01");
        err("20Aug 2014 19:36:01");
        err("20 Aug2014 19:36:01");
        // missing day
        err("Aug 2014 19:36:01");
        // missing whitespace before the asctime year
        err("Aug 20 19:36:01GMT2014");
        // trailing content
        err("Aug 20 19:36:01GMT Manchester");
        err("Wed, 20 Aug 2014 19:36:01 GMT Manchester");
    }

    #[test]
    fn invalid_dates() {
        err("Mon, 32 Aug 2014 19:36:01 GMT");
        err("Sat, 29 Feb 2014 19:36:01 GMT");
        err("Wed, 00 Aug 2014 19:36:01 GMT");
        // negative year
        err("Feb 28 19:36:01 GMT -2000");
    }

    #[test]
    fn invalid_times() {
        // wrong separators
        err("Aug 20 19.36.01GMT 2014");
        // minutes and seconds must be two digits
        err("Aug 20 19:3:01GMT 2014");
        err("Aug 20 19:36:0.1GMT 2014");
        err("Aug 20 19:36:GMT 2014");
        // fraction without digits
        err("Aug 20 19:36:01.GMT 2014");
        // out of range
        err("Aug 20 29:36:01GMT 2014");
        err("Aug 20 19:66:01GMT 2014");
        err("Aug 20 19:36:71GMT 2014");
        // no leap seconds in xs:dateTime
        err("20 Aug 2014 19:36:60");
    }

    #[test]
    fn invalid_timezones() {
        // unknown timezone names
        err("Wed, 20 Aug 2014 19:36:01 CET");
        err("Aug 20 19:36:01 -05:00 () 2014");
        err("Aug 20 19:36:01 -05:00 (CET) 2014");
        // an unparenthesized tzname cannot follow a tzoffset
        err("Aug 20 19:36:01 -05:00 EST 2014");
        // a parenthesized tzname is only allowed after a tzoffset
        err("Aug 20 19:36(EST) 2014");
        // minutes part must be two digits
        err("Aug 20 19:36:01 -05:0 2014");
        // out of range
        err("Aug 20 19:36:01 -15:00 2014");
        err("Aug 20 19:36:01 -05:60 2014");
        err("Wed, 20 Aug 2014 14:36:01 +14:01");
        err("Wed, 20 Aug 2014 14:36:01 -1401");
    }
}
