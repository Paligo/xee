use ibig::error::OutOfBoundsError;
use strum::EnumMessage;
use strum_macros::{Display, EnumMessage};
use xee_xpath_ast::ParserError;
use xot::xmlname::NameStrInfo;

use crate::span::SourceSpan;

/// An error code with an optional source span.
///
/// Also known as `SpannedError` internally.
#[derive(Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SpannedError {
    /// The error code
    pub error: Error,
    /// The source span where the error occurred
    pub span: Option<SourceSpan>,
}

/// XPath/XSLT error code
///
/// These are specified by the XPath and XSLT specifications.
///
/// Xee extends them with a few additional error codes.
///
/// Also known as `Error` internally.
#[derive(Debug, Clone, PartialEq, Display, EnumMessage)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Error {
    /// Stack overflow.
    ///
    /// Internal stack overflow.
    StackOverflow,

    /// Unsupported XPath feature.
    ///
    /// This XPath feature is not supported by Xee.
    Unsupported,

    /// Used query with wrong queries.
    ///
    /// The query was created with a different queries collection.
    UsedQueryWithWrongQueries,

    // XPath error conditions: https://www.w3.org/TR/xpath-31/#id-errors
    /// Component absent in static context.
    ///  
    /// It is a static error if analysis of an expression relies on some
    /// component of the static context that is absent.
    XPST0001,
    /// Component absent in dynamic context.
    ///
    /// It is a dynamic error if evaluation of an expression relies on some
    /// part of the dynamic context that is absent.
    XPDY0002,
    /// Parse error.
    ///
    /// It is a static error if an expression is not a valid instance of the
    /// grammar defined in A.1 EBNF.
    XPST0003,
    /// Type error.
    ///
    /// It is a type error if, during the static analysis phase, an expression
    /// is found to have a static type that is not appropriate for the context
    /// in which the expression occurs, or during the dynamic evaluation phase,
    /// the dynamic type of a value does not match a required type as specified
    /// by the matching rules in 2.5.5 SequenceType Matching.
    XPTY0004,
    /// Empty Sequence type error.
    ///
    /// During the analysis phase, it is a static error if the static type
    /// assigned to an expression other than the expression `()` or `data(())`
    /// is `empty-sequence()`.
    XPST0005,
    /// Name not defined.
    ///
    /// It is a static error if an expression refers to an element name,
    /// attribute name, schema type name, namespace prefix, or variable name
    /// that is not defined in the static context, except for an ElementName in
    /// an ElementTest or an AttributeName in an AttributeTest.
    XPST0008,
    /// Namespace axis not supported.
    ///
    /// An implementation that does not support the namespace axis must raise a
    /// static error if it encounters a reference to the namespace axis and
    /// XPath 1.0 compatibility mode is false.
    XPST0010,
    /// Type error: incorrect function name or number of arguments.
    ///
    /// It is a static error if the expanded QName and number of arguments in a
    /// static function call do not match the name and arity of a function
    /// signature in the static context.
    XPST0017,
    /// Type error: inconsistent sequence.
    ///
    /// It is a type error if the result of a path operator contains both nodes
    /// and non-nodes.
    XPTY0018,
    /// Type error: path operator must be applied to node sequence
    ///
    /// It is a type error if E1 in a path expression E1/E2 does not evaluate to a
    /// sequence of nodes.
    XPTY0019,
    /// Type error: context item is not a node in an axis step.
    ///
    /// It is a type error if, in an axis step, the context item is not a node.
    XPTY0020,
    /// Multiple parameters with same name.
    ///
    /// It is a static error for an inline function expression to have more
    /// than one parameter with the same name.
    XQST0039,
    /// Invalid Braced URI Literal.
    ///
    /// An implementation MAY raise a static error if the value of a
    /// BracedURILiteral is of nonzero length and is neither an absolute URI
    /// nor a relative URI.
    XQST0046,
    /// Treat type does not match sequence type.
    ///
    /// It is a dynamic error if the dynamic type of the operand of a treat
    /// expression does not match the sequence type specified by the treat
    /// expression. This error might also be raised by a path expression
    /// beginning with "/" or "//" if the context node is not in a tree that is
    /// rooted at a document node. This is because a leading "/" or "//" in a
    /// path expression is an abbreviation for an initial step that includes
    /// the clause `treat as document-node()`.
    XPDY0050,
    /// Undefined type reference
    ///
    /// It is a static error if the expanded QName for an AtomicOrUnionType in
    /// a SequenceType is not defined in the in-scope schema types as a
    /// generalized atomic type.
    XPST0051,
    /// Invalid type named in cast or castable expression.
    ///
    /// The type named in a cast or castable expression must be the name of a
    /// type defined in the in-scope schema types, and the type must be simple.
    XQST0052,
    /// Illegal prefix
    ///
    /// A static error is raised if any of the following conditions is
    /// statically detected in any expression:
    ///
    /// - The prefix xml is bound to some namespace URI other than
    ///   `http://www.w3.org/XML/1998/namespace`.
    /// - A prefix other than xml is bound to the namespace URI
    ///   `http://www.w3.org/XML/1998/namespace`.
    /// - The prefix xmlns is bound to any namespace URI.
    /// - A prefix other than xmlns is bound to the namespace URI
    ///   `http://www.w3.org/2000/xmlns/`.
    XQST0070,
    /// Invalid target type of cast or castable expression.
    ///
    /// It is a static error if the target type of a cast or castable
    /// expression is xs:NOTATION, xs:anySimpleType, or xs:anyAtomicType.
    XPST0080,
    /// Unknown namespace prefix.
    ///
    /// It is a static error if a QName used in an expression contains a
    /// namespace prefix that cannot be expanded into a namespace URI by using
    /// the statically known namespaces.
    XPST0081,
    /// Type error: namespace-sensitive type expected.
    ///
    /// When applying the function conversion rules, if an item is of type
    /// xs:untypedAtomic and the expected type is namespace-sensitive, a type
    /// error is raised.
    XPTY0117,
    /// Implementation-dependent limit exceeded.
    ///
    /// An implementation-dependent limit has been exceeded.
    XPDY0130,
    /// Namespace axis not supported.
    ///
    /// The namespace axis is not supported.
    XQST0134,
    /// Duplicate key values in a map.
    ///
    /// No two keys in a map may have the same key value.
    XQDY0137,
    // XPath errors and functions: https://www.w3.org/TR/xpath-functions-31/#error-summary
    /// Wrong number of arguments.
    ///
    /// Raised when fn:apply is called and the arity of the supplied function
    /// is not the same as the number of members in the supplied array.
    FOAP0001,
    /// Division by zero.
    ///
    /// This error is raised whenever an attempt is made to divide by zero.
    FOAR0001,
    /// Numeric operation overflow/underflow.
    ///
    /// This error is raised whenever numeric operations result in an overflow or underflow.
    FOAR0002,
    /// Array index out of bounds.
    ///
    /// This error is raised when an integer used to select a member of an array is outside the range of values for that array.
    FOAY0001,
    /// Negative array length.
    ///
    /// This error is raised when the $length argument to array:subarray is negative.
    FOAY0002,
    /// Input value too large for decimal.
    ///
    /// Raised when casting to xs:decimal if the supplied value exceeds the implementation-defined limits for the datatype.
    FOCA0001,
    /// Invalid lexical value.
    ///
    /// Raised by fn:resolve-QName and fn:QName when a supplied value does not
    /// have the lexical form of a QName or URI respectively; and when casting
    /// to decimal, if the supplied value is NaN or Infinity.
    FOCA0002,
    /// Input too large for integer.
    ///
    /// Raised when casting to xs:integer if the supplied value exceeds the implementation-defined limits for the datatype.
    FOCA0003,
    /// NaN supplied as float/double value.
    ///
    /// Raised when multiplying or dividing a duration by a number, if the
    /// number supplied is NaN.
    FOCA0005,
    /// String to be cast to decimal has too many digits of precision.
    ///
    /// Raised when casting a string to xs:decimal if the string has more
    /// digits of precision than the implementation can represent (the
    /// implementation also has the option of rounding).
    FOCA0006,
    /// Codepoint not valid.
    ///
    /// Raised by fn:codepoints-to-string if the input contains an integer that is not the codepoint of a valid XML character.
    FOCH0001,
    /// Unsupported collation.
    ///
    /// Raised by any function that uses a collation if the requested collation
    /// is not recognized.
    FOCH0002,
    /// Unsupported normalization form.
    ///
    /// Raised by fn:normalize-unicode if the requested normalization form is
    /// not supported by the implementation.
    FOCH0003,
    /// Collation does not support collation units.
    ///
    /// Raised by functions such as fn:contains if the requested collation does
    /// not operate on a character-by-character basis.
    FOCH0004,
    /// No context document.
    ///
    /// Raised by fn:id, fn:idref, and fn:element-with-id if the node that
    /// identifies the tree to be searched is a node in a tree whose root is
    /// not a document node.
    FODC0001,
    /// Error retrieving resource.
    ///
    /// Raised by fn:doc, fn:collection, and fn:uri-collection to indicate that
    /// either the supplied URI cannot be dereferenced to obtain a resource, or
    /// the resource that is returned is not parseable as XML.
    FODC0002,
    /// Function not defined as deterministic.
    ///
    /// Raised by fn:doc, fn:collection, and fn:uri-collection to indicate that
    /// it is not possible to return a result that is guaranteed deterministic.
    FODC0003,
    /// Invalid collection URI.
    ///
    /// Raised by fn:collection and fn:uri-collection if the argument is not
    /// a valid xs:anyURI.
    FODC0004,
    /// Invalid argument to fn:doc or fn:doc-available.
    ///
    /// Raised (optionally) by fn:doc and fn:doc-available if the argument is
    /// not a valid URI reference.
    FODC0005,
    /// String passed to fn:parse-xml is not a well-formed XML document.
    ///
    /// Raised by fn:parse-xml if the supplied string is not a well-formed and
    /// namespace-well-formed XML document; or if DTD validation is requested
    /// and the document is not valid against its DTD.
    FODC0006,
    /// The processor does not support serialization.
    ///
    /// Raised when fn:serialize is called and the processor does not support
    /// serialization, in cases where the host language makes serialization an
    /// optional feature.
    FODC0010,
    /// Invalid decimal format name.
    ///
    /// This error is raised if the decimal format name supplied to
    /// fn:format-number is not a valid QName, or if the prefix in the QName is
    /// undeclared, or if there is no decimal format in the static context with
    /// a matching name.
    FODF1280,
    /// Invalid decimal format picture string.
    ///
    /// This error is raised if the picture string supplied to fn:format-number
    /// or fn:format-integer has invalid syntax.
    FODF1310,
    /// Overflow/underflow in date/time operation.
    ///
    /// Raised when casting to date/time datatypes, or performing arithmetic
    /// with date/time values, if arithmetic overflow or underflow occurs.
    FODT0001,
    /// err:FODT0002, Overflow/underflow in duration operation.
    ///
    /// Raised when casting to duration datatypes, or performing arithmetic
    /// with duration values, if arithmetic overflow or underflow occurs.
    FODT0002,
    /// Invalid timezone value.
    ///
    /// Raised by adjust-date-to-timezone and related functions if the supplied
    /// timezone is invalid.
    FODT0003,
    /// Unidentified error.
    ///
    /// Error code used by fn:error when no other error code is provided.
    FOER0000,
    /// Invalid date/time formatting parameters.
    ///
    /// This error is raised if the picture string or calendar supplied to
    /// fn:format-date, fn:format-time, or fn:format-dateTime has invalid
    /// syntax.
    FOFD1340,
    /// Invalid date/time formatting component.
    ///
    /// This error is raised if the picture string supplied to fn:format-date
    /// selects a component that is not present in a date, or if the picture
    /// string supplied to fn:format-time selects a component that is not
    /// present in a time.
    FOFD1350,
    /// JSON syntax error.
    ///
    /// Raised by functions such as fn:json-doc, fn:parse-json or
    /// fn:json-to-xml if the string supplied as input does not conform to the
    /// JSON grammar (optionally with implementation-defined extensions).
    FOJS0001,
    /// JSON duplicate keys.
    ///
    /// Raised by functions such as map:merge, fn:json-doc, fn:parse-json or
    /// fn:json-to-xml if the input contains duplicate keys, when the chosen
    /// policy is to reject duplicates.
    FOJS0003,
    /// JSON: not schema-aware.
    ///
    /// Raised by fn:json-to-xml if validation is requested when the processor
    /// does not support schema validation or typed nodes.
    FOJS0004,
    /// Invalid options.
    ///
    /// Raised by functions such as map:merge, fn:parse-json, and
    /// fn:xml-to-json if the $options map contains an invalid entry.
    FOJS0005,
    /// Invalid XML representation of JSON.
    ///
    /// Raised by fn:xml-to-json if the XML input does not conform to the rules
    /// for the XML representation of JSON.
    FOJS0006,
    /// Bad JSON escape sequence.
    ///
    /// Raised by fn:xml-to-json if the XML input uses the attribute
    /// escaped="true" or escaped-key="true", and the corresponding string or
    /// key contains an invalid JSON escape sequence.
    FOJS0007,
    /// No namespace found for prefix.
    ///
    /// Raised by fn:resolve-QName and analogous functions if a supplied QName
    /// has a prefix that has no binding to a namespace.
    FONS0004,
    /// Base-uri not defined in the static context.
    ///
    /// Raised by fn:resolve-uri if no base URI is available for resolving a
    /// relative URI.
    FONS0005,
    /// Module URI is a zero-length string.
    ///
    /// Raised by fn:load-xquery-module if the supplied module URI is zero-length.
    FOQM0001,
    /// Module URI not found.
    ///
    /// Raised by fn:load-xquery-module if no module can be found with the
    /// supplied module URI.
    FOQM0002,
    /// Static error in dynamically-loaded XQuery module.
    ///
    /// Raised by fn:load-xquery-module if a static error (including a
    /// statically-detected type error) is encountered when processing the
    /// library module.
    FOQM0003,
    /// Parameter for dynamically-loaded XQuery module has incorrect type.
    ///
    /// Raised by fn:load-xquery-module if a value is supplied for the initial
    /// context item or for an external variable, and the value does not
    /// conform to the required type declared in the dynamically loaded module.
    FOQM0005,
    /// No suitable XQuery processor available.
    ///
    /// Raised by fn:load-xquery-module if no XQuery processor is available
    /// supporting the requested XQuery version (or if none is available at
    /// all).
    FOQM0006,
    /// Invalid value for cast/constructor.
    ///
    /// A general-purpose error raised when casting, if a cast between two
    /// datatypes is allowed in principle, but the supplied value cannot be
    /// converted: for example when attempting to cast the string "nine" to an
    /// integer.
    FORG0001,
    /// Invalid argument to fn:resolve-uri().
    ///
    /// Raised when either argument to fn:resolve-uri is not a valid URI/IRI.
    FORG0002,
    /// fn:zero-or-one called with a sequence containing more than one item.
    ///
    /// Raised by fn:zero-or-one if the supplied value contains more than one item.
    FORG0003,
    /// fn:one-or-more called with a sequence containing no items.
    ///
    /// Raised by fn:one-or-more if the supplied value is an empty sequence.
    FORG0004,
    /// fn:exactly-one called with a sequence containing zero or more than one item.
    ///
    /// Raised by fn:exactly-one if the supplied value is not a singleton sequence.
    FORG0005,
    /// Invalid argument type.
    ///
    /// Raised by functions such as fn:max, fn:min, fn:avg, fn:sum if the
    /// supplied sequence contains values inappropriate to this function.
    FORG0006,
    /// The two arguments to fn:dateTime have inconsistent timezones.
    ///
    /// Raised by fn:dateTime if the two arguments both have timezones and the
    /// timezones are different.
    FORG0008,
    /// Error in resolving a relative URI against a base URI in fn:resolve-uri.
    ///
    /// A catch-all error for fn:resolve-uri, recognizing that the
    /// implementation can choose between a variety of algorithms and that some
    /// of these may fail for a variety of reasons.
    FORG0009,
    /// Invalid date/time.
    ///
    /// Raised when the input to fn:parse-ietf-date does not match the
    /// prescribed grammar, or when it represents an invalid date/time such as
    /// 31 February.
    FORG0010,
    /// Invalid regular expression flags.
    ///
    /// Raised by regular expression functions such as fn:matches and
    /// fn:replace if the regular expression flags contain a character other
    /// than i, m, q, s, or x.
    FORX0001,
    /// Invalid regular expression.
    ///
    /// Raised by regular expression functions such as fn:matches and
    /// fn:replace if the regular expression is syntactically invalid.
    FORX0002,
    /// Regular expression matches zero-length string.
    ///
    /// For functions such as fn:replace and fn:tokenize, raises an error if
    /// the supplied regular expression is capable of matching a zero length
    /// string.
    FORX0003,
    /// Invalid replacement string.
    ///
    /// Raised by fn:replace to report errors in the replacement string.
    FORX0004,
    /// Argument to fn:data() contains a node that does not have a typed value.
    ///
    /// Raised by fn:data, or by implicit atomization, if applied to a node
    /// with no typed value, the main example being an element validated
    /// against a complex type that defines it to have element-only content.
    FOTY0012,
    /// The argument to fn:data() contains a function item.
    ///
    /// Raised by fn:data, or by implicit atomization, if the sequence to be
    /// atomized contains a function item.
    FOTY0013,
    /// The argument to fn:string() is a function item.
    ///
    /// Raised by fn:string, or by implicit string conversion, if the input
    /// sequence contains a function item.
    FOTY0014,
    /// An argument to fn:deep-equal() contains a function item.
    ///
    /// Raised by fn:deep-equal if either input sequence contains a function
    /// item.
    FOTY0015,
    /// Invalid $href argument to fn:unparsed-text() (etc.)
    ///
    /// A dynamic error is raised if the $href argument contains a fragment
    /// identifier, or if it cannot be used to retrieve a resource containing
    /// text.
    FOUT1170,
    /// Cannot decode resource retrieved by fn:unparsed-text() (etc.)
    ///
    /// A dynamic error is raised if the retrieved resource contains octets
    /// that cannot be decoded into Unicode ·characters· using the specified
    /// encoding, or if the resulting characters are not permitted XML
    /// characters. This includes the case where the processor does not support
    /// the requested encoding.
    FOUT1190,
    /// Cannot infer encoding of resource retrieved by fn:unparsed-text()
    /// (etc.)
    ///
    /// A dynamic error is raised if $encoding is absent and the processor
    /// cannot infer the encoding using external information and the encoding
    /// is not UTF-8.
    FOUT1200,
    /// No suitable XSLT processor available
    ///
    /// A dynamic error is raised if no XSLT processor suitable for evaluating
    /// a call on fn:transform is available.
    FOXT0001,
    /// Invalid parameters to XSLT transformation
    ///
    /// A dynamic error is raised if the parameters supplied to fn:transform
    /// are invalid, for example if two mutually-exclusive parameters are
    /// supplied. If a suitable XSLT error code is available (for example in
    /// the case where the requested initial-template does not exist in the
    /// stylesheet), that error code should be used in preference.
    FOXT0002,
    /// XSLT transformation failed
    ///
    /// A dynamic error is raised if an XSLT transformation invoked using
    /// fn:transform fails with a static or dynamic error. The XSLT error code
    /// is used if available; this error code provides a fallback when no XSLT
    /// error code is returned, for example because the processor is an XSLT
    /// 1.0 processor.
    FOXT0003,
    /// XSLT transformation has been disabled
    ///
    /// A dynamic error is raised if the fn:transform function is invoked when
    /// XSLT transformation (or a specific transformation option) has been
    /// disabled for security or other reasons.
    FOXT0004,
    /// XSLT output contains non-accepted characters
    ///
    /// A dynamic error is raised if the result of the fn:transform function
    /// contains characters available only in XML 1.1 and the calling processor
    /// cannot handle such characters.
    FOXT0006,

    /// Duplicate global variable name.
    ///
    /// It is a static error if a package contains more than one non-hidden
    /// binding of a global variable with the same name and same import
    /// precedence, unless it also contains another binding with the same name
    /// and higher import precedence.
    XTSE0630,
    /// Circularity
    ///
    /// Circularity in global declarations is now allowed.
    XTDE0640,
    /// Shallow copy
    ///
    /// Shallow copy of sequence of more than one item is not allowed.
    XTTE3180,
    /// Function item in complex content
    ///
    /// The result sequence to be added as content cannot contain a function
    /// item.
    XTDE0450,

    /// Function cannot be normalized for serialization.
    ///
    /// It is an error if an item in S in sequence normalization is an
    /// attribute node or a namespace node.
    SENR0001,

    /// Entity serialization error
    ///
    /// The serializer is unable to satisfy the rules for either a well-formed
    /// XML document entity or a well-formed XML external general parsed
    /// entity, or both, except for content modified by the character expansion
    /// phase of serialization.
    SERE0003,

    /// Standalone or doctype-system parameter disallowed for XML fragment.
    ///
    /// It's not allowed to specify the doctype-system parameter, or to specify
    /// the standalone parameter with a value other than omit, if the instance
    /// of the data model contains text nodes or multiple element nodes as
    /// children of the root node.
    SEPM0004,

    /// Invalid character in NCName according to namespaces version.
    ///
    /// It is an error if the serialized result would contain an NCNameNames
    /// that contains a character that is not permitted by the version of
    /// Namespaces in XML specified by the version parameter.
    SERE0005,

    /// Invalid character according to XML version
    ///
    /// It is an error if the serialized result would contain a character that
    /// is not permitted by the version of XML specified by the version
    /// parameter.
    SERE0006,

    /// Invalid encoding
    ///
    /// It is an error if an output encoding other than UTF-8 or UTF-16 is
    /// requested and the serializer does not support that encoding.
    SESU0007,

    /// Illegal character for encoding
    ///
    /// It is an error if a character that cannot be represented in the
    /// encoding that the serializer is using for output appears in a context
    /// where character references are not allowed (for example if the
    /// character occurs in the name of an element).
    SERE0008,

    /// standalone even though XML declaration is omitted
    ///
    /// It is an error if the omit-xml-declaration parameter has the value yes,
    /// true or 1, and the standalone attribute has a value other than omit; or
    /// the version parameter has a value other than 1.0 and the doctype-system
    /// parameter is specified.
    SEPM0009,

    /// undeclare-prefixes is not allowed in XML version 1.0
    ///
    /// It is an error if the output method is xml or xhtml, the value of the
    /// undeclare-prefixes parameter is one of, yes, true or 1, and the value
    /// of the version parameter is 1.0.
    SEPM0010,

    /// Unsupported normalization form
    ///
    /// It is an error if the value of the normalization-form parameter
    /// specifies a normalization form that is not supported by the serializer.
    SESU0011,

    /// Combining character at start of fully-normalized result
    ///
    /// It is an error if the value of the normalization-form parameter is
    /// fully-normalized and any relevant construct of the result begins with a
    /// combining character.
    SERE0012,

    /// Unsupported version
    ///
    /// It is an error if the serializer does not support the version of XML or
    /// HTML specified by the version parameter.
    SESU0013,

    /// Illegal characters in HTML output.
    ///
    /// It is an error to use the HTML output method if characters which are
    /// permitted in XML but not in HTML appear in the instance of the data
    /// model.
    SERE0014,

    /// Illegal characters in processing instruction for HTML output.
    ///
    /// It is an error to use the HTML output method when > appears within a
    /// processing instruction in the data model instance being serialized.
    SERE0015,

    /// Parameter value is invalid for the defined domain.
    SEPM0016,

    /// Error evaluating serialization parameter expression.
    ///
    /// It is an error if evaluating an expression in order to extract the
    /// setting of a serialization parameter from a data model instance would
    /// yield an error.
    SEPM0017,

    /// Multiple values for use-character-maps serialization parameter.
    ///
    /// It is an error if evaluating an expression in order to extract the
    /// setting of the use-character-maps serialization parameter from a data
    /// model instance would yield a sequence of length greater than one.
    SEPM0018,

    /// Multiple values for serialization parameter.
    ///
    /// It is an error if an instance of the data model used to specify the
    /// settings of serialization parameters specifies the value of the same
    /// parameter more than once.
    SEPM0019,

    /// Invalid numeric value in JSON.
    ///
    /// It is an error if a numeric value being serialized using the JSON
    /// output method cannot be represented in the JSON grammar (e.g. +INF,
    /// -INF, NaN).
    SERE0020,

    /// Item not allowed in JSON output.
    ///
    /// It is an error if a sequence being serialized using the JSON output
    /// method includes items for which no rules are provided in the
    /// appropriate section of the serialization rules.
    SERE0021,

    /// Duplicate key in JSON output.
    ///
    /// It is an error if a map being serialized using the JSON output method
    /// has two keys with the same string value, unless the
    /// allow-duplicate-names has the value yes, true or 1.
    SERE0022,

    /// Sequence of length greater than one in JSON output.
    ///
    /// It is an error if a sequence being serialized using the JSON output
    /// method is of length greater than one.
    SERE0023,

    /// An application generated error
    Application(Box<ApplicationError>),
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ApplicationError {
    qname: xot::xmlname::OwnedName,
    description: String,
    // FIXME: error object is not supported right now
    // it would require storing an arbitrary sequence in here,
    // but that's not really supported by this simple error.
}

impl ApplicationError {
    pub fn new(qname: xot::xmlname::OwnedName, description: String) -> Self {
        Self { qname, description }
    }
}

impl Error {
    pub fn with_span(self, span: SourceSpan) -> SpannedError {
        SpannedError {
            error: self,
            span: Some(span),
        }
    }
    pub fn with_ast_span(self, span: xee_xpath_ast::ast::Span) -> SpannedError {
        Self::with_span(self, span.into())
    }

    pub fn code(&self) -> String {
        match self {
            Self::Application(application_error) => {
                application_error.qname.local_name().to_string()
            }
            _ => self.to_string(),
        }
    }

    pub fn code_qname(&self) -> xot::xmlname::OwnedName {
        match self {
            Self::Application(application_error) => application_error.qname.clone(),
            _ => xot::xmlname::OwnedName::new(
                self.code(),
                "http://www.w3.org/2005/xqt-errors".to_string(),
                "".to_string(),
            ),
        }
    }

    pub fn message(&self) -> &str {
        self.documentation_pieces().0
    }

    pub fn note(&self) -> &str {
        self.documentation_pieces().1
    }

    fn documentation_pieces(&self) -> (&str, &str) {
        if let Some(documentation) = self.get_documentation() {
            let mut pieces = documentation.splitn(2, "\n\n");
            let first = pieces.next().unwrap_or("");
            let second = pieces.next().unwrap_or("");
            (first, second)
        } else {
            ("", "")
        }
    }
}
impl std::error::Error for Error {}

impl std::fmt::Display for SpannedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = self.span {
            let span = span.range();
            write!(f, "{} ({}..{})", self.error, span.start, span.end)
        } else {
            write!(f, "{}", self.error)
        }
    }
}

impl std::error::Error for SpannedError {}

// note: this is only used for internal conversions of names
// for now, not the full grammar.
impl From<xee_xpath_ast::ParserError> for Error {
    fn from(e: xee_xpath_ast::ParserError) -> Self {
        let spanned_error: SpannedError = e.into();
        spanned_error.error
    }
}

impl From<xee_xpath_ast::ParserError> for SpannedError {
    fn from(e: xee_xpath_ast::ParserError) -> Self {
        let span = e.span();
        let error = match e {
            ParserError::ExpectedFound { .. } => Error::XPST0003,
            // this is what fn-function-arity-017 expects, even though
            // implementation limit exceeded (XPST00130) seems reasonable to me.
            ParserError::ArityOverflow { .. } => Error::FOAR0002,
            ParserError::Reserved { .. } => Error::XPST0003,
            ParserError::UnknownPrefix { .. } => Error::XPST0081,
            ParserError::UnknownType { .. } => Error::XPST0051,
            // TODO: this this the right error code?
            ParserError::IllegalFunctionInPattern { .. } => Error::XPST0003,
        };
        Self {
            error,
            span: Some(span.into()),
        }
    }
}

impl From<regexml::Error> for Error {
    fn from(e: regexml::Error) -> Self {
        use regexml::Error::*;
        // TODO: pass more error details into error codes
        match e {
            Internal => panic!("Internal error in regexml engine"),
            InvalidFlags(_) => Self::FORX0001,
            Syntax(_) => Self::FORX0002,
            MatchesEmptyString => Self::FORX0003,
            InvalidReplacementString(_) => Self::FORX0004,
        }
    }
}

impl From<xot::Error> for Error {
    fn from(e: xot::Error) -> Self {
        match e {
            xot::Error::MissingPrefix(_) => Self::XPST0081,
            // TODO: are there other xot errors that need to be translated?
            _ => Self::XPST0003,
        }
    }
}

impl From<Error> for SpannedError {
    fn from(e: Error) -> Self {
        Self {
            error: e,
            span: None,
        }
    }
}

// impl From<xee_name::Error> for Error {
//     fn from(e: xee_name::Error) -> Self {
//         match e {
//             xee_name::Error::MissingPrefix => Error::XPST0081,
//         }
//     }
// }

impl From<OutOfBoundsError> for Error {
    fn from(_e: OutOfBoundsError) -> Self {
        Self::FOCA0003
    }
}

/// The result type for errors without span information.
pub type Result<T> = std::result::Result<T, Error>;

/// The result type for errors with (optional) source spans.
///
/// Also known as `SpannedResult` internally.
pub type SpannedResult<T> = std::result::Result<T, SpannedError>;

impl SpannedError {
    /// get the underlying [`Error`] value
    pub fn value(self) -> Error {
        self.error
    }
}
