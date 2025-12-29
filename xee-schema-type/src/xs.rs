const XS_NAMESPACE: &str = "http://www.w3.org/2001/XMLSchema";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[repr(u16)]
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
    pub const fn to_u16(self) -> u16 {
        self as u16
    }

    pub const fn from_u16(value: u16) -> Option<Self> {
        use Xs::*;
        match value {
            x if x == AnyType as u16 => Some(AnyType),
            x if x == AnySimpleType as u16 => Some(AnySimpleType),
            x if x == Error as u16 => Some(Error),
            x if x == Untyped as u16 => Some(Untyped),
            x if x == AnyAtomicType as u16 => Some(AnyAtomicType),
            x if x == Numeric as u16 => Some(Numeric),
            x if x == String as u16 => Some(String),
            x if x == UntypedAtomic as u16 => Some(UntypedAtomic),
            x if x == Boolean as u16 => Some(Boolean),
            x if x == Decimal as u16 => Some(Decimal),
            x if x == NonPositiveInteger as u16 => Some(NonPositiveInteger),
            x if x == NegativeInteger as u16 => Some(NegativeInteger),
            x if x == NonNegativeInteger as u16 => Some(NonNegativeInteger),
            x if x == PositiveInteger as u16 => Some(PositiveInteger),
            x if x == Integer as u16 => Some(Integer),
            x if x == Long as u16 => Some(Long),
            x if x == Int as u16 => Some(Int),
            x if x == Short as u16 => Some(Short),
            x if x == Byte as u16 => Some(Byte),
            x if x == UnsignedLong as u16 => Some(UnsignedLong),
            x if x == UnsignedInt as u16 => Some(UnsignedInt),
            x if x == UnsignedShort as u16 => Some(UnsignedShort),
            x if x == UnsignedByte as u16 => Some(UnsignedByte),
            x if x == Float as u16 => Some(Float),
            x if x == Double as u16 => Some(Double),
            x if x == QName as u16 => Some(QName),
            x if x == Notation as u16 => Some(Notation),
            x if x == Duration as u16 => Some(Duration),
            x if x == YearMonthDuration as u16 => Some(YearMonthDuration),
            x if x == DayTimeDuration as u16 => Some(DayTimeDuration),
            x if x == Time as u16 => Some(Time),
            x if x == GYearMonth as u16 => Some(GYearMonth),
            x if x == GYear as u16 => Some(GYear),
            x if x == GMonthDay as u16 => Some(GMonthDay),
            x if x == GMonth as u16 => Some(GMonth),
            x if x == GDay as u16 => Some(GDay),
            x if x == Base64Binary as u16 => Some(Base64Binary),
            x if x == HexBinary as u16 => Some(HexBinary),
            x if x == AnyURI as u16 => Some(AnyURI),
            x if x == DateTime as u16 => Some(DateTime),
            x if x == DateTimeStamp as u16 => Some(DateTimeStamp),
            x if x == Date as u16 => Some(Date),
            x if x == NormalizedString as u16 => Some(NormalizedString),
            x if x == Token as u16 => Some(Token),
            x if x == Language as u16 => Some(Language),
            x if x == NMTOKEN as u16 => Some(NMTOKEN),
            x if x == Name as u16 => Some(Name),
            x if x == NCName as u16 => Some(NCName),
            x if x == ID as u16 => Some(ID),
            x if x == IDREF as u16 => Some(IDREF),
            x if x == ENTITY as u16 => Some(ENTITY),
            _ => None,
        }
    }

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
