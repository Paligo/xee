// Canonical serialization of years outside 1..=9999.
//
// chrono's %Y formats years above 9999 with a leading '+', which is not
// valid in XSD lexical space, so the engine's own output failed to cast
// back: xs:dateTime(string(xs:dateTime("10000-01-01T00:00:00Z"))) raised
// FORG0001.

use xee_xpath::{error, Documents, Queries, Query};

fn eval(xpath: &str) -> error::Result<String> {
    let mut documents = Documents::new();
    let queries = Queries::default();
    let q = queries.one(xpath, |_, item| Ok(item.try_into_value::<String>()?))?;
    q.execute_build_context(&mut documents, |_| {})
}

#[test]
fn test_five_digit_years_serialize_without_plus() {
    assert_eq!(
        eval(r#"string(xs:dateTime("10000-01-01T00:00:00Z"))"#).unwrap(),
        "10000-01-01T00:00:00Z"
    );
    assert_eq!(
        eval(r#"string(xs:dateTimeStamp("10000-01-01T00:00:00Z"))"#).unwrap(),
        "10000-01-01T00:00:00Z"
    );
    assert_eq!(
        eval(r#"string(xs:date("10000-01-01"))"#).unwrap(),
        "10000-01-01"
    );
}

#[test]
fn test_five_digit_year_round_trips_through_the_engine() {
    assert_eq!(
        eval(r#"string(xs:dateTime(string(xs:dateTime("10000-01-01T00:00:00Z"))))"#).unwrap(),
        "10000-01-01T00:00:00Z"
    );
    assert_eq!(
        eval(r#"string(xs:date(string(xs:date("10000-01-01"))))"#).unwrap(),
        "10000-01-01"
    );
}

#[test]
fn test_other_year_forms_stay_canonical() {
    // negative years and year zero were already canonical
    assert_eq!(
        eval(r#"string(xs:dateTime("-0005-03-01T12:00:00"))"#).unwrap(),
        "-0005-03-01T12:00:00"
    );
    assert_eq!(
        eval(r#"string(xs:date("0000-01-01"))"#).unwrap(),
        "0000-01-01"
    );
    // the gregorian types formatted years manually all along
    assert_eq!(eval(r#"string(xs:gYear("10000"))"#).unwrap(), "10000");
}
