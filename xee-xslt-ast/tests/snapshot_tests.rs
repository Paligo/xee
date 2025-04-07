use xee_xslt_ast::{parse_sequence_constructor_item, parse_transform};

use insta::assert_ron_snapshot;

#[test]
fn test_if() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()">Hello</xsl:if>"#
    ));
}

#[test]
fn test_variable() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" select="true()">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_missing_required() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_broken_xpath() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" select="let $x := 1">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_sequence_type() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" name="foo" as="xs:string" select="true()">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_boolean_default_no_with_explicit_yes() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" static="yes" select="true()">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_variable_visibility() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" visibility="public">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_variable_visibility_abstract_with_select_is_error() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" visibility="abstract" select="true()">Hello</xsl:variable>"#
    ));
}

#[test]
fn test_copy() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" copy-namespaces="no" inherit-namespaces="no" validation="strict">Hello</xsl:copy>"#
    ));
}

#[test]
fn test_eqnames() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" use-attribute-sets="foo bar baz">Hello</xsl:copy>"#
    ));
}

#[test]
fn test_eqnames_error() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" use-attribute-sets="foo br!ken bar">Hello</xsl:copy>"#
    ));
}

#[test]
fn test_nested_if() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><xsl:if test="true()">Hello</xsl:if></xsl:if>"#
    ));
}

#[test]
fn test_if_with_standard_attribute() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes">Hello</xsl:if>"#
    ));
}

#[test]
fn test_literal_result_element() {
    assert_ron_snapshot!(parse_sequence_constructor_item(r#"<foo/>"#));
}

#[test]
fn test_literal_result_element_with_standard_attribute() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<foo xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:expand-text="yes"/>"#
    ));
}

#[test]
fn test_no_fn_namespace_by_default() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="fn:true()">Hello</xsl:if>"#
    ));
}

#[test]
fn test_attribute_value_template_just_string() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:assert xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" error-code="foo">Hello</xsl:assert>"#
    ));
}

#[test]
fn test_analyze_string() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
    ));
}

#[test]
fn test_analyze_string_absent_matching_substring() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
    ));
}

#[test]
fn test_analyze_string_absent_non_matching_substring() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
    ));
}

#[test]
fn test_analyze_string_absent_fallbacks() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring></xsl:analyze-string>"#
    ));
}

#[test]
fn test_analyze_string_absent_all() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"></xsl:analyze-string>"#
    ));
}

#[test]
fn test_analyze_string_matching_non_matching_wrong_order() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:matching-substring>Matching</xsl:matching-substring></xsl:analyze-string>"#
    ));
}

#[test]
fn test_accumulator() {
    assert_ron_snapshot!(parse_transform(
        r#"<xsl:transform version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:accumulator name="foo" initial-value="1"><xsl:accumulator-rule match="foo"/></xsl:accumulator></xsl:transform>"#
    ));
}

#[test]
fn test_should_be_empty_not_empty() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:copy-of xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()">Illegal content</xsl:copy-of>"#
    ))
}

#[test]
fn test_apply_templates_with_mixed_content() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:apply-templates xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:sort>Sort</xsl:sort><xsl:with-param name="a">With param</xsl:with-param></xsl:apply-templates>"#
    ))
}

#[test]
fn test_for_each() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:for-each xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()"><xsl:sort>Sort 1</xsl:sort><xsl:sort>Sort 2</xsl:sort>Sequence constructor</xsl:for-each>"#
    ))
}

#[test]
fn test_fork1() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:fork xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:sequence>Sequence 1</xsl:sequence><xsl:sequence>Sequence 2</xsl:sequence></xsl:fork>"#
    ))
}

#[test]
fn test_fork2() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:fork xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:for-each-group select="true()">Content</xsl:for-each-group></xsl:fork>"#
    ))
}

#[test]
fn test_unsupported_attribute() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" unsupported="Unsupported">Hello</xsl:if>"#
    ));
}

#[test]
fn test_no_expand_text_should_not_expand_text() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()">Hello {world}!</xsl:if>"#
    ));
}

#[test]
fn test_expand_text_should_expand_text() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes">Hello {world}!</xsl:if>"#
    ));
}

#[test]
#[ignore]
fn test_xsl_expand_text_should_expand_text() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p xsl:expand-text="yes">Hello {world}!</p></xsl:if>"#
    ));
}

#[test]
fn test_expand_text_disabled_should_not_expand_text() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes"><xsl:if expand-text="no" test="true()">Hello {world}!</xsl:if></xsl:if>"#
    ));
}

#[test]
fn test_nested_literal_elements() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p><another/></p></xsl:if>"#
    ));
}

#[test]
fn test_sequence_constructor_nested_in_literal_element() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p><xsl:if test="true()">foo</xsl:if></p></xsl:if>"#
    ));
}

#[test]
fn test_attributes_on_literal_element() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<p xmlns:xsl="http://www.w3.org/1999/XSL/Transform" foo="FOO"/>"#
    ));
}

#[test]
fn test_template_unnamed_mode() {
    assert_ron_snapshot!(parse_transform(
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*">a</xsl:template></xsl:transform>"#
    ));
}

#[test]
fn test_template_named_mode() {
    assert_ron_snapshot!(parse_transform(
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*" mode="foo">a</xsl:template></xsl:transform>"#
    ));
}

#[test]
fn test_template_default_mode_explicit_fallback_to_default() {
    // to include #" in the raw string, we need to mark it with double #
    // https://rahul-thakoor.github.io/rust-raw-string-literals/
    assert_ron_snapshot!(parse_transform(
        r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" default-mode="foo"><xsl:template match="*" mode="#default">a</xsl:template></xsl:transform>"##
    ));
}

#[test]
fn test_template_default_mode_implicit_fallback_to_default() {
    // to include #" in the raw string, we need to mark it with double #
    // https://rahul-thakoor.github.io/rust-raw-string-literals/
    assert_ron_snapshot!(parse_transform(
        r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" default-mode="foo"><xsl:template match="*">a</xsl:template></xsl:transform>"##
    ));
}

#[test]
fn test_template_default_mode_explicit_fallback_to_unnamed() {
    // to include #" in the raw string, we need to mark it with double #
    // https://rahul-thakoor.github.io/rust-raw-string-literals/
    assert_ron_snapshot!(parse_transform(
        r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*" mode="#default">a</xsl:template></xsl:transform>"##
    ));
}

#[test]
fn test_apply_templates_implicit_default_mode_is_unnamed() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:apply-templates xmlns:xsl="http://www.w3.org/1999/XSL/Transform" />"#
    ));
}

#[test]
fn test_apply_templates_explicit_default_mode() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:apply-templates default-mode="foo" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="*"/>"#
    ));
}

#[test]
fn test_apply_templates_explicit_mode() {
    assert_ron_snapshot!(parse_sequence_constructor_item(
        r#"<xsl:apply-templates mode="foo" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="*"/>"#
    ));
}
