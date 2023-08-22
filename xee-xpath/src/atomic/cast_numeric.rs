use ibig::{ibig, IBig};
use num_traits::Float;
use ordered_float::OrderedFloat;
use rust_decimal::prelude::*;
use std::rc::Rc;

use xee_schema_type::Xs;

use crate::atomic;
use crate::error;

use super::cast::Parsed;

impl atomic::Atomic {
    pub(crate) fn canonical_float<F>(f: F) -> String
    where
        F: Float
            + TryInto<Decimal, Error = rust_decimal::Error>
            + lexical::ToLexicalWithOptions<Options = lexical::WriteFloatOptions>
            + num::Signed,
    {
        // https://www.w3.org/TR/xpath-functions-31/#casting-to-string
        // If SV has an absolute value that is greater than or equal to
        // 0.000001 (one millionth) and less than 1000000 (one
        // million), then the value is converted to an xs:decimal and
        // the resulting xs:decimal is converted to an xs:string
        let abs_f = f.abs();
        let minimum: F = num::cast(0.000001).unwrap();
        let maximum: F = num::cast(1000000.0).unwrap();
        if abs_f >= minimum && abs_f < maximum {
            // TODO: is this the right conversion?
            let d: Decimal = f.try_into().unwrap();
            atomic::Atomic::Decimal(Rc::new(d)).into_canonical()
        } else {
            if f.is_zero() {
                if f.is_negative() {
                    return "-0".to_string();
                } else {
                    return "0".to_string();
                }
            }
            let options = lexical::WriteFloatOptionsBuilder::new()
                .exponent(b'E')
                .inf_string(Some(b"INF"))
                .build()
                .unwrap();
            lexical::to_string_with_options::<_, { lexical::format::XML }>(f, &options)
        }
    }

    pub(crate) fn parse_decimal(s: &str) -> error::Result<Decimal> {
        if s.contains('_') {
            return Err(error::Error::FORG0001);
        }
        s.parse::<Decimal>().map_err(|_| error::Error::FORG0001)
    }

    pub(crate) fn parse_integer_number<V>(s: &str) -> error::Result<V>
    where
        V: lexical::FromLexical,
    {
        lexical::parse::<V, _>(s).map_err(|_| error::Error::FORG0001)
    }

    // we can't use lexical::parse_float_options::XML as it doesn't allow INF
    // which is allowed by the XML Schema spec

    pub(crate) fn parse_float(s: &str) -> error::Result<f32> {
        let options = lexical::ParseFloatOptionsBuilder::new()
            .inf_string(Some(b"INF"))
            .build()
            .unwrap();
        lexical::parse_with_options::<f32, _, { lexical::format::XML }>(s, &options)
            .map_err(|_| error::Error::FORG0001)
    }

    pub(crate) fn parse_double(s: &str) -> error::Result<f64> {
        let options = lexical::ParseFloatOptionsBuilder::new()
            .inf_string(Some(b"INF"))
            .build()
            .unwrap();
        lexical::parse_with_options::<f64, _, { lexical::format::XML }>(s, &options)
            .map_err(|_| error::Error::FORG0001)
    }

    pub(crate) fn cast_to_float(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::Untyped(s) => Self::parse_atomic::<f32>(&s),
            atomic::Atomic::String(_, s) => Self::parse_atomic::<f32>(&s),
            atomic::Atomic::Float(_) => Ok(self.clone()),
            // TODO: this should implement the rule in 19.1.2.1
            atomic::Atomic::Double(OrderedFloat(d)) => {
                // https://www.w3.org/TR/xpath-functions-31/#casting-to-numerics
                // specifies a complex rule that involves the ranges for e of
                // -149 to 104. As far as I can tell Rust uses a different
                // range, -125 to 128. Both refer to IEEE 754-2008, so I'm
                // confused. For now I'm keeping the simple implementation
                // below, which may not behave exactly as per the conversion
                // rules at the extremes of the ranges.
                Ok(atomic::Atomic::Float(OrderedFloat(d as f32)))
            }
            atomic::Atomic::Decimal(_) | atomic::Atomic::Integer(_, _) => {
                Self::parse_atomic::<f32>(&self.into_canonical())
            }
            // TODO: any type of integer needs to cast to string first,
            // then to that from float
            atomic::Atomic::Boolean(b) => {
                if b {
                    Ok(atomic::Atomic::Float(OrderedFloat(1.0)))
                } else {
                    Ok(atomic::Atomic::Float(OrderedFloat(0.0)))
                }
            }
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_double(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::Untyped(s) => Self::parse_atomic::<f64>(&s),
            atomic::Atomic::String(_, s) => Self::parse_atomic::<f64>(&s),
            atomic::Atomic::Float(OrderedFloat(f)) => {
                Ok(atomic::Atomic::Double(OrderedFloat(f as f64)))
            }
            atomic::Atomic::Double(_) => Ok(self.clone()),
            atomic::Atomic::Decimal(_) | atomic::Atomic::Integer(_, _) => {
                Self::parse_atomic::<f64>(&self.into_canonical())
            }
            atomic::Atomic::Boolean(b) => {
                if b {
                    Ok(atomic::Atomic::Double(OrderedFloat(1.0)))
                } else {
                    Ok(atomic::Atomic::Double(OrderedFloat(0.0)))
                }
            }
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_decimal(self) -> error::Result<atomic::Atomic> {
        match self {
            atomic::Atomic::Untyped(s) => Self::parse_atomic::<Decimal>(&s),
            atomic::Atomic::String(_, s) => Self::parse_atomic::<Decimal>(&s),
            atomic::Atomic::Float(OrderedFloat(f)) => {
                if f.is_nan() || f.is_infinite() {
                    return Err(error::Error::FOCA0002);
                }

                Ok(atomic::Atomic::Decimal(Rc::new(
                    Decimal::try_from(f).map_err(|_| error::Error::FOCA0001)?,
                )))
            }
            atomic::Atomic::Double(OrderedFloat(f)) => {
                if f.is_nan() || f.is_infinite() {
                    return Err(error::Error::FOCA0002);
                }

                Ok(atomic::Atomic::Decimal(Rc::new(
                    Decimal::try_from(f).map_err(|_| error::Error::FOCA0001)?,
                )))
            }
            atomic::Atomic::Decimal(_) => Ok(self.clone()),
            atomic::Atomic::Integer(_, i) => Ok(atomic::Atomic::Decimal(
                // rust decimal doesn't support arbitrary precision integers,
                // so we fail some conversions
                Rc::new(
                    Decimal::try_from_i128_with_scale(
                        // if this is bigger than an i128, it certainly can't be
                        // an integer
                        i.as_ref().try_into().map_err(|_| error::Error::Overflow)?,
                        0,
                    )
                    .map_err(|_| error::Error::Overflow)?,
                ),
            )),
            atomic::Atomic::Boolean(b) => {
                if b {
                    Ok(atomic::Atomic::Decimal(Rc::new(Decimal::from(1))))
                } else {
                    Ok(atomic::Atomic::Decimal(Rc::new(Decimal::from(0))))
                }
            }
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn cast_to_integer(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::Integer,
            Rc::new(self.cast_to_integer_value::<IBig>()?),
        ))
    }

    pub(crate) fn cast_to_long(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::Long,
            Rc::new(self.cast_to_integer_value::<i64>()?.into()),
        ))
    }

    pub(crate) fn cast_to_int(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::Int,
            Rc::new(self.cast_to_integer_value::<i32>()?.into()),
        ))
    }

    pub(crate) fn cast_to_short(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::Short,
            Rc::new(self.cast_to_integer_value::<i16>()?.into()),
        ))
    }

    pub(crate) fn cast_to_byte(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::Byte,
            Rc::new(self.cast_to_integer_value::<i8>()?.into()),
        ))
    }

    pub(crate) fn cast_to_unsigned_long(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::UnsignedLong,
            Rc::new(self.cast_to_integer_value::<u64>()?.into()),
        ))
    }

    pub(crate) fn cast_to_unsigned_int(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::UnsignedInt,
            Rc::new(self.cast_to_integer_value::<u32>()?.into()),
        ))
    }

    pub(crate) fn cast_to_unsigned_short(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::UnsignedShort,
            Rc::new(self.cast_to_integer_value::<u16>()?.into()),
        ))
    }

    pub(crate) fn cast_to_unsigned_byte(self) -> error::Result<atomic::Atomic> {
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::UnsignedByte,
            Rc::new(self.cast_to_integer_value::<u8>()?.into()),
        ))
    }

    pub(crate) fn cast_to_non_positive_integer(self) -> error::Result<atomic::Atomic> {
        let i = self.cast_to_integer_value::<IBig>()?;
        if i > ibig!(0) {
            return Err(error::Error::FOCA0003);
        }
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::NonPositiveInteger,
            Rc::new(i),
        ))
    }

    pub(crate) fn cast_to_negative_integer(self) -> error::Result<atomic::Atomic> {
        let i = self.cast_to_integer_value::<IBig>()?;
        if i >= ibig!(0) {
            return Err(error::Error::FOCA0003);
        }
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::NegativeInteger,
            Rc::new(i),
        ))
    }

    pub(crate) fn cast_to_non_negative_integer(self) -> error::Result<atomic::Atomic> {
        let i = self.cast_to_integer_value::<IBig>()?;
        if i < ibig!(0) {
            return Err(error::Error::FOCA0003);
        }
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::NonNegativeInteger,
            Rc::new(i),
        ))
    }

    pub(crate) fn cast_to_positive_integer(self) -> error::Result<atomic::Atomic> {
        let i = self.cast_to_integer_value::<IBig>()?;
        if i <= ibig!(0) {
            return Err(error::Error::FOCA0003);
        }
        Ok(atomic::Atomic::Integer(
            atomic::IntegerType::PositiveInteger,
            Rc::new(i),
        ))
    }

    pub(crate) fn cast_to_integer_value<V>(self) -> error::Result<V>
    where
        V: TryFrom<IBig>
            + TryFrom<i128>
            + TryFrom<i64>
            + TryFrom<i32>
            + TryFrom<i16>
            + TryFrom<i8>
            + TryFrom<u64>
            + TryFrom<u32>
            + TryFrom<u16>
            + TryFrom<u8>,
        Parsed<V>: FromStr<Err = error::Error>,
    {
        match self {
            atomic::Atomic::Untyped(s) => Ok(s.parse::<Parsed<V>>()?.into_inner()),
            atomic::Atomic::String(_, s) => Ok(s.parse::<Parsed<V>>()?.into_inner()),
            atomic::Atomic::Float(OrderedFloat(f)) => {
                if f.is_nan() | f.is_infinite() {
                    return Err(error::Error::FOCA0002);
                }
                // we first go to a decimal. Any larger number we won't be able to
                // express, even though bigint strictly speaking could handle it.
                // But converting a float to a bigint directly isn't possible.
                let d: Decimal = f.trunc().try_into().map_err(|_| error::Error::FOCA0003)?;
                let i: i128 = d.try_into().map_err(|_| error::Error::FOCA0003)?;
                let i: V = i.try_into().map_err(|_| error::Error::FOCA0003)?;
                Ok(i)
            }
            atomic::Atomic::Double(OrderedFloat(d)) => {
                if d.is_nan() | d.is_infinite() {
                    return Err(error::Error::FOCA0002);
                }
                // we first go to a decimal. Any larger number we won't be able to
                // express, even though bigint strictly speaking could handle it.
                // But converting a float to a bigint directly isn't possible.
                let d: Decimal = d.trunc().try_into().map_err(|_| error::Error::FOCA0003)?;
                let i: i128 = d.try_into().map_err(|_| error::Error::FOCA0003)?;
                let i: V = i.try_into().map_err(|_| error::Error::FOCA0003)?;
                Ok(i)
            }
            atomic::Atomic::Decimal(d) => decimal_to_integer(d),
            atomic::Atomic::Integer(_, i) => {
                let i: V = i
                    .as_ref()
                    .clone()
                    .try_into()
                    .map_err(|_| error::Error::FOCA0003)?;
                Ok(i)
            }
            atomic::Atomic::Boolean(b) => {
                let v: V = if b {
                    1.try_into().map_err(|_| error::Error::FOCA0003)?
                } else {
                    0.try_into().map_err(|_| error::Error::FOCA0003)?
                };
                Ok(v)
            }
            _ => Err(error::Error::Type),
        }
    }

    pub(crate) fn type_promote(self, xs: Xs) -> error::Result<atomic::Atomic> {
        // Section B.1 type promotion
        let schema_type = self.schema_type();
        if xs == Xs::Double
            && (schema_type.derives_from(Xs::Float) || schema_type.derives_from(Xs::Decimal))
        {
            return self.cast_to_double();
        }

        if xs == Xs::Float && schema_type.derives_from(Xs::Decimal) {
            return self.cast_to_float();
        }
        // TODO: handle xs:anyURI
        Ok(self)
    }
}

// shared casting logic for binary operations; both comparison and arithmetic use
// this
pub(crate) fn cast_numeric_binary<F>(
    a: atomic::Atomic,
    b: atomic::Atomic,
    non_numeric: F,
) -> error::Result<(atomic::Atomic, atomic::Atomic)>
where
    F: Fn(atomic::Atomic, atomic::Atomic) -> error::Result<(atomic::Atomic, atomic::Atomic)>,
{
    use atomic::Atomic::*;
    match (&a, &b) {
        // the concrete types need no casting
        (Float(_), Float(_))
        | (Double(_), Double(_))
        | (Decimal(_), Decimal(_))
        | (Integer(_, _), Integer(_, _)) => Ok((a, b)),
        // Cast a to a float
        (Decimal(_), Float(_)) | (Integer(_, _), Float(_)) => Ok((a.cast_to_float()?, b)),
        // Cast b to a float
        (Float(_), Decimal(_)) | (Float(_), Integer(_, _)) => Ok((a, b.cast_to_float()?)),
        // Cast a to a double
        (Decimal(_), Double(_)) | (Integer(_, _), Double(_)) | (Float(_), Double(_)) => {
            Ok((a.cast_to_double()?, b))
        }
        // Cast b to a double
        (Double(_), Decimal(_)) | (Double(_), Integer(_, _)) | (Double(_), Float(_)) => {
            Ok((a, b.cast_to_double()?))
        }
        // Cast integer to decimal
        (Decimal(_), Integer(_, _)) => Ok((a, b.cast_to_decimal()?)),
        (Integer(_, _), Decimal(_)) => Ok((a.cast_to_decimal()?, b)),
        // Non-numeric types are handled by the function passed in
        _ => non_numeric(a, b),
    }
}

pub(crate) fn cast_numeric(
    a: atomic::Atomic,
    b: atomic::Atomic,
) -> error::Result<(atomic::Atomic, atomic::Atomic)> {
    use atomic::Atomic::*;

    match (&a, &b) {
        // Cast a to a float
        (Decimal(_), Float(_)) | (Integer(_, _), Float(_)) => Ok((a.cast_to_float()?, b)),
        // Cast b to a float
        (Float(_), Decimal(_)) | (Float(_), Integer(_, _)) => Ok((a, b.cast_to_float()?)),
        // Cast a to a double
        (Decimal(_), Double(_)) | (Integer(_, _), Double(_)) | (Float(_), Double(_)) => {
            Ok((a.cast_to_double()?, b))
        }
        // Cast b to a double
        (Double(_), Decimal(_)) | (Double(_), Integer(_, _)) | (Double(_), Float(_)) => {
            Ok((a, b.cast_to_double()?))
        }
        // Cast integer to decimal
        (Decimal(_), Integer(_, _)) => Ok((a, b.cast_to_decimal()?)),
        (Integer(_, _), Decimal(_)) => Ok((a.cast_to_decimal()?, b)),
        // otherwise, we don't cast
        _ => Ok((a, b)),
    }
}

pub(crate) fn decimal_to_integer<V>(d: Rc<Decimal>) -> error::Result<V>
where
    V: TryFrom<IBig>
        + TryFrom<i128>
        + TryFrom<i64>
        + TryFrom<i32>
        + TryFrom<i16>
        + TryFrom<i8>
        + TryFrom<u64>
        + TryFrom<u32>
        + TryFrom<u16>
        + TryFrom<u8>,
{
    // we first go to a i128; this accomodates the largest Decimal
    let i: i128 = d.trunc().try_into().map_err(|_| error::Error::FOCA0003)?;
    // then we convert this into the target integer type
    let i: V = i.try_into().map_err(|_| error::Error::FOCA0003)?;
    Ok(i)
}

impl FromStr for Parsed<Decimal> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_decimal(s)?))
    }
}

impl FromStr for Parsed<IBig> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(s.parse().map_err(|_| error::Error::FOCA0003)?))
    }
}

impl FromStr for Parsed<i64> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<i32> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<i16> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<i8> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<u64> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<u32> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<u16> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<u8> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_integer_number(s)?))
    }
}

impl FromStr for Parsed<f64> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_double(s)?))
    }
}

impl FromStr for Parsed<f32> {
    type Err = error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Parsed(atomic::Atomic::parse_float(s)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use ibig::ibig;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_decimal() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<Decimal>("1.0").unwrap(),
            atomic::Atomic::Decimal(Rc::new(dec!(1.0)))
        );
    }

    #[test]
    fn test_parse_decimal_no_underscore() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<Decimal>("1_000.0"),
            Err(error::Error::FORG0001)
        );
    }

    #[test]
    fn test_parse_integer() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<IBig>("1").unwrap(),
            atomic::Atomic::Integer(atomic::IntegerType::Integer, ibig!(1).into())
        );
    }

    #[test]
    fn test_parse_long() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<i64>("1").unwrap(),
            atomic::Atomic::Integer(atomic::IntegerType::Long, ibig!(1).into())
        );
    }

    #[test]
    fn test_parse_integer_no_underscore() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<i64>("1_000"),
            Err(error::Error::FORG0001)
        );
    }

    #[test]
    fn test_parse_double() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("1.0").unwrap(),
            atomic::Atomic::Double(OrderedFloat(1.0))
        );
    }

    #[test]
    fn test_parse_double_exponent() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("1.0e10").unwrap(),
            atomic::Atomic::Double(OrderedFloat(1.0e10))
        );
    }

    #[test]
    fn test_parse_double_exponent_capital() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("1.0E10").unwrap(),
            atomic::Atomic::Double(OrderedFloat(1.0e10))
        );
    }

    #[test]
    fn test_parse_double_inf() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("INF").unwrap(),
            atomic::Atomic::Double(OrderedFloat(f64::INFINITY))
        );
    }

    #[test]
    fn test_parse_double_minus_inf() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("-INF").unwrap(),
            atomic::Atomic::Double(OrderedFloat(-f64::INFINITY))
        );
    }

    #[test]
    fn test_parse_double_nan() {
        let a = atomic::Atomic::parse_atomic::<f64>("NaN").unwrap();
        match a {
            atomic::Atomic::Double(OrderedFloat(f)) => assert!(f.is_nan()),
            _ => panic!("Expected double"),
        }
    }

    #[test]
    fn test_parse_double_invalid_nan() {
        assert_eq!(
            atomic::Atomic::parse_atomic::<f64>("NAN"),
            Err(error::Error::FORG0001)
        );
    }

    #[test]
    fn test_cast_double_to_float_inf() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(f64::INFINITY))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(f32::INFINITY))
        );
    }

    #[test]
    fn test_cast_double_to_float_negative_inf() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(f64::NEG_INFINITY))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(f32::NEG_INFINITY))
        );
    }

    #[test]
    fn test_cast_double_to_float_nan() {
        let a = atomic::Atomic::Double(OrderedFloat(f64::NAN))
            .cast_to_float()
            .unwrap();
        match a {
            atomic::Atomic::Float(OrderedFloat(f)) => assert!(f.is_nan()),
            _ => panic!("Expected float"),
        }
    }

    #[test]
    fn test_cast_double_to_float_zero() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(0.0))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(0.0))
        );
    }

    #[test]
    fn test_cast_double_to_float_negative_zero() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(-0.0))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(-0.0))
        );
    }

    #[test]
    fn test_cast_double_to_float_too_big() {
        // TODO: are the boundaries exactly as in spec?
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(f32::MAX as f64 * 1.1))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(f32::INFINITY))
        );
    }

    #[test]
    fn test_cast_double_to_float_too_small() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(1e-150))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(0.0))
        );
    }

    #[test]
    fn test_cast_double_to_float_not_too_small() {
        assert_eq!(
            atomic::Atomic::Double(OrderedFloat(1e-5))
                .cast_to_float()
                .unwrap(),
            atomic::Atomic::Float(OrderedFloat(1e-5))
        );
    }

    #[test]
    fn test_cast_int_to_decimal() {
        assert_eq!(
            atomic::Atomic::Integer(atomic::IntegerType::Integer, ibig!(15).into())
                .cast_to_decimal()
                .unwrap(),
            atomic::Atomic::Decimal(Rc::new(dec!(15)))
        );
    }

    #[test]
    fn test_cast_float_to_decimal() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(15.5))
                .cast_to_decimal()
                .unwrap(),
            atomic::Atomic::Decimal(Rc::new(dec!(15.5)))
        );
    }

    #[test]
    fn test_cast_inf_float_to_decimal() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(f32::INFINITY)).cast_to_decimal(),
            Err(error::Error::FOCA0002)
        );
    }

    #[test]
    fn test_cast_huge_float_to_decimal() {
        assert_eq!(
            atomic::Atomic::Float(OrderedFloat(1.0e30)).cast_to_decimal(),
            Err(error::Error::FOCA0001)
        );
    }

    #[test]
    fn test_cast_short_to_short() {
        assert_eq!(
            atomic::Atomic::Integer(atomic::IntegerType::Short, ibig!(15).into())
                .cast_to_short()
                .unwrap(),
            atomic::Atomic::Integer(atomic::IntegerType::Short, ibig!(15).into())
        );
    }
}
