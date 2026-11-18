// Tests for the interaction between the current date/time in the dynamic
// context, timezone offsets, and the date/time types.
//
// https://github.com/Paligo/xee/issues/117

use xee_xpath::{error, Documents, Queries, Query};

// At 2025-04-01T23:00:00-01:00 the local date is April 1st, but the UTC
// date is already April 2nd.
const NOW: &str = "2025-04-01T23:00:00-01:00";

fn eval(xpath: &str, now: &str) -> error::Result<String> {
    let mut documents = Documents::new();
    let queries = Queries::default();
    let q = queries.one(xpath, |_, item| Ok(item.try_into_value::<String>()?))?;
    let now = chrono::DateTime::parse_from_rfc3339(now).unwrap();
    q.execute_build_context(&mut documents, |builder| {
        builder.current_datetime(now);
    })
}

#[test]
fn test_current_date_uses_local_date() {
    assert_eq!(
        eval("string(current-date())", NOW).unwrap(),
        "2025-04-01-01:00"
    );
}

#[test]
fn test_date_of_current_date_time_matches_current_date() {
    assert_eq!(
        eval("string(xs:date(current-dateTime()))", NOW).unwrap(),
        "2025-04-01-01:00"
    );
}

#[test]
fn test_current_date_time_string() {
    assert_eq!(
        eval("string(current-dateTime())", NOW).unwrap(),
        "2025-04-01T23:00:00-01:00"
    );
}

#[test]
fn test_date_time_stamp_from_string_round_trips() {
    assert_eq!(
        eval(
            r#"string(xs:dateTimeStamp("2025-04-01T23:00:00-01:00"))"#,
            NOW
        )
        .unwrap(),
        "2025-04-01T23:00:00-01:00"
    );
    assert_eq!(
        eval(r#"string(xs:dateTimeStamp("2025-04-02T00:00:00Z"))"#, NOW).unwrap(),
        "2025-04-02T00:00:00Z"
    );
}

#[test]
fn test_date_time_to_date_time_stamp_round_trips() {
    assert_eq!(
        eval(
            r#"string(xs:dateTimeStamp(xs:dateTime("2025-04-01T23:00:00-01:00")))"#,
            NOW
        )
        .unwrap(),
        "2025-04-01T23:00:00-01:00"
    );
}

#[test]
fn test_date_time_stamp_to_date_uses_local_date() {
    assert_eq!(
        eval(
            r#"string(xs:date(xs:dateTimeStamp("2025-04-01T23:00:00-01:00")))"#,
            NOW
        )
        .unwrap(),
        "2025-04-01-01:00"
    );
}

#[test]
fn test_local_date_ahead_of_utc() {
    // here the local date is April 2nd while the UTC date is April 1st
    let now = "2025-04-02T01:00:00+03:00";
    assert_eq!(
        eval("string(current-date())", now).unwrap(),
        "2025-04-02+03:00"
    );
    assert_eq!(
        eval("string(xs:date(current-dateTime()))", now).unwrap(),
        "2025-04-02+03:00"
    );
}

#[test]
fn test_date_time_and_date_time_stamp_are_equal_map_keys() {
    // the same instant must be the same map key, whether it is stored as
    // xs:dateTime or as xs:dateTimeStamp
    assert_eq!(
        eval(
            r#"map{xs:dateTimeStamp("2025-01-01T13:00:00+01:00"): "hit"}(xs:dateTime("2025-01-01T12:00:00Z"))"#,
            NOW
        )
        .unwrap(),
        "hit"
    );
    assert_eq!(
        eval(
            r#"map{xs:dateTime("2025-01-01T13:00:00+01:00"): "hit"}(xs:dateTimeStamp("2025-01-01T12:00:00Z"))"#,
            NOW
        )
        .unwrap(),
        "hit"
    );
}

#[test]
fn test_date_time_stamp_out_of_range_is_an_error_not_a_panic() {
    // normalizing this value to UTC overflows chrono's datetime range;
    // that must surface as a dynamic error, never a panic
    let parsed = eval(
        r#"string("262142-12-31T23:30:00-01:00" cast as xs:dateTimeStamp)"#,
        NOW,
    );
    assert!(
        format!("{:?}", parsed.unwrap_err()).contains("FODT0001"),
        "string cast must raise FODT0001"
    );
    let cast = eval(
        r#"string(xs:dateTimeStamp(xs:dateTime("262142-12-31T23:30:00-01:00")))"#,
        NOW,
    );
    assert!(
        format!("{:?}", cast.unwrap_err()).contains("FODT0001"),
        "dateTime cast must raise FODT0001"
    );
}
