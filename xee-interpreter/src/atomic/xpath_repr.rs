use xot::xmlname::NameStrInfo;

use super::{Atomic, StringType};

impl Atomic {
    /// XPath representation of the atomic value.
    pub fn xpath_representation(&self) -> String {
        match self {
            Self::String(string_type, v) => match string_type {
                StringType::String => string_literal(v),
                _ => {
                    let schema_type = string_type.schema_type();
                    format!("xs:{}({})", schema_type.local_name(), string_literal(v))
                }
            },

            Self::Boolean(v) => {
                if *v {
                    "true()".to_string()
                } else {
                    "false()".to_string()
                }
            }
            // for any numeric type the canonical notation is enough
            Self::Decimal(_) | Self::Integer(_, _) | Self::Float(_) | Self::Double(_) => {
                self.string_value()
            }

            // QName is not represented by casting, as according to 3.14.2
            // in the XPath 3.1 spec casting to xs:QName can cause surprises
            // and it's preferable to use the fn:QName function
            Self::QName(v) => {
                format!(
                    r#"fn:QName({}, {})"#,
                    string_literal(v.namespace()),
                    string_literal(v.local_name())
                )
            }
            // everything else is represented by taking the canonical notation
            // and then casting it into the required type
            _ => self.canonical_xpath_representation(),
        }
    }

    fn canonical_xpath_representation(&self) -> String {
        format!(
            "xs:{}({})",
            self.schema_type().local_name(),
            string_literal(&self.string_value())
        )
    }
}

fn string_literal(s: &str) -> String {
    if s.contains('\"') {
        if s.contains('\'') {
            let s = s.replace('\"', r#""""#);
            format!(r#""{}""#, s)
        } else {
            format!(r#"'{}'"#, s)
        }
    } else {
        format!(r#""{}""#, s)
    }
}

#[cfg(test)]
mod tests {
    use ibig::IBig;
    use rust_decimal_macros::dec;

    use crate::atomic::{BinaryType, Duration, IntegerType};

    use super::*;

    #[test]
    fn test_string_simple() {
        let atomic: Atomic = "foo".into();
        assert_eq!(atomic.xpath_representation(), r#""foo""#);
    }

    #[test]
    fn test_string_with_single_quote() {
        let atomic: Atomic = "foo'bar".into();
        assert_eq!(atomic.xpath_representation(), r#""foo'bar""#);
    }

    #[test]
    fn test_string_with_double_quote() {
        let atomic: Atomic = r#"foo"bar"#.into();
        assert_eq!(atomic.xpath_representation(), r#"'foo"bar'"#);
    }

    #[test]
    fn test_string_with_both_quotes() {
        let atomic: Atomic = r#"foo'bar"baz"#.into();
        assert_eq!(atomic.xpath_representation(), r#""foo'bar""baz""#);
    }

    #[test]
    fn test_normalized_string() {
        let atomic = Atomic::String(StringType::NormalizedString, "foo".into());
        assert_eq!(
            atomic.xpath_representation(),
            r#"xs:normalizedString("foo")"#
        );
    }

    #[test]
    fn test_untyped() {
        let atomic = Atomic::Untyped("foo".into());
        assert_eq!(atomic.xpath_representation(), r#"xs:untypedAtomic("foo")"#);
    }

    #[test]
    fn test_boolean_true() {
        let atomic = Atomic::Boolean(true);
        assert_eq!(atomic.xpath_representation(), "true()");
    }

    #[test]
    fn test_boolean_false() {
        let atomic = Atomic::Boolean(false);
        assert_eq!(atomic.xpath_representation(), "false()");
    }

    #[test]
    fn test_decimal_left_right() {
        let atomic = Atomic::Decimal(dec!(1.5).into());
        assert_eq!(atomic.xpath_representation(), "1.5");
    }

    #[test]
    fn test_decimal_is_integer() {
        let atomic = Atomic::Decimal(dec!(1.0).into());
        assert_eq!(atomic.xpath_representation(), "1");
    }

    #[test]
    fn test_decimal_only_right() {
        let atomic = Atomic::Decimal(dec!(0.5).into());
        assert_eq!(atomic.xpath_representation(), "0.5");
    }

    #[test]
    fn test_integer() {
        let i: IBig = 1.into();
        let atomic = Atomic::Integer(IntegerType::Integer, i.into());
        assert_eq!(atomic.xpath_representation(), "1");
    }

    #[test]
    fn test_qname() {
        let name = xot::xmlname::OwnedName::new(
            "foo".to_string(),
            "http://example.com".to_string(),
            "".to_string(),
        );
        let atomic = Atomic::QName(name.into());
        assert_eq!(
            atomic.xpath_representation(),
            r#"fn:QName("http://example.com", "foo")"#
        );
    }

    #[test]
    fn test_hex_binary() {
        let atomic = Atomic::Binary(BinaryType::Hex, vec![0xDE, 0xAD, 0xBE, 0xEF].into());
        assert_eq!(atomic.xpath_representation(), "xs:hexBinary(\"DEADBEEF\")");
    }

    #[test]
    fn test_base64_binary() {
        let atomic = Atomic::Binary(BinaryType::Base64, vec![0xDE, 0xAD, 0xBE, 0xEF].into());
        assert_eq!(
            atomic.xpath_representation(),
            "xs:base64Binary(\"3q2+7w==\")"
        );
    }

    #[test]
    fn test_duration() {
        let duration = Duration::new(14, chrono::Duration::seconds(0));

        let atomic = Atomic::Duration(duration.into());
        assert_eq!(atomic.xpath_representation(), r#"xs:duration("P1Y2M")"#);
    }

    #[test]
    fn test_day_time_duration() {
        let atomic = Atomic::DayTimeDuration(chrono::Duration::seconds(641).into());
        assert_eq!(
            atomic.xpath_representation(),
            r#"xs:dayTimeDuration("PT10M41S")"#
        );
    }
}
