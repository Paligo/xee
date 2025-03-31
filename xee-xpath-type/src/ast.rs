use xee_name::Name;
use xee_schema_type::Xs;
use xot::xmlname::NameStrInfo;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SequenceType {
    Empty,
    Item(Item),
}

impl SequenceType {
    pub fn display_representation(&self) -> String {
        match self {
            Self::Empty => "empty-sequence()".to_string(),
            Self::Item(item) => item.display_representation(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Item {
    pub item_type: ItemType,
    pub occurrence: Occurrence,
}

impl Item {
    fn display_representation(&self) -> String {
        let occurrence = match self.occurrence {
            Occurrence::One => "".to_string(),
            Occurrence::Option => "?".to_string(),
            Occurrence::Many => "*".to_string(),
            Occurrence::NonEmpty => "+".to_string(),
        };

        format!("{}{}", self.item_type.display_representation(), occurrence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ItemType {
    Item,
    AtomicOrUnionType(Xs),
    KindTest(KindTest),
    FunctionTest(FunctionTest),
    MapTest(MapTest),
    ArrayTest(ArrayTest),
}

impl ItemType {
    fn display_representation(&self) -> String {
        match self {
            Self::Item => "item()".to_string(),
            Self::AtomicOrUnionType(xs) => format!("xs:{}", xs.local_name()),
            Self::KindTest(kind_test) => kind_test.display_representation(),
            Self::FunctionTest(function_test) => function_test.display_representation(),
            Self::MapTest(map_test) => map_test.display_representation(),
            Self::ArrayTest(array_test) => array_test.display_representation(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Occurrence {
    One,
    Option,
    Many,
    NonEmpty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum KindTest {
    Document(Option<DocumentTest>),
    Element(Option<ElementOrAttributeTest>),
    Attribute(Option<ElementOrAttributeTest>),
    SchemaElement(SchemaElementTest),
    SchemaAttribute(SchemaAttributeTest),
    PI(Option<PITest>),
    Comment,
    Text,
    NamespaceNode,
    Any,
}

impl KindTest {
    fn display_representation(&self) -> String {
        match self {
            Self::Document(document_test) => {
                format!(
                    "document-node({})",
                    document_test
                        .as_ref()
                        .map_or("".to_string(), |dt| dt.display_representation())
                )
            }
            Self::Element(element_test) => {
                format!(
                    "element({})",
                    element_test
                        .as_ref()
                        .map_or("".to_string(), |et| et.display_representation())
                )
            }
            Self::Attribute(attribute_test) => {
                format!(
                    "attribute({})",
                    attribute_test
                        .as_ref()
                        .map_or("".to_string(), |at| at.display_representation())
                )
            }
            Self::SchemaElement(schema_element_test) => {
                format!(
                    "schema-element({})",
                    schema_element_test.display_representation()
                )
            }
            Self::SchemaAttribute(schema_attribute_test) => {
                format!(
                    "schema-attribute({})",
                    schema_attribute_test.display_representation()
                )
            }
            Self::PI(pi_test) => {
                format!(
                    "processing-instruction({})",
                    pi_test
                        .as_ref()
                        .map_or("".to_string(), |pt| pt.display_representation())
                )
            }
            Self::Comment => "comment()".to_string(),
            Self::Text => "text()".to_string(),
            Self::NamespaceNode => "namespace-node()".to_string(),
            Self::Any => "node()".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DocumentTest {
    Element(Option<ElementOrAttributeTest>),
    SchemaElement(SchemaElementTest),
}

impl DocumentTest {
    fn display_representation(&self) -> String {
        match self {
            Self::Element(element_test) => {
                format!(
                    "element({})",
                    element_test
                        .as_ref()
                        .map_or("".to_string(), |et| et.display_representation())
                )
            }
            Self::SchemaElement(schema_element_test) => {
                format!(
                    "schema-element({})",
                    schema_element_test.display_representation()
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ElementOrAttributeTest {
    pub name_or_wildcard: NameOrWildcard,
    pub type_name: Option<TypeName>,
}

impl ElementOrAttributeTest {
    fn display_representation(&self) -> String {
        let name_or_wildcard = match &self.name_or_wildcard {
            NameOrWildcard::Name(name) => name.full_name().to_string(),
            NameOrWildcard::Wildcard => "*".to_string(),
        };

        let type_name = self
            .type_name
            .as_ref()
            .map_or("".to_string(), |tn| tn.display_representation());

        format!("{}{}", name_or_wildcard, type_name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypeName {
    pub name: Xs,
    // only relevant for elements; for attributes it's always true
    pub can_be_nilled: bool,
}

impl TypeName {
    fn display_representation(&self) -> String {
        let nilled = if self.can_be_nilled { "? " } else { "" };
        format!(" as {}{}", self.name.local_name(), nilled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NameOrWildcard {
    Name(Name),
    Wildcard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaElementTest {
    pub name: Name,
}

impl SchemaElementTest {
    fn display_representation(&self) -> String {
        self.name.full_name().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SchemaAttributeTest {
    pub name: Name,
}

impl SchemaAttributeTest {
    fn display_representation(&self) -> String {
        self.name.full_name().to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum FunctionTest {
    AnyFunctionTest,
    TypedFunctionTest(Box<TypedFunctionTest>),
}

impl FunctionTest {
    fn display_representation(&self) -> String {
        match self {
            Self::AnyFunctionTest => "function(*)".to_string(),
            Self::TypedFunctionTest(tft) => tft.display_representation(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypedFunctionTest {
    pub parameter_types: Vec<SequenceType>,
    pub return_type: SequenceType,
}

impl TypedFunctionTest {
    fn display_representation(&self) -> String {
        let parameter_types = self
            .parameter_types
            .iter()
            .map(|pt| pt.display_representation())
            .collect::<Vec<_>>()
            .join(", ");
        let return_type = self.return_type.display_representation();
        format!("function({}) as {}", parameter_types, return_type)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MapTest {
    AnyMapTest,
    TypedMapTest(Box<TypedMapTest>),
}

impl MapTest {
    fn display_representation(&self) -> String {
        match self {
            Self::AnyMapTest => "map(*)".to_string(),
            Self::TypedMapTest(tmt) => tmt.display_representation(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypedMapTest {
    pub key_type: Xs,
    pub value_type: SequenceType,
}

impl TypedMapTest {
    fn display_representation(&self) -> String {
        format!(
            "map(xs:{} as {})",
            self.key_type.local_name(),
            self.value_type.display_representation()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ArrayTest {
    AnyArrayTest,
    TypedArrayTest(Box<TypedArrayTest>),
}

impl ArrayTest {
    fn display_representation(&self) -> String {
        match self {
            Self::AnyArrayTest => "array(*)".to_string(),
            Self::TypedArrayTest(tat) => tat.display_representation(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TypedArrayTest {
    pub item_type: SequenceType,
}

impl TypedArrayTest {
    fn display_representation(&self) -> String {
        format!("array({})", self.item_type.display_representation())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PITest {
    Name(String),
    StringLiteral(String),
}

impl PITest {
    fn display_representation(&self) -> String {
        match self {
            Self::Name(name) => name.to_string(),
            Self::StringLiteral(string_literal) => {
                format!(r#""{}""#, string_literal)
            }
        }
    }
}
