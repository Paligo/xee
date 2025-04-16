const XS_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Xs {
    AnyType,
    AnySimpleType,
    Error,
    Untyped,
    AnyAtomicType,
    Numeric,
    String,
    UntypedAtomic,
    Boolean,
    Decimal,
    NonPositiveInteger,
    NegativeInteger,
    NonNegativeInteger,
    PositiveInteger,
    Integer,
    Long,
    Int,
    Short,
    Byte,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
    Float,
    Double,
    QName,
    Notation,
    Duration,
    YearMonthDuration,
    DayTimeDuration,
    Time,
    GYearMonth,
    GYear,
    GMonthDay,
    GMonth,
    GDay,
    Base64Binary,
    HexBinary,
    AnyURI,
    DateTime,
    DateTimeStamp,
    Date,
    NormalizedString,
    Token,
    Language,
    NMTOKEN,
    Name,
    NCName,
    ID,
    IDREF,
    ENTITY,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustInfo {
    rust_name: String,
    as_ref: bool,
}

impl RustInfo {
    fn new(rust_name: &str) -> Self {
        Self {
            rust_name: rust_name.to_string(),
            as_ref: false,
        }
    }

    fn as_ref(rust_name: &str) -> Self {
        Self {
            rust_name: rust_name.to_string(),
            as_ref: true,
        }
    }

    pub fn rust_name(&self) -> &str {
        &self.rust_name
    }

    pub fn is_reference(&self) -> bool {
        self.as_ref
    }
}

impl Xs {
    pub fn by_name(namespace: &str, local_name: &str) -> Option<Self> {
        if namespace == XS_NAMESPACE {
            Xs::by_local_name(local_name)
        } else {
            None
        }
    }

    pub fn by_local_name(local_name: &str) -> Option<Self> {
        use Xs::*;
        let xs = match local_name {
            "anyType" => AnyType,
            "anySimpleType" => AnySimpleType,
            "error" => Error,
            "untyped" => Untyped,
            "anyAtomicType" => AnyAtomicType,
            "numeric" => Numeric,
            "string" => String,
            "untypedAtomic" => UntypedAtomic,
            "boolean" => Boolean,
            "decimal" => Decimal,
            "nonPositiveInteger" => NonPositiveInteger,
            "negativeInteger" => NegativeInteger,
            "nonNegativeInteger" => NonNegativeInteger,
            "positiveInteger" => PositiveInteger,
            "integer" => Integer,
            "long" => Long,
            "int" => Int,
            "short" => Short,
            "byte" => Byte,
            "unsignedLong" => UnsignedLong,
            "unsignedInt" => UnsignedInt,
            "unsignedShort" => UnsignedShort,
            "unsignedByte" => UnsignedByte,
            "float" => Float,
            "double" => Double,
            "QName" => QName,
            "NOTATION" => Notation,
            "duration" => Duration,
            "yearMonthDuration" => YearMonthDuration,
            "dayTimeDuration" => DayTimeDuration,
            "time" => Time,
            "gYearMonth" => GYearMonth,
            "gYear" => GYear,
            "gMonthDay" => GMonthDay,
            "gMonth" => GMonth,
            "gDay" => GDay,
            "base64Binary" => Base64Binary,
            "hexBinary" => HexBinary,
            "anyURI" => AnyURI,
            "dateTime" => DateTime,
            "dateTimeStamp" => DateTimeStamp,
            "date" => Date,
            "normalizedString" => NormalizedString,
            "token" => Token,
            "language" => Language,
            "NMTOKEN" => NMTOKEN,
            "Name" => Name,
            "NCName" => NCName,
            "ID" => ID,
            "IDREF" => IDREF,
            "ENTITY" => ENTITY,
            _ => return None,
        };
        Some(xs)
    }

    pub fn namespace() -> &'static str {
        XS_NAMESPACE
    }

    pub fn local_name(&self) -> &str {
        use Xs::*;
        match self {
            AnyType => "anyType",
            AnySimpleType => "anySimpleType",
            Error => "error",
            Untyped => "untyped",
            AnyAtomicType => "anyAtomicType",
            Numeric => "numeric",
            String => "string",
            UntypedAtomic => "untypedAtomic",
            Boolean => "boolean",
            Decimal => "decimal",
            NonPositiveInteger => "nonPositiveInteger",
            NegativeInteger => "negativeInteger",
            NonNegativeInteger => "nonNegativeInteger",
            PositiveInteger => "positiveInteger",
            Integer => "integer",
            Long => "long",
            Int => "int",
            Short => "short",
            Byte => "byte",
            UnsignedLong => "unsignedLong",
            UnsignedInt => "unsignedInt",
            UnsignedShort => "unsignedShort",
            UnsignedByte => "unsignedByte",
            Float => "float",
            Double => "double",
            QName => "QName",
            Notation => "NOTATION",
            Duration => "duration",
            YearMonthDuration => "yearMonthDuration",
            DayTimeDuration => "dayTimeDuration",
            Time => "time",
            GYearMonth => "gYearMonth",
            GYear => "gYear",
            GMonthDay => "gMonthDay",
            GMonth => "gMonth",
            GDay => "gDay",
            Base64Binary => "base64Binary",
            HexBinary => "hexBinary",
            AnyURI => "anyURI",
            DateTime => "dateTime",
            DateTimeStamp => "dateTimeStamp",
            Date => "date",
            NormalizedString => "normalizedString",
            Token => "token",
            Language => "language",
            NMTOKEN => "NMTOKEN",
            Name => "Name",
            NCName => "NCName",
            ID => "ID",
            IDREF => "IDREF",
            ENTITY => "ENTITY",
        }
    }

    pub fn parent(&self) -> Option<Xs> {
        use Xs::*;
        match self {
            AnyType => None,
            AnySimpleType => Some(AnyType),
            Error => None,
            Untyped => Some(AnyType),
            AnyAtomicType => Some(AnySimpleType),
            UntypedAtomic => Some(AnyAtomicType),
            Numeric => Some(AnySimpleType),
            String => Some(AnyAtomicType),
            Boolean => Some(AnyAtomicType),
            Float => Some(AnyAtomicType),
            Double => Some(AnyAtomicType),
            Decimal => Some(AnyAtomicType),
            Integer => Some(Decimal),
            NonPositiveInteger => Some(Integer),
            NegativeInteger => Some(NonPositiveInteger),
            Long => Some(Integer),
            Int => Some(Long),
            Short => Some(Int),
            Byte => Some(Short),
            NonNegativeInteger => Some(Integer),
            PositiveInteger => Some(NonNegativeInteger),
            UnsignedLong => Some(NonNegativeInteger),
            UnsignedInt => Some(UnsignedLong),
            UnsignedShort => Some(UnsignedInt),
            UnsignedByte => Some(UnsignedShort),
            QName => Some(AnyAtomicType),
            Notation => Some(AnyAtomicType),
            Duration => Some(AnyAtomicType),
            YearMonthDuration => Some(Duration),
            DayTimeDuration => Some(Duration),
            Time => Some(AnyAtomicType),
            GYearMonth => Some(AnyAtomicType),
            GYear => Some(AnyAtomicType),
            GMonthDay => Some(AnyAtomicType),
            GMonth => Some(AnyAtomicType),
            GDay => Some(AnyAtomicType),
            Base64Binary => Some(AnyAtomicType),
            HexBinary => Some(AnyAtomicType),
            AnyURI => Some(AnyAtomicType),
            DateTime => Some(AnyAtomicType),
            DateTimeStamp => Some(DateTime),
            Date => Some(AnyAtomicType),
            NormalizedString => Some(String),
            Token => Some(NormalizedString),
            Language => Some(Token),
            NMTOKEN => Some(Token),
            Name => Some(Token),
            NCName => Some(Name),
            ID => Some(NCName),
            IDREF => Some(NCName),
            ENTITY => Some(NCName),
        }
    }

    #[inline]
    pub fn derives_from(&self, other: Xs) -> bool {
        if self == &other {
            return true;
        }
        let mut xs = *self;
        while let Some(parent) = xs.parent() {
            if parent == other {
                return true;
            }
            xs = parent;
        }
        false
    }

    #[inline]
    pub fn matches(&self, other: Xs) -> bool {
        if other != Xs::Numeric {
            return self == &other;
        }
        self.derives_from(Xs::Double)
            || self.derives_from(Xs::Float)
            || self.derives_from(Xs::Decimal)
    }

    pub fn rust_info(&self) -> Option<RustInfo> {
        use Xs::*;
        match self {
            AnyType => None,
            AnySimpleType => None,
            Error => None,
            Untyped => None,
            AnyAtomicType => None,
            UntypedAtomic => Some(RustInfo::as_ref("String")),
            Numeric => None,
            String => Some(RustInfo::as_ref("String")),
            Float => Some(RustInfo::new("f32")),
            Double => Some(RustInfo::new("f64")),
            Decimal => Some(RustInfo::new("rust_decimal::Decimal")),
            Integer => Some(RustInfo::new("ibig::IBig")),
            Duration => Some(RustInfo::new("crate::atomic::Duration")),
            YearMonthDuration => Some(RustInfo::new("crate::atomic::YearMonthDuration")),
            DayTimeDuration => Some(RustInfo::new("chrono::Duration")),
            DateTime => Some(RustInfo::new("crate::atomic::NaiveDateTimeWithOffset")),
            DateTimeStamp => Some(RustInfo::new("chrono::DateTime<chrono::FixedOffset>>")),
            Time => Some(RustInfo::new("crate::atomic::NaiveTimeWithOffset")),
            Date => Some(RustInfo::new("crate::atomic::NaiveDateWithOffset")),
            GYearMonth => Some(RustInfo::new("crate::atomic::GYearMonth")),
            GYear => Some(RustInfo::new("crate::atomic::GYear")),
            GMonthDay => Some(RustInfo::new("crate::atomic::GMonthDay")),
            GDay => Some(RustInfo::new("crate::atomic::GDay")),
            GMonth => Some(RustInfo::new("crate::atomic::GMonth")),
            Boolean => Some(RustInfo::new("bool")),
            Base64Binary => Some(RustInfo::as_ref("Vec<u8>")),
            HexBinary => Some(RustInfo::as_ref("Vec<u8>")),
            QName => Some(RustInfo::new("xee_xpath_ast::ast::Name")),
            Notation => None,

            // integer types; are these correct or should we use ibig everywhere?
            NonPositiveInteger => Some(RustInfo::new("i64")),
            NegativeInteger => Some(RustInfo::new("i64")),
            Long => Some(RustInfo::new("i64")),
            Int => Some(RustInfo::new("i32")),
            Short => Some(RustInfo::new("i16")),
            Byte => Some(RustInfo::new("i8")),
            NonNegativeInteger => Some(RustInfo::new("u64")),
            PositiveInteger => Some(RustInfo::new("u64")),
            UnsignedLong => Some(RustInfo::new("u64")),
            UnsignedInt => Some(RustInfo::new("u32")),
            UnsignedShort => Some(RustInfo::new("u16")),
            UnsignedByte => Some(RustInfo::new("u8")),

            // string types (and AnyURI)
            NormalizedString => Some(RustInfo::as_ref("String")),
            Token => Some(RustInfo::as_ref("String")),
            Language => Some(RustInfo::as_ref("String")),
            NMTOKEN => Some(RustInfo::as_ref("String")),
            Name => Some(RustInfo::as_ref("String")),
            NCName => Some(RustInfo::as_ref("String")),
            ID => Some(RustInfo::as_ref("String")),
            IDREF => Some(RustInfo::as_ref("String")),
            ENTITY => Some(RustInfo::as_ref("String")),
            AnyURI => Some(RustInfo::as_ref("String")),
        }
    }
}

impl std::fmt::Display for Xs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "xs:{}", self.local_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derives_from() {
        assert!(Xs::Integer.derives_from(Xs::Integer));
        assert!(Xs::Integer.derives_from(Xs::Decimal));
        assert!(Xs::Integer.derives_from(Xs::AnyAtomicType));
        assert!(Xs::Integer.derives_from(Xs::AnySimpleType));
        assert!(Xs::Integer.derives_from(Xs::AnyType));
        assert!(Xs::Byte.derives_from(Xs::AnyAtomicType));
    }
}
