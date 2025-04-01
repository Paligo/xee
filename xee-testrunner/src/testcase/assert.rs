use ahash::AHashMap;
use chrono::Offset;
use std::fmt;
use std::path::Path;
use xee_xpath::context::{Collation, DynamicContext};
use xot::xmlname::{NameStrInfo, OwnedName as Name};
use xot::Xot;

use xee_xpath::query::RecurseQuery;
use xee_xpath::{context, error, Documents, Item, Queries, Query, Recurse, Sequence};
use xee_xpath_load::{convert_boolean, convert_string, ContextLoadable};

use crate::catalog::LoadContext;
use crate::ns::XPATH_TEST_NS;

use super::outcome::{TestOutcome, UnexpectedError};

type XPathExpr = String;

pub(crate) trait Assertable {
    fn assert_result(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        match result {
            Ok(sequence) => self.assert_value(context, documents, sequence),
            Err(error) => TestOutcome::RuntimeError(error.clone()),
        }
    }

    fn assert_value(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertAnyOf(Vec<TestCaseResult>);

impl AssertAnyOf {
    pub(crate) fn new(test_case_results: Vec<TestCaseResult>) -> Self {
        Self(test_case_results)
    }

    pub(crate) fn assert_error(&self, error: &error::ErrorValue) -> TestOutcome {
        let mut failed_test_results = Vec::new();
        for test_case_result in &self.0 {
            if let TestCaseResult::AssertError(assert_error) = test_case_result {
                let result = assert_error.assert_error(error);
                match result {
                    TestOutcome::Passed => return result,
                    _ => failed_test_results.push(result),
                }
            } else {
                // any non-error is a failure, as we arrived with an error
                return TestOutcome::Failed(Failure::AnyOf(self.clone(), failed_test_results));
            }
        }
        TestOutcome::Failed(Failure::AnyOf(self.clone(), failed_test_results))
    }
}

impl Assertable for AssertAnyOf {
    fn assert_result(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        let mut failed_test_results = Vec::new();
        for test_case_result in &self.0 {
            let result = test_case_result.assert_result(context, documents, result);
            match result {
                TestOutcome::Passed => return result,
                _ => failed_test_results.push(result),
            }
        }
        match result {
            Ok(_value) => TestOutcome::Failed(Failure::AnyOf(self.clone(), failed_test_results)),
            Err(error) => TestOutcome::RuntimeError(error.clone()),
        }
    }

    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        _sequence: &Sequence,
    ) -> TestOutcome {
        unreachable!();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssertAllOf(Vec<TestCaseResult>);

impl AssertAllOf {
    pub(crate) fn new(test_case_results: Vec<TestCaseResult>) -> Self {
        Self(test_case_results)
    }
}

impl Assertable for AssertAllOf {
    fn assert_result(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        for test_case_result in &self.0 {
            let result = test_case_result.assert_result(context, documents, result);
            match result {
                TestOutcome::Passed | TestOutcome::UnexpectedError(..) => {}
                _ => return result,
            }
        }
        TestOutcome::Passed
    }

    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        _sequence: &Sequence,
    ) -> TestOutcome {
        unreachable!();
    }
}

#[derive(PartialEq, Clone, Eq)]
pub struct AssertNot(Box<TestCaseResult>);

impl AssertNot {
    pub(crate) fn new(test_case_result: TestCaseResult) -> Self {
        Self(Box::new(test_case_result))
    }
}

impl Assertable for AssertNot {
    fn assert_result(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        let result = self.0.assert_result(context, documents, result);
        match result {
            TestOutcome::Passed => {
                TestOutcome::Failed(Failure::Not(self.clone(), Box::new(result)))
            }
            TestOutcome::Failed(_) => TestOutcome::Passed,
            _ => result,
        }
    }

    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        _sequence: &Sequence,
    ) -> TestOutcome {
        unreachable!();
    }
}

impl fmt::Debug for AssertNot {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AssertNot({:?})", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assert(XPathExpr);

impl Assert {
    pub(crate) fn new(expr: XPathExpr) -> Self {
        Self(expr)
    }
}

impl Assertable for Assert {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let result_sequence = run_xpath_with_result(&self.0, sequence, documents);

        match result_sequence {
            Ok(result_sequence) => match result_sequence.effective_boolean_value() {
                Ok(value) => {
                    if value {
                        TestOutcome::Passed
                    } else {
                        TestOutcome::Failed(Failure::Assert(self.clone(), sequence.clone()))
                    }
                }
                Err(error) => TestOutcome::RuntimeError(error),
            },
            Err(error) => TestOutcome::UnsupportedExpression(error.value()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertEq(XPathExpr);

impl AssertEq {
    pub(crate) fn new(expr: XPathExpr) -> Self {
        Self(expr)
    }
}

impl Assertable for AssertEq {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let expected_sequence = run_xpath(&self.0);

        match expected_sequence {
            Ok(expected_sequence) => {
                let atom = xee_xpath::iter::one(sequence.atomized(documents.xot()));
                let atom = match atom {
                    Ok(atom) => atom,
                    Err(error) => return TestOutcome::RuntimeError(error),
                };
                let expected_atom =
                    xee_xpath::iter::one(expected_sequence.atomized(documents.xot()))
                        .expect("Should get single atom in sequence");
                if expected_atom.simple_equal(&atom) {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed(Failure::Eq(self.clone(), sequence.clone()))
                }
            }
            Err(error) => TestOutcome::UnsupportedExpression(error.value()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertDeepEq(XPathExpr);

impl AssertDeepEq {
    pub(crate) fn new(expr: XPathExpr) -> Self {
        Self(expr)
    }
}

impl Assertable for AssertDeepEq {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let expected_sequence = run_xpath(&self.0);

        match expected_sequence {
            Ok(expected_sequence) => {
                if expected_sequence
                    .deep_equal(
                        sequence,
                        &Collation::CodePoint,
                        chrono::offset::Utc.fix(),
                        documents.xot(),
                    )
                    .unwrap_or(false)
                {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed(Failure::DeepEq(self.clone(), sequence.clone()))
                }
            }
            Err(error) => TestOutcome::UnsupportedExpression(error.value()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertCount(usize);

impl AssertCount {
    pub(crate) fn new(count: usize) -> Self {
        Self(count)
    }
}

impl Assertable for AssertCount {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let found_len = sequence.len();
        if found_len == self.0 {
            TestOutcome::Passed
        } else {
            TestOutcome::Failed(Failure::Count(self.clone(), AssertCountFailure(found_len)))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertPermutation(XPathExpr);

impl AssertPermutation {
    pub(crate) fn new(expr: XPathExpr) -> Self {
        Self(expr)
    }
}

impl Assertable for AssertPermutation {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        // we won't use the sequence sorting mechanisms defined in XPath
        // here, as in them we can't compare bools with integers for instance,
        // and that gives rise to a test failure because we cannot sort

        // we use a frequency comparison algorithm, by counting how
        // many times we see a particular atom in a hashmap, and checking
        // whether the result sequence has the same counts.

        let mut frequency = AHashMap::new();
        for atom in sequence.atomized(documents.xot()) {
            let atom = match atom {
                Ok(atom) => atom,
                Err(err) => return TestOutcome::RuntimeError(err),
            };
            let count = frequency.entry(atom).or_insert(0);
            *count += 1;
        }

        let result_sequence = run_xpath(&self.0);
        let result_sequence = match result_sequence {
            Ok(result_sequence) => result_sequence,
            Err(error) => return TestOutcome::UnsupportedExpression(error.value()),
        };

        for atom in result_sequence.atomized(documents.xot()) {
            let atom = match atom {
                Ok(atom) => atom,
                Err(err) => return TestOutcome::RuntimeError(err),
            };
            let count = frequency.entry(atom.clone()).or_insert(0);
            *count -= 1;
            if *count == 0 {
                frequency.remove(&atom);
            }
        }

        if frequency.is_empty() {
            TestOutcome::Passed
        } else {
            TestOutcome::Failed(Failure::Permutation(self.clone(), sequence.clone()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertXml(String);

impl AssertXml {
    pub(crate) fn new(xml: String) -> Self {
        Self(xml)
    }
}

impl Assertable for AssertXml {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let xml = serialize(documents.xot(), sequence);

        let xml = if let Ok(xml) = xml {
            xml
        } else {
            return TestOutcome::Failed(Failure::Xml(
                self.clone(),
                AssertXmlFailure::WrongValue(sequence.clone()),
            ));
        };
        // also wrap expected XML in a sequence element
        let expected_xml = format!("<sequence>{}</sequence>", self.0);

        let mut compare_xot = Xot::new();
        // now parse both with Xot
        let found = compare_xot.parse(&xml).unwrap();
        let expected = compare_xot.parse(&expected_xml).unwrap();

        // and compare
        let c = compare_xot.deep_equal(expected, found);

        if c {
            TestOutcome::Passed
        } else {
            TestOutcome::Failed(Failure::Xml(self.clone(), AssertXmlFailure::WrongXml(xml)))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertEmpty;

impl AssertEmpty {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Assertable for AssertEmpty {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        if sequence.is_empty() {
            TestOutcome::Passed
        } else {
            TestOutcome::Failed(Failure::Empty(self.clone(), sequence.clone()))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssertSerializationMatches;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssertSerializationError(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertType(String);

impl AssertType {
    pub(crate) fn new(type_name: String) -> Self {
        Self(type_name)
    }
}

impl Assertable for AssertType {
    fn assert_value(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let matches = sequence.matches_type(&self.0, documents.xot(), &|function| {
            context.function_info(function).signature()
        });
        match matches {
            Ok(matches) => {
                if matches {
                    TestOutcome::Passed
                } else {
                    TestOutcome::Failed(Failure::Type(self.clone(), sequence.clone()))
                }
            }
            Err(_) => {
                // we don't support this sequence type expression yet
                // this should resolve itself once we do and we can parse it
                TestOutcome::Unsupported
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertTrue;

impl AssertTrue {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Assertable for AssertTrue {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        if let Ok(item) = sequence.clone().one() {
            if let Ok(atomic) = item.to_atomic() {
                let b: error::ValueResult<bool> = atomic.try_into();
                if let Ok(b) = b {
                    if b {
                        return TestOutcome::Passed;
                    }
                }
            }
        }
        TestOutcome::Failed(Failure::True(self.clone(), sequence.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertFalse;

impl AssertFalse {
    pub(crate) fn new() -> Self {
        Self
    }
}

impl Assertable for AssertFalse {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        if let Ok(item) = sequence.clone().one() {
            if let Ok(atomic) = item.to_atomic() {
                let b: error::ValueResult<bool> = atomic.try_into();
                if let Ok(b) = b {
                    if !b {
                        return TestOutcome::Passed;
                    }
                }
            }
        }
        TestOutcome::Failed(Failure::False(self.clone(), sequence.clone()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertStringValue(String, bool);

impl AssertStringValue {
    pub(crate) fn new(string: String, normalize_space: bool) -> Self {
        Self(string, normalize_space)
    }
}

impl Assertable for AssertStringValue {
    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        documents: &mut Documents,
        sequence: &Sequence,
    ) -> TestOutcome {
        let items = sequence.iter();

        let strings = items
            .map(|item| item.string_value(documents.xot()))
            .collect::<error::ValueResult<Vec<_>>>();
        match strings {
            Ok(strings) => {
                let joined = strings.join(" ");
                let joined = if self.1 {
                    // normalize space
                    joined
                        .split_ascii_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                } else {
                    joined
                };
                if joined == self.0 {
                    TestOutcome::Passed
                } else {
                    // the string value is not what we expected
                    TestOutcome::Failed(Failure::StringValue(
                        self.clone(),
                        AssertStringValueFailure::WrongStringValue(joined),
                    ))
                }
            }
            // we weren't able to produce a string value
            Err(_) => TestOutcome::Failed(Failure::StringValue(
                self.clone(),
                AssertStringValueFailure::WrongValue(sequence.clone()),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertError(String);

impl AssertError {
    pub(crate) fn new(code: String) -> Self {
        Self(code)
    }

    pub(crate) fn assert_error(&self, error: &error::ErrorValue) -> TestOutcome {
        if self.0 == "*" {
            return TestOutcome::Passed;
        }
        // all errors are officially a pass, but we check whether the error
        // code matches too
        let code = error.code_qname();
        // FIXME: there is no checking for the correct namespace here, should
        // there be?
        if code.local_name() == self.0 {
            TestOutcome::Passed
        } else {
            TestOutcome::UnexpectedError(UnexpectedError(code.local_name().to_string()))
        }
    }
}

impl Assertable for AssertError {
    fn assert_result(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        match result {
            Ok(sequence) => TestOutcome::Failed(Failure::Error(self.clone(), sequence.clone())),
            Err(error) => self.assert_error(error),
        }
    }

    fn assert_value(
        &self,
        _context: &DynamicContext<'_>,
        _documents: &mut Documents,
        _sequence: &Sequence,
    ) -> TestOutcome {
        unreachable!();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TestCaseResult {
    AnyOf(AssertAnyOf),
    AllOf(AssertAllOf),
    Not(AssertNot),
    // The assert element contains an XPath expression whose effective boolean
    // value must be true; usually the expression will use the variable $result
    // which references the result of the expression.
    Assert(Assert),
    // The assert element contains an XPath expression (usually a simple string
    // or numeric literal) which must be equal to the result of the test case
    // under the rules of the XPath 'eq' operator.
    AssertEq(AssertEq),
    // Asserts that the result must be a sequence of atomic values that is
    // deep-equal to the supplied sequence under the rules of the deep-equal()
    // function.
    AssertDeepEq(AssertDeepEq),
    // Asserts that the result must be a sequence containing a given number of
    // items. The value of the element is an integer giving the expected length
    // of the sequence.
    AssertCount(AssertCount),
    //  Asserts that the result must be a sequence of atomic values that has
    //  some permutation (reordering) that is deep-equal to the supplied
    //  sequence under the rules of the deep-equal() function.
    // Note this implies that NaN is equal to NaN.
    AssertPermutation(AssertPermutation),
    // Asserts the result of the query by providing a serialization of the
    // expression result using the default serialization parameters
    // method="xml" indent="no" omit-xml-declaration="yes".
    AssertXml(AssertXml),
    //  Asserts that the result of the test is an empty sequence.
    AssertEmpty(AssertEmpty),
    // Asserts the result of serializing the query matches a given regular
    // expression.
    // XXX values not right
    #[allow(dead_code)]
    SerializationMatches(AssertSerializationMatches),
    // Asserts that the query can be executed without error, but serializing
    // the result produces a serialization error. The result of the query must
    // be serialized using the serialization options specified within the query
    // (if any).
    #[allow(dead_code)]
    AssertSerializationError(AssertSerializationError),
    // Asserts that the result of the test matches the sequence type given as
    // the value of the assert-type element.
    AssertType(AssertType),
    // Asserts that the result of the test is the singleton boolean value
    // false(). Note, the test expression must actually evaluate to false: this
    // is not an assertion on the effective boolean value.
    AssertTrue(AssertTrue),
    // Asserts that the result of the test is the singleton boolean value
    // false(). Note, the test expression must actually evaluate to false: this
    // is not an assertion on the effective boolean value.
    AssertFalse(AssertFalse),
    // Asserts that the result of the test, after conversion to a string by
    // applying the expression string-join(for $r in $result return string($r),
    // " ") is equal to the string value of the assert-string-value element.
    // Note that this test cannot be used if the result includes items that do
    // not have a string value (elements with element-only content; function
    // items) If the normalize-space attribute is present with the value true,
    // then both the string value of the query result and the value of the
    // assert-string-value element should be processed as if by the XPath
    // normalize-space() function before the comparison.
    AssertStringValue(AssertStringValue),
    //  Asserts that the test is expected to fail with a static or dynamic
    //  error condition. The "code" attribute gives the expected error code.
    //
    // For the purpose of official test reporting, an implementation is
    // considered to pass a test if the test expects and error and the
    // implementation raises an error, regardless whether the error codes
    // match.
    AssertError(AssertError),
    // This assertion type is as of yet unsupported, and will automatically error
    Unsupported,
}

impl TestCaseResult {
    pub(crate) fn assert_result(
        &self,
        context: &DynamicContext<'_>,
        documents: &mut Documents,
        result: &error::ValueResult<Sequence>,
    ) -> TestOutcome {
        match self {
            TestCaseResult::AnyOf(a) => a.assert_result(context, documents, result),
            TestCaseResult::AllOf(a) => a.assert_result(context, documents, result),
            TestCaseResult::Not(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertEq(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertDeepEq(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertTrue(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertFalse(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertCount(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertStringValue(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertXml(a) => a.assert_result(context, documents, result),
            TestCaseResult::Assert(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertPermutation(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertError(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertEmpty(a) => a.assert_result(context, documents, result),
            TestCaseResult::AssertType(a) => a.assert_result(context, documents, result),
            TestCaseResult::Unsupported => TestOutcome::Unsupported,
            _ => {
                panic!("unimplemented test case result {:?}", self);
            }
        }
    }
}

impl ContextLoadable<LoadContext> for TestCaseResult {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(
        queries: &Queries,
        context: &LoadContext,
    ) -> anyhow::Result<impl Query<Self>> {
        let code_query = queries.one("@code/string()", convert_string)?;
        let error_query = queries.one(".", move |documents, item| {
            Ok(TestCaseResult::AssertError(AssertError::new(
                code_query.execute(documents, item)?,
            )))
        })?;
        let assert_count_query = queries.one("string()", |_, item| {
            let count: String = item.to_atomic()?.try_into()?;
            // XXX unwrap is a hack
            let count = count.parse::<usize>().unwrap();
            Ok(TestCaseResult::AssertCount(AssertCount::new(count)))
        })?;

        let assert_xml_query = queries.one("string()", |_, item| {
            let xml: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::AssertXml(AssertXml::new(xml)))
        })?;

        let assert_eq_query = queries.one("string()", |_, item| {
            let eq: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::AssertEq(AssertEq::new(eq)))
        })?;

        let assert_deep_eq_query = queries.one("string()", |_, item| {
            let eq: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::AssertDeepEq(AssertDeepEq::new(eq)))
        })?;

        let string_value_contents = queries.one("string()", convert_string)?;
        let normalize_space_query = queries.option("@normalize-space/string()", convert_boolean)?;

        let assert_string_value_query = queries.one(".", move |documents, item| {
            let string_value = string_value_contents.execute(documents, item)?;
            let normalize_space = normalize_space_query
                .execute(documents, item)?
                .unwrap_or(false);
            Ok(TestCaseResult::AssertStringValue(AssertStringValue::new(
                string_value,
                normalize_space,
            )))
        })?;

        let assert_type_query = queries.one("string()", |_, item| {
            let string_value: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::AssertType(AssertType::new(string_value)))
        })?;

        let assert_query = queries.one("string()", |_, item| {
            let xpath: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::Assert(Assert::new(xpath)))
        })?;

        let assert_permutation_query = queries.one("string()", |_, item| {
            let xpath: String = item.to_atomic()?.try_into()?;
            Ok(TestCaseResult::AssertPermutation(AssertPermutation::new(
                xpath,
            )))
        })?;

        let any_all_recurse = queries.many_recurse("*")?;
        let not_recurse = queries.one_recurse("*")?;

        // we use a local-name query here as it's the easiest way support this:
        // there is a single entry in the "result" element, but this may be
        // "any-of" and this contains a list of entries Using a relative path with
        // `query.option()` to detect entries (like "error", "assert-true", etc)
        // doesn't work for "any-of", as it contains a list of entries.
        let local_name_query = queries.one("local-name()", convert_string)?;
        let result_query =
            queries.one("result/*", move |documents: &mut Documents, item: &Item| {
                let f =
                    |documents: &mut Documents, item: &Item, recurse: &Recurse<TestCaseResult>| {
                        let local_name = local_name_query.execute(documents, item)?;
                        let r = match local_name.as_ref() {
                            "any-of" => {
                                let contents = any_all_recurse.execute(documents, item, recurse)?;
                                TestCaseResult::AnyOf(AssertAnyOf::new(contents))
                            }
                            "all-of" => {
                                let contents = any_all_recurse.execute(documents, item, recurse)?;
                                TestCaseResult::AllOf(AssertAllOf::new(contents))
                            }
                            "not" => {
                                let contents = not_recurse.execute(documents, item, recurse)?;
                                TestCaseResult::Not(AssertNot::new(contents))
                            }
                            "error" => error_query.execute(documents, item)?,
                            "assert-true" => TestCaseResult::AssertTrue(AssertTrue::new()),
                            "assert-false" => TestCaseResult::AssertFalse(AssertFalse::new()),
                            "assert-count" => assert_count_query.execute(documents, item)?,
                            "assert-xml" => assert_xml_query.execute(documents, item)?,
                            "assert-eq" => assert_eq_query.execute(documents, item)?,
                            "assert-deep-eq" => assert_deep_eq_query.execute(documents, item)?,
                            "assert-string-value" => {
                                assert_string_value_query.execute(documents, item)?
                            }
                            "assert" => assert_query.execute(documents, item)?,
                            "assert-permutation" => {
                                assert_permutation_query.execute(documents, item)?
                            }
                            "assert-empty" => TestCaseResult::AssertEmpty(AssertEmpty::new()),
                            "assert-type" => assert_type_query.execute(documents, item)?,
                            _ => TestCaseResult::Unsupported,
                        };
                        Ok(r)
                    };
                let recurse = Recurse::new(&f);
                recurse.execute(documents, item)
            })?;
        Ok(result_query)
    }
}
#[derive(Debug, PartialEq)]
pub struct AssertCountFailure(usize);

#[derive(Debug, PartialEq)]
pub enum AssertStringValueFailure {
    WrongStringValue(String),
    WrongValue(Sequence),
}

#[derive(Debug, PartialEq)]
pub enum AssertXmlFailure {
    WrongXml(String),
    WrongValue(Sequence),
}

#[derive(Debug, PartialEq)]
pub enum Failure {
    AnyOf(AssertAnyOf, Vec<TestOutcome>),
    Not(AssertNot, Box<TestOutcome>),
    Eq(AssertEq, Sequence),
    DeepEq(AssertDeepEq, Sequence),
    True(AssertTrue, Sequence),
    False(AssertFalse, Sequence),
    Count(AssertCount, AssertCountFailure),
    StringValue(AssertStringValue, AssertStringValueFailure),
    Xml(AssertXml, AssertXmlFailure),
    Assert(Assert, Sequence),
    Permutation(AssertPermutation, Sequence),
    Empty(AssertEmpty, Sequence),
    Error(AssertError, Sequence),
    Type(AssertType, Sequence),
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Failure::AnyOf(_, outcomes) => {
                writeln!(f, "any of:")?;
                for outcome in outcomes {
                    match outcome {
                        TestOutcome::Failed(failure) => {
                            writeln!(f, "  {}", failure)?;
                        }
                        _ => {
                            writeln!(f, "  Unexpected test outcome {:?}", outcome)?;
                        }
                    }
                }
                Ok(())
            }
            Failure::Not(_a, _outcome) => {
                writeln!(f, "not:")?;
                // writeln!(f, "  {}", outcome)?;
                Ok(())
            }
            Failure::Eq(a, value) => {
                writeln!(f, "eq:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
            Failure::DeepEq(a, value) => {
                writeln!(f, "deep-eq:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
            Failure::True(_a, value) => {
                writeln!(f, "true:")?;
                writeln!(f, "  expected: true")?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
            Failure::False(_a, value) => {
                writeln!(f, "false:")?;
                writeln!(f, "  expected: false")?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
            Failure::Count(a, failure) => {
                writeln!(f, "count:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", failure)?;
                Ok(())
            }
            Failure::StringValue(a, failure) => {
                writeln!(f, "string-value:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", failure)?;
                Ok(())
            }
            Failure::Xml(a, failure) => {
                writeln!(f, "xml:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", failure)?;
                Ok(())
            }
            Failure::Assert(_a, failure) => {
                writeln!(f, "assert:")?;
                writeln!(f, "  actual: {:?}", failure)?;
                Ok(())
            }
            Failure::Permutation(a, failure) => {
                writeln!(f, "permutation:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", failure)?;
                Ok(())
            }
            Failure::Empty(_a, value) => {
                writeln!(f, "empty:")?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
            Failure::Type(_a, value) => {
                writeln!(f, "type:")?;
                writeln!(f, "  expected type: {:?}", _a.0)?;
                writeln!(f, "  value of wrong type: {:?}", value)?;
                Ok(())
            }
            Failure::Error(a, value) => {
                writeln!(f, "error:")?;
                writeln!(f, "  expected: {:?}", a.0)?;
                writeln!(f, "  actual: {:?}", value)?;
                Ok(())
            }
        }
    }
}

fn run_xpath(expr: &XPathExpr) -> error::Result<Sequence> {
    let queries = Queries::default();
    let q = queries.sequence(expr)?;

    let mut documents = Documents::default();

    // we don't need any particular context to execute this query
    q.execute_build_context(&mut documents, |_build| {})
}

fn run_xpath_with_result(
    expr: &XPathExpr,
    sequence: &Sequence,
    documents: &mut Documents,
) -> error::Result<Sequence> {
    let mut builder = context::StaticContextBuilder::default();
    let name = Name::name("result");
    builder.variable_names([name.clone()]);
    let static_context = builder.build();

    let queries = Queries::default();
    let q = queries.sequence_with_context(expr, static_context)?;

    let variables = AHashMap::from([(name, sequence.clone())]);

    q.execute_build_context(documents, |build| {
        build.variables(variables);
    })
}

pub(crate) fn serialize(xot: &Xot, sequence: &Sequence) -> crate::error::Result<String> {
    let mut xmls = Vec::with_capacity(sequence.len());
    for item in sequence.iter() {
        if let Ok(node) = item.to_node() {
            let xml_value = xot.to_string(node);
            if let Ok(xml_value) = xml_value {
                xmls.push(xml_value);
            } else {
                return Err(crate::error::Error::CannotRepresentAsXml);
            }
        } else {
            return Err(crate::error::Error::CannotRepresentAsXml);
        }
    }
    Ok(format!("<sequence>{}</sequence>", xmls.join("")))
}

#[cfg(test)]
mod tests {
    use std::{path::PathBuf, vec};

    use super::*;

    use crate::ns::XPATH_TEST_NS;

    #[test]
    fn test_test_case_result() {
        let xml = format!(
            r#"<doc xmlns="{}"><result><assert-eq>0</assert-eq></result></doc>"#,
            XPATH_TEST_NS
        );
        let context = LoadContext {
            path: PathBuf::new(),
            catalog_ns: XPATH_TEST_NS,
        };
        let test_case_result = TestCaseResult::load_from_xml_with_context(&xml, &context).unwrap();
        assert_eq!(
            test_case_result,
            TestCaseResult::AssertEq(AssertEq::new("0".to_string()))
        );
    }

    #[test]
    fn test_test_case_result2() {
        let xml = format!(
            r#"
<doc xmlns="{}">
  <result>
    <any-of>
      <assert>$result/x = ('http://www.example.com', 'http://www.example.com/')</assert>
      <assert>$result/x = 'http://www.example.com/base'</assert>
   </any-of>
  </result>
</doc>"#,
            XPATH_TEST_NS
        );
        let context = LoadContext {
            path: PathBuf::new(),
            catalog_ns: XPATH_TEST_NS,
        };
        let test_case_result = TestCaseResult::load_from_xml_with_context(&xml, &context).unwrap();
        assert_eq!(
            test_case_result,
            TestCaseResult::AnyOf(AssertAnyOf::new(vec![
                TestCaseResult::Assert(Assert::new(
                    "$result/x = ('http://www.example.com', 'http://www.example.com/')".to_string()
                )),
                TestCaseResult::Assert(Assert::new(
                    "$result/x = 'http://www.example.com/base'".to_string()
                )),
            ]))
        );
    }
}
