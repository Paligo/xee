use std::fmt::Write;

use xee_interpreter::{context::StaticContext, error, sequence::Sequence};
use xee_name::{Namespaces, FN_NAMESPACE};
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
fn test_duplicate_local_template_params_are_rejected() {
    let namespaces = Namespaces::new(
        Namespaces::default_namespaces(),
        "".to_string(),
        FN_NAMESPACE.to_string(),
    );
    let static_context = StaticContext::from_namespaces(namespaces);
    let output = parse(
        static_context,
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <xsl:apply-templates select="doc">
      <xsl:with-param name="mod" select="3"/>
    </xsl:apply-templates>
  </xsl:template>

  <xsl:template match="doc">
    <xsl:param name="mod" select="1"/>
    <xsl:param name="mod" select="2"/>
    <out result="{$mod}"/>
  </xsl:template>
</xsl:transform>"#,
    );

    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::XTSE0580,
            span: _
        })
    ));
}

#[test]
fn test_pattern_predicate_position_ignores_whitespace_text_nodes() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<servlet-mapping>
   <servlet-name>MyServlet</servlet-name>
   <url-pattern>/servlet/MyServlet/*</url-pattern>
</servlet-mapping>"#,
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0" expand-text="yes">
  <xsl:template match="/">
    <xsl:apply-templates select="servlet-mapping/url-pattern"/>
  </xsl:template>
  <xsl:template match="url-pattern[position()=last()]">
    <out>{.}</out>
  </xsl:template>
  <xsl:template match="url-pattern"><wrong/></xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>/servlet/MyServlet/*</out>");
}

#[test]
fn test_message_is_ignored_in_result_sequence() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o>
      <xsl:message>debug</xsl:message>
      <a/>
    </o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<o><a/></o>");
}

  #[test]
  fn test_local_variable_as_type_is_enforced() {
    let mut xot = Xot::new();
    let output = evaluate(
      &mut xot,
      "<doc/>",
      r#"
  <xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"
    xmlns:xs="http://www.w3.org/2001/XMLSchema">
    <xsl:template match="/">
    <xsl:variable name="v" as="xs:integer" select="true()"/>
    <out value="{$v}"/>
    </xsl:template>
  </xsl:transform>"#,
    );

    assert!(matches!(
      output,
      error::SpannedResult::Err(error::SpannedError {
        error: error::Error::XTTE0570,
        span: _
      })
    ));
  }

#[test]
fn test_global_variable_sequence_constructor_creates_temporary_tree() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"
    xmlns:xs="http://www.w3.org/2001/XMLSchema"
    exclude-result-prefixes="xs">
  <xsl:variable name="data">
    <a xmlns:p="http://p.com/ns"/>
  </xsl:variable>

  <xsl:param name="prefix" select="'p'"/>

  <xsl:template match="/">
    <out>
      <xsl:variable name="uri" select="namespace-uri-for-prefix($prefix, $data/*)" as="xs:string"/>
      <xsl:value-of select="$uri"/>
    </out>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>http://p.com/ns</out>");
}

#[test]
fn test_global_variable_is_out_of_scope_within_its_own_declaration() {
  let namespaces = Namespaces::new(
    Namespaces::default_namespaces(),
    "".to_string(),
    FN_NAMESPACE.to_string(),
  );
  let static_context = StaticContext::from_namespaces(namespaces);
  let output = parse(
    static_context,
    r#"
<xsl:stylesheet version="3.0"
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:xs="http://www.w3.org/2001/XMLSchema">
  <xsl:template match="/">
  <out att="{$gcd(4,2)}"/>
  </xsl:template>

  <xsl:variable name="gcd" as="function(*)"
    select="function($x as xs:integer, $y as xs:integer) {
    if ($y eq 0)
    then abs($x)
    else $gcd($y,$x mod $y)
    }"/>
</xsl:stylesheet>"#,
  );

  assert!(matches!(
    output,
    error::SpannedResult::Err(error::SpannedError {
      error: error::Error::XPST0008,
      span: _
    })
  ));
}

#[test]
fn test_top_level_non_xsl_elements_do_not_break_parse() {
  let namespaces = Namespaces::new(
    Namespaces::default_namespaces(),
    "".to_string(),
    FN_NAMESPACE.to_string(),
  );
  let static_context = StaticContext::from_namespaces(namespaces);
  let output = parse(
    static_context,
    r#"
<xsl:stylesheet version="2.0"
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:test="my:test">
  <?spec xslt#with-param?>
  <test:test/>
  <xsl:template match="/">
  <out/>
  </xsl:template>
</xsl:stylesheet>"#,
  );

  assert!(output.is_ok());
}

#[test]
fn test_builtin_template_rule_passes_params_in_xslt_2_mode() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><group><para/></group></doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="doc">
    <out>
      <xsl:apply-templates>
        <xsl:with-param name="x" select="42"/>
      </xsl:apply-templates>
    </out>
  </xsl:template>

  <xsl:template match="para">
    <xsl:param name="x" select="0"/>
    <x><xsl:value-of select="$x"/></x>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out><x>42</x></out>");
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
