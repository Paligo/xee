use std::fmt::Write;

use xee_interpreter::{
    context::{StaticContext, TypeTableRef},
    error,
    sequence::Sequence,
    xml::Documents,
};
use xee_name::{Namespaces, FN_NAMESPACE};
use xee_schema_type::Xs;
use xee_xslt_compiler::{evaluate, parse};
use xot::Xot;

fn xml(xot: &Xot, sequence: Sequence) -> String {
    let mut f = String::new();

    for item in sequence.iter() {
        f.write_str(&xot.to_string(item.to_node().unwrap()).unwrap())
            .unwrap();
    }
    f
}

fn evaluate_with_type_table(
    xot: &mut Xot,
    xml: &str,
    xslt: &str,
    type_table: TypeTableRef,
) -> error::SpannedResult<Sequence> {
    let namespaces = Namespaces::new(
        Namespaces::default_namespaces(),
        "".to_string(),
        FN_NAMESPACE.to_string(),
    );
    let static_context = StaticContext::from_namespaces(namespaces);
    let root = xot.parse(xml).unwrap();
    let program = parse(static_context, xslt).unwrap();
    let mut documents = Documents::new();
    let handle = documents.add_root(None, root).unwrap();
    let root = documents.get_node_by_handle(handle).unwrap();
    let mut dynamic_context_builder = program.dynamic_context_builder();
    dynamic_context_builder.context_node(root);
    dynamic_context_builder.documents(documents);
    dynamic_context_builder.type_table(type_table);
    let context = dynamic_context_builder.build();
    let runnable = program.runnable(&context);
    runnable.many(xot)
}

fn child_element_named(xot: &Xot, parent: xot::Node, name: &str) -> xot::Node {
    let target = xot.name(name).unwrap();
    let mut child = xot.first_child(parent);
    while let Some(node) = child {
        if xot.is_element(node) && xot.node_name(node) == Some(target) {
            return node;
        }
        child = xot.next_sibling(node);
    }
    panic!("child element not found: {}", name);
}

#[test]
fn test_transform() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/"><a/></xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<a/>");
}

#[test]
fn test_transform_nested() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/"><a><b/><b/></a></xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<a><b/><b/></a>");
}

#[test]
fn test_transform_text_node() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/"><a>foo</a></xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<a>foo</a>");
}

#[test]
fn test_transform_nested_apply_templates() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><bar/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:apply-templates select="doc/*" /></o>
  </xsl:template>
  <xsl:template match="foo">
    <f/>
  </xsl:template>
  <xsl:template match="bar">
    <b/>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><f/><b/></o>");
}

#[test]
fn test_transform_value_of_select() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:value-of select="1 to 4" /></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>1 2 3 4</o>");
}

#[test]
fn test_transform_value_of_select_separator() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:value-of select="1 to 4" separator="|" /></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>1|2|3|4</o>");
}

#[test]
fn test_value_of_with_sequence_constructor() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:value-of>Hello</xsl:value-of></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>Hello</o>");
}

#[test]
fn test_transform_local_variable() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <xsl:variable name="foo" select="'FOO'"/>
    <o><xsl:value-of select="$foo"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o>FOO</o>");
}

#[test]
fn test_transform_local_variable_shadow() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <xsl:variable name="foo" select="'FOO'"/>
    <xsl:variable name="foo" select="'BAR'"/>
    <o><xsl:value-of select="$foo"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o>BAR</o>");
}

#[test]
fn test_transform_local_variable_from_sequence_constructor() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <xsl:variable name="foo"><b>B</b></xsl:variable>
    <o><xsl:value-of select="$foo"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o>B</o>");
}

#[test]

fn test_transform_document_order_dynamically_with_variable() {
    let mut xot = Xot::new();

    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <xsl:variable name="foo"><a><b/><b/></a></xsl:variable>
    <o><xsl:for-each select="$foo//node()"><v/></xsl:for-each></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    // TODO: I am not sure whether this is correct; I'd expect $foo//node() to
    // also get the root nodes of the sequence, but it doesn't seem to do so
    // but the main point of this test is to check that the nodes found
    // do have document order (created dynamically) and they do
    assert_eq!(xml(&xot, output), "<o><v/><v/></o>");
}

#[test]
fn test_transform_if_true() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:if test="1"><foo/></xsl:if></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><foo/></o>");
}

#[test]
fn test_transform_if_false() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:if test="0"><foo/></xsl:if></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o/>");
}

#[test]
fn test_transform_choose_when() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:choose>
      <xsl:when test="1"><foo/></xsl:when>
      <xsl:otherwise><bar/></xsl:otherwise>
    </xsl:choose></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><foo/></o>");
}

#[test]
fn test_transform_choose_otherwise() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:choose>
      <xsl:when test="0"><foo/></xsl:when>
      <xsl:otherwise><bar/></xsl:otherwise>
    </xsl:choose></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><bar/></o>");
}

#[test]
fn test_transform_choose_when_false_no_otherwise() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:choose>
      <xsl:when test="0"><foo/></xsl:when>
    </xsl:choose></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o/>");
}

#[test]
fn test_transform_multiple_when() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:choose>
      <xsl:when test="0"><foo/></xsl:when>
      <xsl:when test="1"><bar/></xsl:when>
      <xsl:otherwise><baz/></xsl:otherwise>
    </xsl:choose></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><bar/></o>");
}

#[test]
fn test_transform_multiple_when_with_otherwise() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:choose>
      <xsl:when test="0"><foo/></xsl:when>
      <xsl:when test="0"><bar/></xsl:when>
      <xsl:otherwise><baz/></xsl:otherwise>
    </xsl:choose></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><baz/></o>");
}

#[test]
fn test_basic_for_each() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo/><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:for-each select="doc/foo"><bar/></xsl:for-each></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><bar/><bar/><bar/></o>");
}

#[test]
fn test_for_each_context() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo>0</foo><foo>1</foo><foo>2</foo></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:for-each select="doc/foo">
      <bar><xsl:value-of select="string()"/></bar>
    </xsl:for-each></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(
        xml(&xot, output),
        "<o><bar>0</bar><bar>1</bar><bar>2</bar></o>"
    );
}

#[test]
fn test_copy_empty_sequence() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:copy select="()"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o/>");
}

#[test]
fn test_copy_not_one_item_fails() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" >
  <xsl:template match="/">
    <o><xsl:copy select="(1, 2)"/></o>
  </xsl:template>
</xsl:transform>"#,
    );
    // TODO: check the right error value
    assert!(matches!(output, error::SpannedResult::Err(_)));
}

#[test]
fn test_copy_atom() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
                 <xsl:template match="/">
                   <xsl:variable name="foo"><xsl:copy select="1"/></xsl:variable>
                   <o><xsl:value-of select="string($foo)"/></o>
                 </xsl:template>
              </xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>1</o>");
}

#[test]
fn test_copy_function() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
                 <xsl:template match="/">
                   <xsl:variable name="foo"><xsl:copy select="function() { 1 }"/></xsl:variable>
                   <o><xsl:value-of select="string($foo)"/></o>
                 </xsl:template>
              </xsl:transform>"#,
    );
    // this is an error as we try to atomize a function
    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::FOTY0014,
            span: _
        })
    ));
}

#[test]
fn test_copy_text() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc>content</doc>",
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
                 <xsl:template match="/">
                   <xsl:variable name="foo"><xsl:copy select="doc/child::node()" /></xsl:variable>
                   <o><xsl:value-of select="string($foo)"/></o>
                 </xsl:template>
              </xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>content</o>");
}

#[test]
fn test_copy_element() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><p>Content</p></doc>",
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
                 <xsl:template match="/">
                   <o><xsl:copy select="doc/*" /></o>
                 </xsl:template>
              </xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><p/></o>");
}

#[test]
fn test_copy_element_with_sequence_constructor() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><p>Content</p></doc>",
        r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
                 <xsl:template match="/">
                   <o><xsl:copy select="doc/*">Constructed</xsl:copy></o>
                 </xsl:template>
              </xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><p>Constructed</p></o>");
}

#[test]
fn test_copy_of_atom() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:variable name="foo"><xsl:copy-of select="'foo'" /></xsl:variable>
      <xsl:value-of select="string($foo)"/>
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>foo</o>");
}

#[test]
fn test_copy_of_node() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo>FOO</foo></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:copy-of select="/doc/foo" />
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><foo>FOO</foo></o>");
}

#[test]
fn test_sequence() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:value-of><xsl:sequence select="1 to 4" /></xsl:value-of></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>1 2 3 4</o>");
}

#[test]
fn test_complex_content_single_string() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:sequence select="'foo'" />
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>foo</o>");
}

#[test]
fn test_complex_content_multiple_strings() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:sequence select="('foo', 'bar')" />
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o>foo bar</o>");
}

#[test]
fn test_complex_content_xml_and_atomic() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:sequence select="('foo', 'bar')" />
      <hello>Hello</hello>
      <xsl:sequence select="('baz', 'qux')" />
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(
        xml(&xot, output),
        "<o>foo bar<hello>Hello</hello>baz qux</o>"
    );
}

#[test]
fn test_function_item_in_complex_content() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:sequence select="function() { 1 }" /></o>
  </xsl:template>
</xsl:transform>"#,
    );

    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::XTDE0450,
            span: _
        })
    ));
}

#[test]
fn test_source_nodes_complex_content() {
    let mut xot = Xot::new();
    // try this twice, so that we verify no mutation of source takes place and
    // source code nodes are properly copied
    let output = evaluate(
        &mut xot,
        "<doc><hello>Hello</hello></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:sequence select="/doc/hello" />
      <xsl:sequence select="/doc/hello" />
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<o><hello>Hello</hello><hello>Hello</hello></o>"
    );
}

#[test]
fn test_transform_predicate() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo>1</foo><foo>2</foo></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:apply-templates select="doc/*" /></o>
  </xsl:template>
  <xsl:template match="foo[2]">
    <found><xsl:value-of select="string()" /></found>
  </xsl:template>
  <xsl:template match="text()" />
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><found>2</found></o>");
}

#[test]
fn test_transform_predicate_with_attribute() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><foo>1</foo><foo bar="BAR">2</foo></doc>"#,
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:apply-templates select="doc/*" /></o>
  </xsl:template>
  <xsl:template match="foo[@bar]">
    <found><xsl:value-of select="string()" /></found>
  </xsl:template>
  <xsl:template match="text()" />
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><found>2</found></o>");
}

#[test]
fn test_text_value_template() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc>VALUE</doc>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>Value: {string()}</o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o>Value: VALUE</o>");
}

#[test]
fn test_literal_attribute() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><foo bar="baz"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><foo bar="baz"/></o>"#);
}

#[test]
fn test_literal_attributes() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><foo bar="BAR" qux="QUX"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><foo bar="BAR" qux="QUX"/></o>"#);
}

#[test]
fn test_literal_attribute_with_value_template() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc>value</doc>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><foo bar="found: {doc/string()}"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><foo bar="found: value"/></o>"#);
}

#[test]
fn test_xsl_element() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:element name="foo">content</xsl:element></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><foo>content</foo></o>"#);
}

// cannot test this yet as we need namespace prefix handling

// #[test]
// fn test_xsl_element_with_namespace() {
//     let mut xot = Xot::new();
//     let output = evaluate(
//         &mut xot,
//         r#"<doc/>"#,
//         r#"
//   <xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
//     <xsl:template match="/">
//       <o><xsl:element name="foo" namespace="http://example.com">content</xsl:element></o>
//     </xsl:template>
//   </xsl:transform>"#,
//     )
//     .unwrap();

//     assert_eq!(xml(&xot, output), r#"<o><foo>content</foo></o>"#);
// }

#[test]
fn test_xsl_text() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:text>content</xsl:text></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o>content</o>"#);
}

#[test]
fn test_xsl_text_empty() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:text/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o/>"#);
}

#[test]
fn test_xsl_text_value_template() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:text>Content: {"foo"}</xsl:text></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o>Content: foo</o>"#);
}

#[test]
fn test_xsl_attribute_with_select() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:attribute name="foo" select="'FOO'"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o foo="FOO"/>"#);
}

#[test]
fn test_xsl_attribute_name_value_template() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:attribute name="{'foo'}" select="'FOO'"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o foo="FOO"/>"#);
}

#[test]
fn test_xsl_attribute_with_content() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:attribute name="foo">FOO</xsl:attribute></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o foo="FOO"/>"#);
}

#[test]
fn test_namespace() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:namespace name="foo" select="'http://example.com'"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o xmlns:foo="http://example.com"/>"#);
}

#[test]
fn test_comment() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:comment>comment</xsl:comment></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><!--comment--></o>"#);
}

#[test]
fn test_pi_with_text() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:processing-instruction name="foo">bar</xsl:processing-instruction></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><?foo bar?></o>"#);
}

#[test]
fn test_pi_without_text() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:processing-instruction name="foo"/></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o><?foo?></o>"#);
}

#[test]
fn test_priority() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><foo/></doc>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="foo" priority="1">
    <o>foo</o>
  </xsl:template>
  <xsl:template match="foo" priority="2">
    <o>foo2</o>
  </xsl:template>
  <xsl:template match="/">
    <xsl:apply-templates select="doc/foo"/>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o>foo2</o>"#);
}

#[test]
fn test_priority_declaration_order_last_one_wins() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><foo/></doc>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="foo" priority="1">
    <o>foo</o>
  </xsl:template>
  <xsl:template match="foo" priority="1">
    <o>foo2</o>
  </xsl:template>
  <xsl:template match="/">
    <xsl:apply-templates select="doc/foo"/>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<o>foo2</o>"#);
}

#[test]
fn test_priority_more_specific_default_priority_wins() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><foo/></doc>"#,
        r#"
<xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="foo">
    <o>foo</o>
  </xsl:template>
  <xsl:template match="*">
    <o>foo2</o>
  </xsl:template>
  <xsl:template match="/">
    <xsl:apply-templates select="doc/foo"/>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    // foo matches as it's more specific
    assert_eq!(xml(&xot, output), r#"<o>foo</o>"#);
}

// TODO: this test has become unreliable afte rI added tdefault
// template rules. It passes sometimes and doesn't pass other times
// and I don't know why yet. This may be related to unreliable tests
// in the XSLT 3.0 test suite.
// #[test]
// fn test_mode_undeclared() {
//     let mut xot = Xot::new();
//     let output = evaluate(
//         &mut xot,
//         r#"<doc><foo/></doc>"#,
//         r#"
// <xsl:transform expand-text="true" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
//   <xsl:template match="/">
//     <o><xsl:apply-templates select="doc/foo" mode="bar"/></o>
//   </xsl:template>
//   <xsl:template match="foo" mode="bar">
//     <bar/>
//   </xsl:template>
// </xsl:transform>"#,
//     )
//     .unwrap();

//     assert_eq!(xml(&xot, output), r#"<o><bar/></o>"#);
// }

#[test]
fn test_generate_text_node() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc>test</doc>"#,
        r#"<?xml version="1.0"?>
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">

<xsl:template match="/doc">
  <out>
    <xsl:value-of select="./text()"/>
  </out>
</xsl:template>

<xsl:template match="text()">
  <xsl:value-of select="."/>
</xsl:template>

</xsl:stylesheet>
    "#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), r#"<out>test</out>"#);
}

#[test]
fn test_basic_iterate() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo/><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:iterate select="doc/foo"><bar/></xsl:iterate></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><bar/><bar/><bar/></o>");
}

#[test]
fn test_basic_iterate_on_complete() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo/><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:iterate select="doc/foo"><xsl:on-completion><bar/></xsl:on-completion><baz/></xsl:iterate></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><baz/><baz/><baz/><bar/></o>");
}

#[test]
fn test_basic_iterate_on_complete_break() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo/><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:iterate select="doc/foo"><xsl:on-completion><bar/></xsl:on-completion><xsl:break/></xsl:iterate></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o/>");
}

#[test]
fn test_basic_iterate_if_break() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo><x/></foo><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:iterate select="doc/foo"><baz/><xsl:if test="x"><xsl:break select="'exit at ' || position() || ' of ' || last()"/></xsl:if></xsl:iterate></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<o><baz/><baz/>exit at 2 of 3</o>");
}
#[test]
fn test_basic_iterate_params() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><foo/><foo/><foo/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:iterate select="doc/foo"><xsl:param name="a" select="1"/><baz><xsl:value-of select="$a"/></baz><xsl:next-iteration><xsl:with-param name="a" select="$a * 2"/></xsl:next-iteration></xsl:iterate></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(
        xml(&xot, output),
        "<o><baz>1</baz><baz>2</baz><baz>4</baz></o>"
    );
}

#[test]
fn test_try_catch_rollback_output() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <xsl:try rollback-output="yes">
      <before/>
      <xsl:sequence select="error()"/>
      <xsl:catch errors="*">
        <caught/>
      </xsl:catch>
    </xsl:try>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();
    assert_eq!(xml(&xot, output), "<caught/>");
}

#[test]
fn test_type_table_for_typed_constructors() {
    let mut xot = Xot::new();
    let type_table = TypeTableRef::new();
    let output = evaluate_with_type_table(
        &mut xot,
        "<doc><src-copy/><src-copy-of/></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
               xmlns:xs="http://www.w3.org/2001/XMLSchema"
               version="3">
  <xsl:template match="/">
    <root>
      <xsl:element name="typed-element" type="xs:string"/>
      <xsl:element name="attr-holder">
        <xsl:attribute name="typed-attr" type="xs:integer" select="'1'"/>
      </xsl:element>
      <xsl:copy select="/doc/src-copy" type="xs:decimal"/>
      <xsl:copy-of select="/doc/src-copy-of" type="xs:boolean"/>
    </root>
    <xsl:document type="xs:anyType">
      <doc-child/>
    </xsl:document>
  </xsl:template>
</xsl:transform>"#,
        type_table.clone(),
    )
    .unwrap();

    let root_name = xot.name("root").unwrap();
    let mut root_node = None;
    let mut document_node = None;
    for item in output.iter() {
        let Ok(node) = item.to_node() else {
            continue;
        };
        if xot.is_element(node) && xot.node_name(node) == Some(root_name) {
            root_node = Some(node);
        } else if xot.is_document(node) {
            document_node = Some(node);
        }
    }

    let root_node = root_node.expect("root element missing from output");
    let document_node = document_node.expect("document node missing from output");
    let typed_element = child_element_named(&xot, root_node, "typed-element");
    let attr_holder = child_element_named(&xot, root_node, "attr-holder");
    let copy_node = child_element_named(&xot, root_node, "src-copy");
    let copy_of_node = child_element_named(&xot, root_node, "src-copy-of");
    let typed_attr = xot
        .attributes(attr_holder)
        .get_node(xot.name("typed-attr").unwrap())
        .unwrap();

    let type_table = type_table.borrow();
    assert_eq!(type_table.get(typed_element), Some(Xs::String));
    assert_eq!(type_table.get(typed_attr), Some(Xs::Integer));
    assert_eq!(type_table.get(copy_node), Some(Xs::Decimal));
    assert_eq!(type_table.get(copy_of_node), Some(Xs::Boolean));
    assert_eq!(type_table.get(document_node), Some(Xs::AnyType));
}
