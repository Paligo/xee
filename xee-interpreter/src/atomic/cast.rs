use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use xot::xmlname::NameStrInfo;

use xee_schema_type::Xs;

use crate::atomic;
use crate::context;
use crate::error;

use super::cast_numeric::decimal_to_integer;

impl atomic::Atomic {
    pub(crate) fn parse_atomic<V>(s: &str) -> error::Result<atomic::Atomic>
    where
        Parsed<V>: FromStr,
        V: Into<atomic::Atomic>,
    {
        // TODO: re-establish error. I am not sure what error is returned
        // from the FromStr trait; it should be error::Error but evidently it's
        // not
        s.parse::<Parsed<V>>()
            .map(|p| p.into_inner().into())
            .map_err(|_| error::Error::new(error::ErrorCode::FORG0001))
    }

    pub(crate) fn parse_boolean(s: &str) -> error::Result<bool> {
        match s {
            "true" => Ok(true),
            "false" => Ok(false),
            "1" => Ok(true),
            "0" => Ok(false),
            _ => Err(error::Error::new(error::ErrorCode::FORG0001)),
        }
    }

    // from an atomic type to a canonical representation as a string
    pub(crate) fn into_canonical(self) -> String {
        match self {
            atomic::Atomic::Untyped(s) => s.to_string(),
            atomic::Atomic::String(_, s) => s.to_string(),
            atomic::Atomic::Float(OrderedFloat(f)) => Self::canonical_float(f),
            atomic::Atomic::Double(OrderedFloat(f)) => Self::canonical_float(f),
            atomic::Atomic::Decimal(d) => {
                if d.is_integer() {
                    let i: IBig = decimal_to_integer(d).unwrap();
                    i.to_string()
                } else {
                    d.normalize().to_string()
                }
            }
            atomic::Atomic::Integer(_, i) => i.to_string(),
            atomic::Atomic::Duration(duration) => Self::canonical_duration(duration.as_ref()),
            atomic::Atomic::YearMonthDuration(year_month) => {
                Self::canonical_year_month_duration(year_month)
            }
            atomic::Atomic::DayTimeDuration(duration) => {
                Self::canonical_day_time_duration(duration.as_ref())
            }
            atomic::Atomic::DateTime(date_time) => Self::canonical_date_time(date_time.as_ref()),
            atomic::Atomic::DateTimeStamp(date_time) => {
                Self::canonical_date_time_stamp(date_time.as_ref())
            }
            atomic::Atomic::Time(time) => Self::canonical_time(time.as_ref()),
            atomic::Atomic::Date(date) => Self::canonical_date(date.as_ref()),
            atomic::Atomic::GYearMonth(year_month) => {
                Self::canonical_g_year_month(year_month.as_ref())
            }
            atomic::Atomic::GYear(year) => Self::canonical_g_year(year.as_ref()),
            atomic::Atomic::GMonthDay(month_day) => Self::canonical_g_month_day(month_day.as_ref()),
            atomic::Atomic::GDay(day) => Self::canonical_g_day(day.as_ref()),
            atomic::Atomic::GMonth(month) => Self::canonical_g_month(month.as_ref()),
            atomic::Atomic::Boolean(b) => {
                if b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            atomic::Atomic::Binary(binary_type, data) => match binary_type {
                atomic::BinaryType::Base64 => Self::canonical_base64_binary(data.as_ref()),
                atomic::BinaryType::Hex => Self::canonical_hex_binary(data.as_ref()),
            },
            atomic::Atomic::QName(name) => name.full_name().to_string(),
        }
    }

    pub(crate) fn cast_to_schema_type(
        self,
        xs: Xs,
        static_context: &context::StaticContext,
    ) -> error::Result<atomic::Atomic> {
        // if we try to cast to any atomic type, we're already the correct type
        if xs == Xs::AnyAtomicType {
            return Ok(self);
        }
        if !xs.derives_from(Xs::AnyAtomicType) && xs != Xs::Numeric {
            todo!("We can only cast to atomic types or numeric types right now")
        }
        if self.schema_type() == xs {
            return Ok(self.clone());
        }
        match xs {
            Xs::UntypedAtomic => Ok(self.cast_to_untyped_atomic()),
            Xs::Numeric => self.cast_to_numeric(),
            Xs::String => Ok(self.cast_to_string()),
            Xs::Float => self.cast_to_float(),
            Xs::Double => self.cast_to_double(),
            Xs::Decimal => self.cast_to_decimal(),
            Xs::Integer => self.cast_to_integer(),
            Xs::Duration => self.cast_to_duration(),
            Xs::YearMonthDuration => self.cast_to_year_month_duration(),
            Xs::DayTimeDuration => self.cast_to_day_time_duration(),
            Xs::DateTime => self.cast_to_date_time(),
            Xs::DateTimeStamp => self.cast_to_date_time_stamp(),
            Xs::Time => self.cast_to_time(),
            Xs::Date => self.cast_to_date(),
            Xs::GYearMonth => self.cast_to_g_year_month(),
            Xs::GYear => self.cast_to_g_year(),
            Xs::GMonthDay => self.cast_to_g_month_day(),
            Xs::GDay => self.cast_to_g_day(),
            Xs::GMonth => self.cast_to_g_month(),
            Xs::Boolean => self.cast_to_boolean(),
            Xs::Base64Binary => self.cast_to_base64_binary(),
            Xs::HexBinary => self.cast_to_hex_binary(),
            Xs::AnyURI => self.cast_to_any_uri(),
            Xs::QName => self.cast_to_qname(static_context),

            // string subtypes
            Xs::NormalizedString => Ok(self.cast_to_normalized_string()),
            Xs::Token => Ok(self.cast_to_token()),
            Xs::Language => self.cast_to_language(),
            Xs::NMTOKEN => self.cast_to_nmtoken(),
            Xs::Name => self.cast_to_name(),
            Xs::NCName => self.cast_to_ncname(),
            Xs::ID => self.cast_to_id(),
            Xs::IDREF => self.cast_to_idref(),
            Xs::ENTITY => self.cast_to_entity(),

            // integer subtypes
            Xs::Long => self.cast_to_long(),
            Xs::Int => self.cast_to_int(),
            Xs::Short => self.cast_to_short(),
            Xs::Byte => self.cast_to_byte(),
            Xs::UnsignedLong => self.cast_to_unsigned_long(),
            Xs::UnsignedInt => self.cast_to_unsigned_int(),
            Xs::UnsignedShort => self.cast_to_unsigned_short(),
            Xs::UnsignedByte => self.cast_to_unsigned_byte(),
            Xs::NonPositiveInteger => self.cast_to_non_positive_integer(),
            Xs::NegativeInteger => self.cast_to_negative_integer(),
            Xs::NonNegativeInteger => self.cast_to_non_negative_integer(),
            Xs::PositiveInteger => self.cast_to_positive_integer(),

            _ => unreachable!(),
        }
    }

    pub(crate) fn cast_to_schema_type_of(
        self,
        other: &atomic::Atomic,
        context: &context::StaticContext,
    ) -> error::Result<atomic::Atomic> {
        self.cast_to_schema_type(other.schema_type(), context)
    }

    // if a derives from b, cast to b, otherwise vice versa
    pub(crate) fn cast_to_same_schema_type(
        self,
        other: atomic::Atomic,
        context: &context::StaticContext,
    ) -> error::Result<(atomic::Atomic, atomic::Atomic)> {
        if self.derives_from(&other) {
            let a = self.cast_to_schema_type_of(&other, context)?;
            Ok((a, other))
        } else if other.derives_from(&self) {
            let b = other.cast_to_schema_type_of(&self, context)?;
            Ok((self, b))
        } else {
            Err(error::Error::new(error::ErrorCode::XPTY0004))
        }
    }

    pub(crate) fn cast_to_boolean(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => {
                Err(error::Error::new(error::ErrorCode::XPTY0004))
            }
            atomic::Atomic::Untyped(s) | atomic::Atomic::String(_, s) => {
                Self::parse_atomic::<bool>(&s)
            }
            atomic::Atomic::Float(f) => Ok(atomic::Atomic::Boolean(!(f.is_nan() || f.is_zero()))),
            atomic::Atomic::Decimal(d) => Ok(atomic::Atomic::Boolean(!d.is_zero())),
            atomic::Atomic::Integer(_, i) => Ok(atomic::Atomic::Boolean(!i.is_zero())),
            atomic::Atomic::Double(d) => Ok(atomic::Atomic::Boolean(!(d.is_nan() || d.is_zero()))),
            atomic::Atomic::Boolean(_) => Ok(self.clone()),
            _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
        }
    }

    fn canonical_hex_binary(data: &[u8]) -> String {
        hex::encode_upper(data)
    }

    fn canonical_base64_binary(data: &[u8]) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(data)
    }

    fn cast_to_binary<F>(
        self,
        binary_type: atomic::BinaryType,
        decode: F,
    ) -> error::Result<atomic::Atomic>
    where
        F: Fn(&str) -> error::Result<Vec<u8>>,
    {
        match self {
            atomic::Atomic::String(atomic::StringType::AnyURI, _) => {
                Err(error::Error::new(error::ErrorCode::XPTY0004))
            }
            atomic::Atomic::String(_, s) | atomic::Atomic::Untyped(s) => {
                let s = s.as_ref();
                let s = whitespace_remove(s);
                let data = decode(&s)?;
                Ok(atomic::Atomic::Binary(binary_type, data.into()))
            }
            atomic::Atomic::Binary(_, data) => Ok(atomic::Atomic::Binary(binary_type, data)),
            _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
        }
    }

    pub(crate) fn cast_to_hex_binary(self) -> error::Result<atomic::Atomic> {
        self.cast_to_binary(atomic::BinaryType::Hex, |s: &str| {
            let data = hex::decode(s);
            data.map_err(|_| error::Error::new(error::ErrorCode::FORG0001))
        })
    }

    pub(crate) fn cast_to_base64_binary(self) -> error::Result<atomic::Atomic> {
        self.cast_to_binary(atomic::BinaryType::Base64, |s: &str| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(s)
                .map_err(|_| error::Error::new(error::ErrorCode::FORG0001))
        })
    }
}

pub(crate) struct Parsed<V>(pub(crate) V);

impl<V> Parsed<V> {
    pub(crate) fn into_inner(self) -> V {
        self.0
    }
}

impl FromStr for Parsed<bool> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_boolean(s)?))
    }
}

pub(crate) fn whitespace_replace(s: &str) -> String {
    // XML Schema whitespace: replace
    // all tab, linefeeds and carriage returns are replaced with a space
    // character
    s.replace(['\t', '\n', '\r'], " ")
}

pub(crate) fn whitespace_collapse(s: &str) -> String {
    // XML Schema whitespace: collapse
    // after doing a replace, collapse all space characters into a single
    // space character. Any space characters at the start or end of string
    // are then removed.
    whitespace_replace(s)
        .split_ascii_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn whitespace_remove(s: &str) -> String {
    // XML Schema whitespace: remove
    // after doing a replace, remove all space characters
    whitespace_replace(s).replace(' ', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    use ibig::ibig;
    use rust_decimal_macros::dec;

    #[test]
    fn test_canonical_decimal_is_integer() {
        assert_eq!(
            atomic::Atomic::Decimal(dec!(1.0).into()).into_canonical(),
            "1"
        );
    }

    #[test]
    fn test_canonical_decimal_is_decimal() {
        assert_eq!(
            atomic::Atomic::Decimal(dec!(1.5).into()).into_canonical(),
            "1.5"
        );
    }

    #[test]
    fn test_canonical_decimal_no_trailing_zeroes() {
        assert_eq!(
            atomic::Atomic::Decimal(dec!(1.50).into()).into_canonical(),
            "1.5"
        );
    }

    #[test]
    fn test_canonical_decimal_no_leading_zeroes() {
        assert_eq!(
            atomic::Atomic::Decimal(dec!(01.50).into()).into_canonical(),
            "1.5"
        );
    }

    #[test]
    fn test_canonical_decimal_single_leading_zero() {
        assert_eq!(
            atomic::Atomic::Decimal(dec!(0.50).into()).into_canonical(),
            "0.5"
        );
    }

    #[test]
    fn test_canonical_integer() {
        assert_eq!(
            atomic::Atomic::Integer(atomic::IntegerType::Integer, ibig!(15).into())
                .into_canonical(),
            "15"
        );
    }

    #[test]
    fn test_canonical_float_formatted_as_integer() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(15.0)).into_canonical(),
            "15"
        );
    }

    #[test]
    fn test_canonical_float_formatted_as_decimal() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(15.5)).into_canonical(),
            "15.5"
        );
    }

    #[test]
    fn test_canonical_float_formatted_as_float_big() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(1500000000000000f32)).into_canonical(),
            "1.5E15"
        );
    }

    #[test]
    fn test_canonical_formatted_as_float_small() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(0.000000000000001f32)).into_canonical(),
            "1.0E-15"
        );
    }

    #[test]
    fn test_canonical_float_zero() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(0.0)).into_canonical(),
            "0"
        );
    }

    #[test]
    fn test_canonical_float_minus_zero() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(-0.0)).into_canonical(),
            "-0"
        );
    }

    #[test]
    fn test_canonical_float_inf() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(f32::INFINITY)).into_canonical(),
            "INF"
        );
    }

    #[test]
    fn test_canonical_double_formatted_as_decimal() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(15.5)).into_canonical(),
            "15.5"
        );
    }

    #[test]
    fn test_whitespace_collapse() {
        let s = "\u{20}\u{09}\u{30}\u{0D}\u{0A}\u{30}\u{A0}\u{20}\u{20}";
        assert_eq!(whitespace_collapse(s), "0 0\u{A0}");
    }
}
