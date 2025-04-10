use rust_decimal::Decimal;
use strum_macros::{EnumDiscriminants, EnumString, VariantNames};

pub use xee_name::Name;
use xee_xpath_ast::ast as xpath_ast;

pub trait SelectOrSequenceConstructor {
    fn select(&self) -> Option<&Expression>;
    fn sequence_constructor(&self) -> &SequenceConstructor;
}

// TODO: standard attribute support such as expand-text during the parse, this
// should be respected and parse into the right thing, so the AST does not need
// to retain knowledge of expand-text

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl From<&xot::Span> for Span {
    fn from(span: &xot::Span) -> Self {
        Self {
            start: span.start,
            end: span.end,
        }
    }
}

pub type EqName = xpath_ast::Name;
pub type QName = String;
pub type NcName = String;
pub type SequenceType = xpath_ast::SequenceType;
pub type ItemType = xpath_ast::ItemType;
pub type Token = String;
pub type Uri = String;
pub type Language = String;
pub type Prefix = String;
pub type NmToken = String;
pub type Id = String;
pub type PcData = String;

// type Expression = xpath_ast::XPath;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Expression {
    pub xpath: xpath_ast::XPath,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Pattern {
    pub pattern: xee_xpath_ast::Pattern<xpath_ast::ExprS>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ValueTemplate<V>
where
    V: Clone + PartialEq + Eq,
{
    pub template: Vec<ValueTemplateItem>,

    // TODO: not sure this type information is useful
    pub phantom: std::marker::PhantomData<V>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ValueTemplateItem {
    String { text: String, span: Span },
    Curly { c: char },
    Value { xpath: xpath_ast::XPath, span: Span },
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Standard {
    pub default_collation: Option<Vec<Uri>>,
    pub default_mode: Option<DefaultMode>,
    pub default_validation: Option<DefaultValidation>,
    pub exclude_result_prefixes: Option<ExcludeResultPrefixes>,
    pub expand_text: Option<bool>,
    pub extension_element_prefixes: Option<Vec<Prefix>>,
    pub use_when: Option<Expression>,
    pub version: Option<Decimal>,
    pub xpath_default_namespace: Option<Uri>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StaticStandard {
    // should default-collation be part of this too?
    pub xpath_default_namespace: Option<Uri>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ExcludeResultPrefixes {
    All,
    Prefixes(Vec<ExcludeResultPrefix>),
}

impl ExcludeResultPrefixes {
    // TODO: This combine isn't good enough; it should take existing prefixes
    // into account, which we do have on context
    pub(crate) fn combine(&self, other: ExcludeResultPrefixes) -> Self {
        match (self, other) {
            (ExcludeResultPrefixes::All, _) => ExcludeResultPrefixes::All,
            (_, ExcludeResultPrefixes::All) => ExcludeResultPrefixes::All,
            (
                ExcludeResultPrefixes::Prefixes(prefixes),
                ExcludeResultPrefixes::Prefixes(other_prefixes),
            ) => {
                let mut prefixes = prefixes.clone();
                prefixes.extend(other_prefixes);
                ExcludeResultPrefixes::Prefixes(prefixes)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ExcludeResultPrefix {
    Prefix(Prefix),
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Visibility {
    Public,
    Private,
    Final,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum VisibilityWithAbstract {
    Public,
    Private,
    Final,
    Abstract,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum VisibilityWithHidden {
    Public,
    Private,
    Final,
    Abstract,
    Hidden,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Validation {
    Strict,
    Lax,
    Preserve,
    Strip,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Component {
    Template,
    Function,
    AttributeSet,
    Variable,
    Mode,
    Star,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Use {
    Required,
    Optional,
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Order {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CaseOrder {
    UpperFirst,
    LowerFirst,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DataType {
    Text,
    Number,
    EQName(EqName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Accept {
    pub component: Component,
    pub names: Vec<Token>,
    pub visibility: VisibilityWithHidden,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Accumulator {
    pub name: EqName,
    pub initial_value: Expression,
    pub as_: Option<SequenceType>,
    pub streamable: bool,

    pub rules: Vec<AccumulatorRule>,

    pub span: Span,
}

impl From<Accumulator> for Declaration {
    fn from(i: Accumulator) -> Self {
        Declaration::Accumulator(Box::new(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AccumulatorRule {
    pub match_: Pattern,
    pub phase: Option<AccumulatorPhase>,
    pub select: Option<Expression>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AccumulatorPhase {
    Start,
    End,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AnalyzeString {
    pub select: Expression,
    pub regex: ValueTemplate<String>,
    pub flags: Option<ValueTemplate<String>>,

    pub matching_substring: Option<MatchingSubstring>,
    pub non_matching_substring: Option<NonMatchingSubstring>,
    pub fallbacks: Vec<Fallback>,

    pub span: Span,
}

impl From<AnalyzeString> for SequenceConstructorItem {
    fn from(i: AnalyzeString) -> Self {
        SequenceConstructorInstruction::AnalyzeString(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ApplyImports {
    pub with_params: Vec<WithParam>,

    pub span: Span,
}

impl From<ApplyImports> for SequenceConstructorItem {
    fn from(i: ApplyImports) -> Self {
        SequenceConstructorInstruction::ApplyImports(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ApplyTemplates {
    pub select: Expression,
    pub mode: ApplyTemplatesModeValue,

    pub content: Vec<ApplyTemplatesContent>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ApplyTemplatesModeValue {
    EqName(EqName),
    Unnamed,
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[allow(clippy::large_enum_variant)]
pub enum ApplyTemplatesContent {
    Sort(Sort),
    WithParam(WithParam),
}

impl From<ApplyTemplates> for SequenceConstructorItem {
    fn from(i: ApplyTemplates) -> Self {
        SequenceConstructorInstruction::ApplyTemplates(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Assert {
    pub test: Expression,
    pub select: Option<Expression>,
    pub error_code: Option<ValueTemplate<EqName>>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Assert> for SequenceConstructorItem {
    fn from(i: Assert) -> Self {
        SequenceConstructorInstruction::Assert(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Attribute {
    pub name: ValueTemplate<QName>,
    pub namespace: Option<ValueTemplate<Uri>>,
    pub select: Option<Expression>,
    pub separator: Option<ValueTemplate<String>>,
    pub type_: Option<EqName>,
    pub validation: Option<Validation>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Attribute> for SequenceConstructorItem {
    fn from(i: Attribute) -> Self {
        SequenceConstructorInstruction::Attribute(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for Attribute {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct AttributeSet {
    pub name: EqName,
    pub use_attribute_sets: Option<Vec<EqName>>,
    pub visibility: Option<VisibilityWithAbstract>,
    pub streamable: bool,

    pub attributes: Vec<Attribute>,

    pub span: Span,
}

impl From<AttributeSet> for OverrideContent {
    fn from(i: AttributeSet) -> Self {
        OverrideContent::AttributeSet(Box::new(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Break {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Break> for SequenceConstructorItem {
    fn from(i: Break) -> Self {
        SequenceConstructorInstruction::Break(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CallTemplate {
    pub name: EqName,

    pub with_params: Vec<WithParam>,

    pub span: Span,
}

impl From<CallTemplate> for SequenceConstructorItem {
    fn from(i: CallTemplate) -> Self {
        SequenceConstructorInstruction::CallTemplate(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Catch {
    pub errors: Option<Vec<Token>>,
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CharacterMap {
    pub name: EqName,
    pub use_character_maps: Option<Vec<EqName>>,

    pub output_characters: Vec<OutputCharacter>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Choose {
    pub when: Vec<When>,
    pub otherwise: Option<Otherwise>,

    pub span: Span,
}

impl From<Choose> for SequenceConstructorItem {
    fn from(i: Choose) -> Self {
        SequenceConstructorInstruction::Choose(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Comment {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Comment> for SequenceConstructorItem {
    fn from(i: Comment) -> Self {
        SequenceConstructorInstruction::Comment(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for Comment {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ContextItem {
    pub as_: Option<ItemType>,
    pub use_: Option<Use>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Copy {
    pub select: Option<Expression>,
    pub copy_namespaces: bool,
    pub inherit_namespaces: bool,
    pub use_attribute_sets: Option<Vec<EqName>>,
    pub type_: Option<EqName>,
    pub validation: Validation,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Copy> for SequenceConstructorItem {
    fn from(i: Copy) -> Self {
        SequenceConstructorInstruction::Copy(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CopyOf {
    pub select: Expression,
    pub copy_accumulators: bool,
    pub copy_namespaces: bool,
    pub type_: Option<EqName>,
    pub validation: Option<Validation>,

    pub span: Span,
}

impl From<CopyOf> for SequenceConstructorItem {
    fn from(i: CopyOf) -> Self {
        SequenceConstructorInstruction::CopyOf(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DecimalFormat {
    pub name: Option<EqName>,
    pub decimal_separator: Option<char>,
    pub grouping_separator: Option<char>,
    pub infinity: Option<String>,
    pub minus_sign: Option<char>,
    pub exponent_separator: Option<char>,
    pub nan: Option<String>,
    pub percent: Option<char>,
    pub per_mille: Option<char>,
    pub zero_digit: Option<char>,
    pub digit: Option<char>,
    pub pattern_separator: Option<char>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Document {
    pub validation: Option<Validation>,
    pub type_: Option<EqName>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Document> for SequenceConstructorItem {
    fn from(i: Document) -> Self {
        SequenceConstructorInstruction::Document(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Element {
    pub name: ValueTemplate<QName>,
    pub namespace: Option<ValueTemplate<Uri>>,
    pub inherit_namespaces: bool,
    pub use_attribute_sets: Option<Vec<EqName>>,
    pub type_: Option<EqName>,
    pub validation: Option<Validation>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Element> for SequenceConstructorItem {
    fn from(i: Element) -> Self {
        SequenceConstructorInstruction::Element(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Evaluate {
    pub xpath: Expression,
    pub as_: Option<SequenceType>,
    pub base_uri: Option<ValueTemplate<Uri>>,
    pub with_params: Option<Expression>,
    pub context_item: Option<Expression>,
    pub namespace_context: Option<Expression>,
    pub schema_aware: Option<ValueTemplate<bool>>,

    pub content: Vec<EvaluateContent>,

    pub span: Span,
}

impl From<Evaluate> for SequenceConstructorItem {
    fn from(i: Evaluate) -> Self {
        SequenceConstructorInstruction::Evaluate(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum EvaluateContent {
    WithParam(WithParam),
    Fallback(Fallback),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Expose {
    pub component: Component,
    pub names: Vec<Token>,
    pub visibility: VisibilityWithAbstract,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Fallback {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Fallback> for SequenceConstructorItem {
    fn from(i: Fallback) -> Self {
        SequenceConstructorInstruction::Fallback(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForEach {
    pub select: Expression,

    pub sort: Vec<Sort>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<ForEach> for SequenceConstructorItem {
    fn from(i: ForEach) -> Self {
        SequenceConstructorInstruction::ForEach(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ForEachGroup {
    pub select: Expression,
    pub group_by: Option<Expression>,
    pub group_adjacent: Option<Expression>,
    pub group_starting_with: Option<Pattern>,
    pub group_ending_with: Option<Pattern>,
    pub composite: bool,
    pub collation: Option<ValueTemplate<Uri>>,

    pub sort: Vec<Sort>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<ForEachGroup> for SequenceConstructorItem {
    fn from(i: ForEachGroup) -> Self {
        SequenceConstructorInstruction::ForEachGroup(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Fork {
    pub fallbacks: Vec<Fallback>,
    pub content: ForkContent,

    pub span: Span,
}

impl From<Fork> for SequenceConstructorItem {
    fn from(i: Fork) -> Self {
        SequenceConstructorInstruction::Fork(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ForkContent {
    SequenceFallbacks(Vec<(Sequence, Vec<Fallback>)>),
    ForEachGroup((ForEachGroup, Vec<Fallback>)),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Function {
    pub name: EqName,
    pub as_: Option<SequenceType>,
    pub visibility: Option<VisibilityWithAbstract>,
    pub streamability: Option<Streamability>,
    pub override_extension_function: bool,
    pub override_: bool,
    pub new_each_time: Option<NewEachTime>,
    pub cache: bool,

    pub params: Vec<Param>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Function> for OverrideContent {
    fn from(i: Function) -> Self {
        OverrideContent::Function(Box::new(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Streamability {
    Unclassified,
    Absorbing,
    Inspection,
    Filter,
    ShallowDescent,
    DeepDescent,
    Ascent,
    EqName(EqName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NewEachTime {
    Yes,
    No,
    Maybe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GlobalContextItem {
    pub as_: Option<ItemType>,
    pub use_: Option<Use>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct If {
    pub test: Expression,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<If> for SequenceConstructorItem {
    fn from(i: If) -> Self {
        SequenceConstructorInstruction::If(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Import {
    pub href: Uri,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ImportSchema {
    pub namespace: Option<Uri>,
    pub schema_location: Option<Uri>,

    pub span: Span,

    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Include {
    pub href: Uri,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Iterate {
    pub select: Expression,

    pub span: Span,

    pub params: Vec<Param>,
    pub on_completion: Option<OnCompletion>,
    pub sequence_constructor: SequenceConstructor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Key {
    pub name: EqName,
    pub match_: Pattern,
    pub use_: Option<Expression>,
    pub composite: bool,
    pub collation: Option<Uri>,

    pub span: Span,

    pub sequence_constructor: SequenceConstructor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Map {
    pub span: Span,

    pub sequence_constructor: SequenceConstructor,
}

impl From<Map> for SequenceConstructorItem {
    fn from(i: Map) -> Self {
        SequenceConstructorInstruction::Map(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MapEntry {
    pub key: Expression,
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<MapEntry> for SequenceConstructorItem {
    fn from(i: MapEntry) -> Self {
        SequenceConstructorInstruction::MapEntry(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MatchingSubstring {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Merge {
    pub span: Span,

    pub merge_sources: Vec<MergeSource>,
    pub merge_action: MergeAction,
    pub fallbacks: Vec<Fallback>,
}

impl From<Merge> for SequenceConstructorItem {
    fn from(i: Merge) -> Self {
        SequenceConstructorInstruction::Merge(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MergeAction {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MergeKey {
    pub select: Option<Expression>,
    pub lang: Option<ValueTemplate<Language>>,
    pub order: Option<ValueTemplate<Order>>,
    pub collation: Option<ValueTemplate<Uri>>,
    pub case_order: Option<ValueTemplate<CaseOrder>>,
    pub data_type: Option<ValueTemplate<DataType>>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MergeSource {
    pub name: Option<NcName>,
    pub for_each_item: Option<Expression>,
    pub for_each_source: Option<Expression>,
    pub select: Expression,
    pub streamable: bool,
    pub use_accumulators: Option<Vec<Token>>,
    pub sort_before_merge: bool,
    pub validation: Option<Validation>,
    pub type_: Option<EqName>,

    pub span: Span,

    pub merge_keys: Vec<MergeKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Message {
    pub select: Option<Expression>,
    pub terminate: Option<ValueTemplate<bool>>,
    pub error_code: Option<ValueTemplate<EqName>>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Message> for SequenceConstructorItem {
    fn from(i: Message) -> Self {
        SequenceConstructorInstruction::Message(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Mode {
    pub name: Option<EqName>,
    pub streamable: bool,
    pub use_accumulators: Option<Vec<Token>>,
    pub on_no_match: Option<OnNoMatch>,
    pub on_multiple_match: Option<OnMultipleMatch>,
    pub warning_on_no_match: bool,
    pub warning_on_multiple_match: bool,
    pub typed: Option<Typed>,
    pub visibility: Option<Visibility>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum OnNoMatch {
    DeepCopy,
    ShallowCopy,
    DeepSkip,
    ShallowSkip,
    TextOnlyCopy,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum OnMultipleMatch {
    UseLast,
    Fail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Typed {
    Boolean,
    Strict,
    Lax,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Namespace {
    pub name: ValueTemplate<NcName>,
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Namespace> for SequenceConstructorItem {
    fn from(i: Namespace) -> Self {
        SequenceConstructorInstruction::Namespace(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for Namespace {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NamespaceAlias {
    pub stylesheet_prefix: PrefixOrDefault,
    pub result_prefix: PrefixOrDefault,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PrefixOrDefault {
    Prefix(Prefix),
    Default,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NextIteration {
    pub span: Span,

    pub with_params: Vec<WithParam>,
}

impl From<NextIteration> for SequenceConstructorItem {
    fn from(i: NextIteration) -> Self {
        SequenceConstructorInstruction::NextIteration(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NextMatch {
    pub content: Vec<NextMatchContent>,

    pub span: Span,
}

impl From<NextMatch> for SequenceConstructorItem {
    fn from(i: NextMatch) -> Self {
        SequenceConstructorInstruction::NextMatch(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NextMatchContent {
    WithParam(WithParam),
    Fallback(Fallback),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct NonMatchingSubstring {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Number {
    pub value: Option<Expression>,
    pub select: Option<Expression>,
    pub level: Option<NumberLevel>,
    pub count: Option<Pattern>,
    pub from: Option<Pattern>,
    pub format: Option<ValueTemplate<String>>,
    pub lang: Option<ValueTemplate<Language>>,
    pub letter_value: Option<ValueTemplate<LetterValue>>,
    pub ordinal: Option<ValueTemplate<String>>,
    pub start_at: Option<ValueTemplate<String>>,
    pub grouping_separator: Option<ValueTemplate<char>>,
    pub grouping_size: Option<ValueTemplate<usize>>,

    pub span: Span,
}

impl From<Number> for SequenceConstructorItem {
    fn from(i: Number) -> Self {
        SequenceConstructorInstruction::Number(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NumberLevel {
    Single,
    Multiple,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum LetterValue {
    Alphabetic,
    Traditional,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OnCompletion {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OnEmpty {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<OnEmpty> for SequenceConstructorItem {
    fn from(i: OnEmpty) -> Self {
        SequenceConstructorInstruction::OnEmpty(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OnNonEmpty {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<OnNonEmpty> for SequenceConstructorItem {
    fn from(i: OnNonEmpty) -> Self {
        SequenceConstructorInstruction::OnNonEmpty(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Otherwise {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Output {
    pub name: Option<EqName>,
    pub method: Option<OutputMethod>,
    pub allow_duplicate_names: bool,
    pub build_tree: bool,
    pub byte_order_mark: bool,
    pub cdata_section_elements: Option<Vec<EqName>>,
    pub doctype_public: Option<String>,
    pub doctype_system: Option<String>,
    pub encoding: Option<String>,
    pub escape_uri_attributes: bool,
    pub html_version: Option<Decimal>,
    pub include_content_type: bool,
    pub indent: bool,
    pub item_separator: Option<String>,
    pub json_node_output_method: Option<JsonNodeOutputMethod>,
    pub media_type: Option<String>,
    pub normalization_form: Option<NormalizationForm>,
    pub omit_xml_declaration: bool,
    pub parameter_document: Option<Uri>,
    pub standalone: Option<Standalone>,
    pub suppress_indentation: Option<Vec<EqName>>,
    pub undeclare_prefixes: bool,
    pub use_character_maps: Option<Vec<EqName>>,
    pub version: Option<NmToken>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum OutputMethod {
    Xml,
    Html,
    Xhtml,
    Text,
    Json,
    Adaptive,
    EqName(EqName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum JsonNodeOutputMethod {
    Xml,
    Html,
    Xhtml,
    Text,
    EqName(EqName),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum NormalizationForm {
    Nfc,
    Nfd,
    Nfkc,
    Nfkd,
    FullyNormalized,
    None,
    NmToken(NmToken),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Standalone {
    Bool(bool),
    Omit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OutputCharacter {
    pub character: char,
    pub string: String,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Override {
    pub content: Vec<OverrideContent>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumDiscriminants)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[strum_discriminants(derive(EnumString, VariantNames))]
#[strum_discriminants(strum(serialize_all = "kebab-case"))]
#[strum_discriminants(name(OverrideContentName))]
pub enum OverrideContent {
    Template(Box<Template>),
    Function(Box<Function>),
    Variable(Box<Variable>),
    Param(Box<Param>),
    AttributeSet(Box<AttributeSet>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Package {
    pub id: Option<Id>,
    pub name: Option<Uri>,
    pub package_version: Option<String>,
    pub version: Decimal,
    pub input_type_annotations: Option<InputTypeAnnotations>,
    pub declared_modes: Option<bool>,
    pub default_mode: Option<DefaultMode>,
    pub default_validation: Option<DefaultValidation>,
    pub default_collation: Option<Vec<Uri>>,
    pub extension_element_prefixes: Option<Vec<Prefix>>,
    pub exclude_result_prefixes: Option<Vec<Prefix>>,
    pub expand_text: Option<bool>,
    pub use_when: Option<Expression>,
    pub xpath_default_namespace: Option<Uri>,

    pub content: Vec<PackageContent>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum InputTypeAnnotations {
    Preserve,
    Strip,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DefaultMode {
    EqName(EqName),
    Unnamed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DefaultValidation {
    Preserve,
    Strip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum PackageContent {
    Expose(Expose),
    Declarations(Declarations),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Param {
    pub name: EqName,
    pub select: Option<Expression>,
    pub as_: Option<SequenceType>,
    pub required: bool,
    pub tunnel: bool,
    pub static_: bool,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Param> for OverrideContent {
    fn from(i: Param) -> Self {
        OverrideContent::Param(Box::new(i))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PerformSort {
    pub select: Option<Expression>,

    pub sorts: Vec<Sort>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PreserveSpace {
    pub elements: Vec<Token>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ProcessingInstruction {
    pub name: ValueTemplate<NcName>,
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<ProcessingInstruction> for SequenceConstructorItem {
    fn from(i: ProcessingInstruction) -> Self {
        SequenceConstructorInstruction::ProcessingInstruction(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for ProcessingInstruction {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ResultDocument {
    pub format: Option<ValueTemplate<EqName>>,
    pub href: Option<ValueTemplate<Uri>>,
    pub validation: Option<Validation>,
    pub type_: EqName,
    pub method: Option<ValueTemplate<OutputMethod>>,
    pub allow_duplicate_names: Option<ValueTemplate<bool>>,
    pub build_tree: Option<ValueTemplate<bool>>,
    pub bye_order_mark: Option<ValueTemplate<bool>>,
    pub cdata_section_elements: Option<ValueTemplate<Vec<EqName>>>,
    pub doctype_public: Option<ValueTemplate<String>>,
    pub doctype_system: Option<ValueTemplate<String>>,
    pub encoding: Option<ValueTemplate<String>>,
    pub escape_uri_attributes: Option<ValueTemplate<bool>>,
    pub html_version: Option<ValueTemplate<Decimal>>,
    pub include_content_type: Option<ValueTemplate<bool>>,
    pub indent: Option<ValueTemplate<bool>>,
    pub item_separator: Option<ValueTemplate<String>>,
    pub json_node_output_method: Option<ValueTemplate<JsonNodeOutputMethod>>,
    pub media_type: Option<ValueTemplate<String>>,
    pub normalization_form: Option<ValueTemplate<NormalizationForm>>,
    pub omit_xml_declaration: Option<ValueTemplate<bool>>,
    pub parameter_document: Option<ValueTemplate<Uri>>,
    pub standalone: Option<ValueTemplate<Standalone>>,
    pub suppress_indentation: Option<ValueTemplate<Vec<EqName>>>,
    pub undeclare_prefixes: Option<ValueTemplate<bool>>,
    pub use_character_maps: Option<Vec<EqName>>,
    pub version: Option<ValueTemplate<NmToken>>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sequence {
    pub select: Option<Expression>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Sequence> for SequenceConstructorItem {
    fn from(i: Sequence) -> Self {
        SequenceConstructorInstruction::Sequence(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for Sequence {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sort {
    pub select: Option<Expression>,
    pub lang: Option<ValueTemplate<Language>>,
    pub order: Option<ValueTemplate<Order>>,
    pub collation: Option<ValueTemplate<Uri>>,
    pub stable: Option<ValueTemplate<bool>>,
    pub case_order: Option<ValueTemplate<CaseOrder>>,
    pub data_type: Option<ValueTemplate<DataType>>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceDocument {
    pub href: ValueTemplate<Uri>,
    pub streamable: bool,
    pub use_accumulators: Option<Vec<Token>>,
    pub validation: Option<Validation>,
    pub type_: Option<EqName>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<SourceDocument> for SequenceConstructorItem {
    fn from(i: SourceDocument) -> Self {
        SequenceConstructorInstruction::SourceDocument(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct StripSpace {
    pub elements: Vec<Token>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Template {
    pub match_: Option<Pattern>,
    pub name: Option<EqName>,
    pub priority: Option<Decimal>,
    pub mode: Vec<ModeValue>,
    pub as_: Option<SequenceType>,
    pub visibility: Option<VisibilityWithAbstract>,

    pub context_item: Option<ContextItem>,
    pub params: Vec<Param>,
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum ModeValue {
    EqName(EqName),
    Unnamed,
    All,
}

impl From<Template> for OverrideContent {
    fn from(t: Template) -> Self {
        OverrideContent::Template(Box::new(t))
    }
}

impl From<Template> for Declaration {
    fn from(t: Template) -> Self {
        Declaration::Template(Box::new(t))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Text {
    // DEPRECATED
    pub disable_output_escaping: bool,

    pub content: ValueTemplate<String>,

    pub span: Span,
}

impl From<Text> for SequenceConstructorItem {
    fn from(i: Text) -> Self {
        SequenceConstructorInstruction::Text(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Transform {
    pub id: Option<Id>,
    pub input_type_annotations: Option<InputTypeAnnotations>,
    pub extension_element_prefixes: Option<Vec<Prefix>>,

    // even though the spec declares more attributes for this,
    // they're all standard attributes
    pub declarations: Declarations,

    pub span: Span,
}

// Stylesheet is an alias for Transform

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Try {
    pub select: Option<Expression>,
    pub rollback_output: Option<bool>,

    pub sequence_constructor: SequenceConstructor,
    // TODO: at least one catch needs to be there, so could fold it into
    // the catches block
    pub catch: Catch,
    pub catches: Vec<TryCatchOrFallback>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TryCatchOrFallback {
    Catch(Catch),
    Fallback(Fallback),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct UsePackage {
    pub name: Uri,
    pub package_version: Option<String>,

    pub content: Vec<UsePackageContent>,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum UsePackageContent {
    Accept(Accept),
    Override(Override),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ValueOf {
    pub select: Option<Expression>,
    pub separator: Option<ValueTemplate<String>>,
    // DEPRECATED
    pub disable_output_escaping: bool,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<ValueOf> for SequenceConstructorItem {
    fn from(i: ValueOf) -> Self {
        SequenceConstructorInstruction::ValueOf(Box::new(i)).into()
    }
}

impl SelectOrSequenceConstructor for ValueOf {
    fn select(&self) -> Option<&Expression> {
        self.select.as_ref()
    }

    fn sequence_constructor(&self) -> &SequenceConstructor {
        &self.sequence_constructor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Variable {
    pub name: EqName,
    pub select: Option<Expression>,
    pub as_: Option<SequenceType>,
    pub static_: bool,
    pub visibility: Option<VisibilityWithAbstract>,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<Variable> for SequenceConstructorItem {
    fn from(v: Variable) -> Self {
        SequenceConstructorInstruction::Variable(Box::new(v)).into()
    }
}

impl From<Variable> for OverrideContent {
    fn from(v: Variable) -> Self {
        OverrideContent::Variable(Box::new(v))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct When {
    pub test: Expression,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WherePopulated {
    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

impl From<WherePopulated> for SequenceConstructorItem {
    fn from(i: WherePopulated) -> Self {
        SequenceConstructorInstruction::WherePopulated(Box::new(i)).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WithParam {
    pub name: EqName,
    pub select: Option<Expression>,
    pub as_: Option<SequenceType>,
    pub tunnel: bool,

    pub sequence_constructor: SequenceConstructor,

    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SequenceConstructorItem {
    Content(Content),
    Instruction(SequenceConstructorInstruction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Content {
    Element(Box<ElementNode>),
    Text(String),
    Value(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq, Eq, EnumDiscriminants)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[strum_discriminants(derive(EnumString, VariantNames))]
#[strum_discriminants(strum(serialize_all = "kebab-case"))]
#[strum_discriminants(name(SequenceConstructorName))]
pub enum SequenceConstructorInstruction {
    AnalyzeString(Box<AnalyzeString>),
    ApplyImports(Box<ApplyImports>),
    ApplyTemplates(Box<ApplyTemplates>),
    Assert(Box<Assert>),
    Attribute(Box<Attribute>),
    Break(Box<Break>),
    CallTemplate(Box<CallTemplate>),
    Choose(Box<Choose>),
    Comment(Box<Comment>),
    Copy(Box<Copy>),
    CopyOf(Box<CopyOf>),
    Document(Box<Document>),
    Element(Box<Element>),
    Evaluate(Box<Evaluate>),
    Fallback(Box<Fallback>),
    ForEach(Box<ForEach>),
    ForEachGroup(Box<ForEachGroup>),
    Fork(Box<Fork>),
    If(Box<If>),
    Iterate(Box<Iterate>),
    Map(Box<Map>),
    MapEntry(Box<MapEntry>),
    Merge(Box<Merge>),
    Message(Box<Message>),
    Namespace(Box<Namespace>),
    NextIteration(Box<NextIteration>),
    NextMatch(Box<NextMatch>),
    Number(Box<Number>),
    OnEmpty(Box<OnEmpty>),
    OnNonEmpty(Box<OnNonEmpty>),
    PerformSort(Box<PerformSort>),
    ProcessingInstruction(Box<ProcessingInstruction>),
    ResultDocument(Box<ResultDocument>),
    Sequence(Box<Sequence>),
    SourceDocument(Box<SourceDocument>),
    Text(Box<Text>),
    Try(Box<Try>),
    ValueOf(Box<ValueOf>),
    Variable(Box<Variable>),
    WherePopulated(Box<WherePopulated>),
}

impl From<SequenceConstructorInstruction> for SequenceConstructorItem {
    fn from(i: SequenceConstructorInstruction) -> Self {
        SequenceConstructorItem::Instruction(i)
    }
}

pub type SequenceConstructor = Vec<SequenceConstructorItem>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ElementNode {
    pub name: Name,
    pub attributes: Vec<(Name, ValueTemplate<String>)>,
    pub sequence_constructor: SequenceConstructor,
    pub span: Span,
}

impl From<ElementNode> for SequenceConstructorItem {
    fn from(e: ElementNode) -> Self {
        SequenceConstructorItem::Content(Content::Element(Box::new(e)))
    }
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// #[cfg_attr(feature = "serde", derive(serde::Serialize))]
// pub struct Name {
//     pub namespace: String,
//     pub local: String,
// }

#[derive(Debug, Clone, PartialEq, Eq, EnumDiscriminants)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[strum_discriminants(derive(EnumString, VariantNames))]
#[strum_discriminants(strum(serialize_all = "kebab-case"))]
#[strum_discriminants(name(DeclarationName))]
pub enum Declaration {
    Accumulator(Box<Accumulator>),
    CharacterMap(Box<CharacterMap>),
    DecimalFormat(Box<DecimalFormat>),
    Function(Box<Function>),
    GlobalContextItem(Box<GlobalContextItem>),
    Import(Box<Import>),
    ImportSchema(Box<ImportSchema>),
    Include(Box<Include>),
    Key(Box<Key>),
    Mode(Box<Mode>),
    NamespaceAlias(Box<NamespaceAlias>),
    Output(Box<Output>),
    Param(Box<Param>),
    PreserveSpace(Box<PreserveSpace>),
    StripSpace(Box<StripSpace>),
    Template(Box<Template>),
    UsePackage(Box<UsePackage>),
    Variable(Box<Variable>),
}

pub type Declarations = Vec<Declaration>;

// xs:schema
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Schema {
    // TODO
}
