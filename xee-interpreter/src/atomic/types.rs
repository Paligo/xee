use xee_schema_type::Xs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntegerType {
    Integer,
    NonPositiveInteger,
    NegativeInteger,
    NonNegativeInteger,
    PositiveInteger,
    Long,
    Int,
    Short,
    Byte,
    UnsignedLong,
    UnsignedInt,
    UnsignedShort,
    UnsignedByte,
}

impl IntegerType {
    pub(crate) fn schema_type(&self) -> Xs {
        match self {
            Self::Integer => Xs::Integer,
            Self::Long => Xs::Long,
            Self::Int => Xs::Int,
            Self::Short => Xs::Short,
            Self::Byte => Xs::Byte,
            Self::UnsignedLong => Xs::UnsignedLong,
            Self::UnsignedInt => Xs::UnsignedInt,
            Self::UnsignedShort => Xs::UnsignedShort,
            Self::UnsignedByte => Xs::UnsignedByte,
            Self::NonPositiveInteger => Xs::NonPositiveInteger,
            Self::NegativeInteger => Xs::NegativeInteger,
            Self::NonNegativeInteger => Xs::NonNegativeInteger,
            Self::PositiveInteger => Xs::PositiveInteger,
        }
    }
}

/// The types of string supported as atomic values.
#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StringType {
    /// xs:string
    String,
    /// xs:normalizedString
    NormalizedString,
    /// xs:token
    Token,
    /// xs:language
    Language,
    /// xs:NMTOKEN
    NMTOKEN,
    /// xs:Name
    Name,
    /// xs:NCName
    NCName,
    /// xs:ID
    ID,
    /// xs:IDREF
    IDREF,
    /// xs:ENTITY
    ENTITY,
    // the qt3 tests make the assumption AnyURI is a type of string
    /// xs:anyURI
    AnyURI,
}

impl StringType {
    pub(crate) fn schema_type(&self) -> Xs {
        match self {
            Self::String => Xs::String,
            Self::NormalizedString => Xs::NormalizedString,
            Self::Token => Xs::Token,
            Self::Language => Xs::Language,
            Self::NMTOKEN => Xs::NMTOKEN,
            Self::Name => Xs::Name,
            Self::NCName => Xs::NCName,
            Self::ID => Xs::ID,
            Self::IDREF => Xs::IDREF,
            Self::ENTITY => Xs::ENTITY,
            Self::AnyURI => Xs::AnyURI,
        }
    }
}

/// The types of binary supported as atomic values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryType {
    /// xs:base64Binary
    Base64,
    /// xs:hexBinary
    Hex,
}

impl BinaryType {
    pub(crate) fn schema_type(&self) -> Xs {
        match self {
            Self::Base64 => Xs::Base64Binary,
            Self::Hex => Xs::HexBinary,
        }
    }
}
