// an Intermediate Representation in ANF - administrative normal form
// XXX is this really ANF? Maybe it is, though it doesn't support recursion
// (without function arguments), as XPath doesn't.

use ahash::{HashMap, HashMapExt};
use ibig::IBig;
use ordered_float::OrderedFloat;
use rust_decimal::Decimal;

pub use xee_interpreter::function::Name;
use xee_interpreter::function::{CastType, Signature, StaticFunctionId};
use xee_interpreter::xml;
use xee_schema_type::Xs;
pub use xee_xpath_ast::ast::{BinaryOperator, SequenceType, UnaryOperator};
use xee_xpath_ast::span::Spanned;
use xee_xpath_ast::Pattern;
use xot::xmlname;

pub type AtomS = Spanned<Atom>;
pub type ExprS = Spanned<Expr>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    Atom(AtomS),
    Let(Let),
    If(If),
    Binary(Binary),
    Unary(Unary),
    FunctionDefinition(FunctionDefinition),
    FunctionCall(FunctionCall),
    Lookup(Lookup),
    WildcardLookup(WildcardLookup),
    Step(Step),
    Deduplicate(Box<ExprS>),
    Map(Map),
    Filter(Filter),
    Iterate(Iterate),
    IterateBreak(IterateBreak),
    IterateLetNext(IterateLetNext),
    PatternPredicate(PatternPredicate),
    Quantified(Quantified),
    Cast(Cast),
    Castable(Castable),
    InstanceOf(InstanceOf),
    Treat(Treat),
    MapConstructor(MapConstructor),
    ArrayConstructor(ArrayConstructor),
    XmlName(XmlName),
    XmlDocument(XmlRoot),
    XmlElement(XmlElement),
    XmlAttribute(XmlAttribute),
    XmlNamespace(XmlNamespace),
    XmlText(XmlText),
    XmlComment(XmlComment),
    XmlProcessingInstruction(XmlProcessingInstruction),
    XmlAppend(XmlAppend),
    ApplyTemplates(ApplyTemplates),
    CopyShallow(CopyShallow),
    CopyDeep(CopyDeep),
}

// not to be confused with an XPath atom; this is a variable or a constant
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Atom {
    Const(Const),
    Variable(Name),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Const {
    Integer(IBig),
    String(String),
    Double(OrderedFloat<f64>),
    Decimal(Decimal),
    StaticFunctionReference(StaticFunctionId, Option<ContextNames>),
    // XXX replace this with a sequence constant? useful once we have constant folding
    EmptySequence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextNames {
    pub item: Name,
    pub position: Name,
    pub last: Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Let {
    pub name: Name,
    pub var_expr: Box<ExprS>,
    pub return_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct If {
    pub condition: AtomS,
    pub then: Box<ExprS>,
    pub else_: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    pub left: AtomS,
    pub op: BinaryOperator,
    pub right: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unary {
    pub op: UnaryOperator,
    pub atom: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionDefinition {
    pub params: Vec<Param>,
    pub return_type: Option<SequenceType>,
    pub body: Box<ExprS>,
}

impl FunctionDefinition {
    pub fn signature(&self) -> Signature {
        Signature::new(
            self.params
                .iter()
                .map(|param| param.type_.clone())
                .collect(),
            self.return_type.clone(),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: Name,
    pub type_: Option<SequenceType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionCall {
    pub atom: AtomS,
    pub args: Vec<AtomS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lookup {
    pub atom: AtomS,
    pub arg_atom: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WildcardLookup {
    pub atom: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub step: xml::Step,
    pub context: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Map {
    pub context_names: ContextNames,
    pub var_atom: AtomS,
    pub return_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filter {
    pub context_names: ContextNames,
    pub var_atom: AtomS,
    pub return_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterateParam {
    pub name: Name,
    pub value: Box<ExprS>,
    pub type_: Option<SequenceType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Iterate {
    pub context_names: ContextNames,
    pub loop_name: Name,
    pub var_atom: AtomS,
    pub params: Vec<IterateParam>,
    pub expr: Box<ExprS>,
    pub on_complete: Option<Box<ExprS>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterateBreak {
    pub loop_name: Name,
    pub return_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IterateLetNext {
    pub params: Vec<IterateParam>,
    pub return_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternPredicate {
    pub context_names: ContextNames,
    pub var_atom: AtomS,
    pub expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quantified {
    pub quantifier: Quantifier,
    pub context_names: ContextNames,
    pub var_atom: AtomS,
    pub satisifies_expr: Box<ExprS>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Quantifier {
    Some,
    Every,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cast {
    pub atom: AtomS,
    pub xs: Xs,
    pub empty_sequence_allowed: bool,
}

impl Cast {
    pub fn cast_type(&self) -> CastType {
        CastType {
            xs: self.xs,
            empty_sequence_allowed: self.empty_sequence_allowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Castable {
    pub atom: AtomS,
    pub xs: Xs,
    pub empty_sequence_allowed: bool,
}

impl Castable {
    pub fn cast_type(&self) -> CastType {
        CastType {
            xs: self.xs,
            empty_sequence_allowed: self.empty_sequence_allowed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceOf {
    pub atom: AtomS,
    pub sequence_type: SequenceType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Treat {
    pub atom: AtomS,
    pub sequence_type: SequenceType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapConstructor {
    pub members: Vec<(AtomS, AtomS)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayConstructor {
    Square(Vec<AtomS>),
    Curly(AtomS),
}

// These are extensions to the IR that are only used by XSLT

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlName {
    pub local_name: AtomS,
    pub namespace: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlRoot {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlElement {
    pub name: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAttribute {
    pub name: AtomS,
    pub value: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlNamespace {
    pub prefix: AtomS,
    pub namespace: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlText {
    pub value: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlComment {
    pub value: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlProcessingInstruction {
    pub target: AtomS,
    pub content: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlAppend {
    pub parent: AtomS,
    pub child: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyTemplates {
    pub mode: ApplyTemplatesModeValue,
    pub select: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApplyTemplatesModeValue {
    Named(xmlname::OwnedName),
    Unnamed,
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyShallow {
    pub select: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyDeep {
    pub select: AtomS,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub modes: Vec<ModeValue>,
    pub priority: Decimal,
    pub pattern: Pattern<FunctionDefinition>,
    pub function_definition: FunctionDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ModeValue {
    Named(xmlname::OwnedName),
    Unnamed,
    All,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mode {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declarations {
    pub rules: Vec<Rule>,
    pub modes: HashMap<Option<xmlname::OwnedName>, Mode>,
    pub functions: Vec<FunctionBinding>,
    pub main: FunctionDefinition,
}

impl Declarations {
    pub fn new(main: FunctionDefinition) -> Self {
        Self {
            rules: Vec::new(),
            modes: HashMap::new(),
            functions: Vec::new(),
            main,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBinding {
    pub name: Name,
    pub main: FunctionDefinition,
}
