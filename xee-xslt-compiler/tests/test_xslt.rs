use std::fmt::Write;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use xee_interpreter::{
    context::{StaticContext, StaticContextBuilder},
    error,
    sequence::Sequence,
    xml::Documents,
};
use xee_name::{Namespaces, FN_NAMESPACE};
use xee_xslt_compiler::{evaluate, parse, parse_with_base_dir};
use xot::Xot;

fn xml(xot: &Xot, sequence: Sequence) -> String {
    let mut f = String::new();

    for item in sequence.iter() {
        f.write_str(&xot.to_string(item.to_node().unwrap()).unwrap())
            .unwrap();
    }
    f
}

fn evaluate_with_stylesheet_base(
    xot: &mut Xot,
    xml: &str,
    xslt: &str,
    stylesheet_path: &std::path::Path,
) -> error::SpannedResult<Sequence> {
    let stylesheet_uri = format!("file://{}", stylesheet_path.display()).replace(' ', "%20");
    let mut static_context_builder = StaticContextBuilder::default();
    static_context_builder.static_base_uri(Some(stylesheet_uri.try_into().unwrap()));
    let static_context = static_context_builder.build();
    let program = parse_with_base_dir(
        static_context,
        xslt,
        stylesheet_path.parent().map(|parent| parent.to_path_buf()),
    )
    .unwrap();

    let root = xot.parse(xml).unwrap();
    let mut documents = Documents::new();
    let handle = documents.add_root(None, root).unwrap();
    let root = documents.get_node_by_handle(handle).unwrap();
    let mut dynamic_context_builder = program.dynamic_context_builder();
    dynamic_context_builder.context_node(root);
    dynamic_context_builder.documents(documents);
    let context = dynamic_context_builder.build();
    let runnable = program.runnable(&context);
    runnable.many(xot)
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
fn test_match_node_pattern_does_not_capture_initial_document_node() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc>\n  <child1>This is the child number 1.</child1>\n</doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="doc">
    <out>
      <xsl:apply-templates select="node()" mode="mode1"/>
    </out>
  </xsl:template>

  <xsl:template match="node()" mode="mode1">
    <xsl:value-of select="."/>
  </xsl:template>

  <xsl:template match="node()">
    This test failed to execute properly.
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out>\n  This is the child number 1.\n</out>"
    );
}

#[test]
fn test_apply_templates_sort_with_param() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc>
  <a-set s="217" g="1st"><a>35</a><a>44</a><a>12</a><a>98</a><a>28</a></a-set>
  <a-set s="531" g="2nd"><a>62</a><a>440</a><a>29</a></a-set>
  <a-set s="172" g="3rd"><a>16</a><a>45</a><a>78</a><a>33</a></a-set>
</doc>"#,
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="doc">
    <out>
      <xsl:apply-templates select="a-set">
        <xsl:sort select="@s" data-type="number" order="ascending"/>
        <xsl:with-param name="total" select="sum(a-set/a)"/>
      </xsl:apply-templates>
    </out>
  </xsl:template>

  <xsl:template match="a-set">
    <xsl:param name="total"/>
    <list from="{@g}" proportion="{concat(sum(a), '/', $total)}">
      <xsl:for-each select="a">
        <xsl:value-of select="."/>
        <xsl:text>,</xsl:text>
      </xsl:for-each>
    </list>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out><list from=\"3rd\" proportion=\"172/920\">16,45,78,33,</list><list from=\"1st\" proportion=\"217/920\">35,44,12,98,28,</list><list from=\"2nd\" proportion=\"531/920\">62,440,29,</list></out>"
    );
}

#[test]
fn test_whitespace_padded_required_attribute_values() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="doc">
    <out>
      <xsl:call-template name="foo">
        <xsl:with-param name="par1" select="'required'"/>
        <xsl:with-param name="par2" select="'notRequired'"/>
      </xsl:call-template>
    </out>
  </xsl:template>

  <xsl:template name="foo">
    <xsl:param name="par1" required=" true "/>
    <xsl:param name="par2" required=" 0 "/>
    <xsl:if test="$par1 = 'required'">
      <xsl:text>Required parameter;</xsl:text>
    </xsl:if>
    <xsl:if test="$par2 = 'notRequired'">
      <xsl:text>Not required parameter</xsl:text>
    </xsl:if>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out>Required parameter;Not required parameter</out>"
    );
}

#[test]
fn test_next_match_with_param_falls_back_to_builtin_rule() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><data><inner><in><last>abc</last></in></inner></data></doc>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="doc">
    <out>
      <xsl:next-match>
        <xsl:with-param name="par1" select="'hola'"/>
      </xsl:next-match>
    </out>
  </xsl:template>

  <xsl:template match="data">
    <xsl:variable name="par1" select="'defaultValue'"/>
    <xsl:value-of select="$par1"/>
  </xsl:template>

  <xsl:template match="text()"/>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>defaultValue</out>");
}

#[test]
fn test_repeated_local_variable_reference_in_union_expression() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="/">
    <xsl:variable name="analysis">
      <result><span/></result>
      <unique><span/></unique>
    </xsl:variable>
    <out>
      <xsl:value-of select="count($analysis/result/span | $analysis/unique/span)"/>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>2</out>");
}

#[test]
fn test_global_variable_can_reference_later_global_param() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:variable name="tata" select="$toto"/>
  <xsl:param name="toto" select="'titi'"/>

  <xsl:template match="/">
    <xsl:param name="toto" select="'templ'"/>
    <out>
      <xsl:value-of select="$toto"/>
      <xsl:text>, </xsl:text>
      <xsl:value-of select="$tata"/>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>templ, titi</out>");
}

#[test]
fn test_call_template_unknown_param_is_ignored_in_xslt_1_mode() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="1.0">
  <xsl:param name="test" select="'global'"/>

  <xsl:template match="/">
    <out>
      <xsl:call-template name="temtest">
        <xsl:with-param name="test" select="'local'"/>
      </xsl:call-template>
    </out>
  </xsl:template>

  <xsl:template name="temtest">
    <xsl:choose>
      <xsl:when test="$test = 'global'">It is global!</xsl:when>
      <xsl:otherwise>Not global!!!</xsl:otherwise>
    </xsl:choose>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>It is global!</out>");
}

#[test]
fn test_union_match_without_explicit_priority_registers_multiple_rules() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><a>1</a><d>2</d></doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="doc">
    <out><xsl:apply-templates/></out>
  </xsl:template>

  <xsl:template match="a|d">
    <xsl:value-of select="name(.)"/>
    <xsl:text>=</xsl:text>
    <xsl:value-of select="."/>
    <xsl:text>;</xsl:text>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>a=1;d=2;</out>");
}

#[test]
fn test_builtin_text_template_rule_constructs_text_nodes() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><a>begin</a><b>middle</b><c>end</c></doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="doc">
    <out>
      <xsl:apply-templates select="a/text(), b/text(), c/text()"/>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>beginmiddleend</out>");
}

#[test]
fn test_xslt_user_defined_function_call_in_select() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
    xmlns:test="urn:test"
    xmlns:xs="http://www.w3.org/2001/XMLSchema"
    exclude-result-prefixes="test xs"
    version="3.0">
  <xsl:function name="test:double" as="xs:integer">
    <xsl:param name="value" as="xs:integer"/>
    <xsl:sequence select="$value * 2"/>
  </xsl:function>

  <xsl:template match="/">
    <out><xsl:value-of select="test:double(21)"/></out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>42</out>");
}

#[test]
fn test_for_each_sort_uses_xslt_sort_order() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="/">
    <out>
      <xsl:for-each select="('id-0', 'id-2', 'id-10')">
        <xsl:sort select="."/>
        <xsl:value-of select="."/>
        <xsl:text>|</xsl:text>
      </xsl:for-each>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>id-0|id-10|id-2|</out>");
}

#[test]
fn test_for_each_numeric_sort_preserves_order_for_nan_keys() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><t>First</t><t>p2</t><t>1.0.9</t><t>00k</t><t>1.u</t><t>1-m</t><t>0.5s</t><t>Last</t></doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="doc">
    <out>
      <xsl:for-each select="t">
        <xsl:sort data-type="number"/>
        <xsl:value-of select="."/>
        <xsl:text>|</xsl:text>
      </xsl:for-each>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out>First|p2|1.0.9|00k|1.u|1-m|0.5s|Last|</out>"
    );
}

#[test]
fn test_for_each_descending_numeric_sort_places_nan_last() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xs="http://www.w3.org/2001/XMLSchema"
               xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
               version="2.0">
  <xsl:template match="/">
    <out>
      <xsl:for-each select="(xs:float(12.5), xs:integer(1), xs:float('NaN'), xs:double('NaN'), xs:float(0.009), xs:double(-0.05), xs:string(-0.00))">
        <xsl:sort select="." data-type="number" order="descending"/>
        <xsl:value-of select="."/>
        <xsl:text>|</xsl:text>
      </xsl:for-each>
    </out>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out>12.5|1|0.009|0|-0.05|NaN|NaN|</out>"
    );
}

#[test]
fn test_mode_all_template_matches_initial_unnamed_mode() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="/" mode="#all">
    <out>ok</out>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>ok</out>");
}

#[test]
fn test_default_mode_attribute_overrides_nested_apply_templates_mode() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><a test="attribute"/></doc>"#,
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0" default-mode="a">
  <xsl:template match="/" mode="#all">
    <out xsl:default-mode="b">
      <xsl:apply-templates select="doc/a" default-mode="a"/>
    </out>
  </xsl:template>

  <xsl:template match="a" default-mode="a">
    <xsl:text>element-mode-a:</xsl:text>
    <xsl:apply-templates select="@test"/>
  </xsl:template>

  <xsl:template match="a" default-mode="b">
    <xsl:text>element-mode-b:</xsl:text>
    <xsl:apply-templates select="@test"/>
  </xsl:template>

  <xsl:template match="@*" mode="a">
    <xsl:value-of select="."/>
  </xsl:template>

  <xsl:template match="@*" mode="b">
    <xsl:text>attribute-mode-b</xsl:text>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>element-mode-a:attribute</out>");
}

#[test]
fn test_default_mode_attribute_on_apply_templates_selects_nested_mode() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc><a test="attribute"/></doc>"#,
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0" default-mode="a">
  <xsl:template match="/" mode="#all">
    <out xsl:default-mode="b">
      <xsl:apply-templates select="doc/a" default-mode="a"/>
    </out>
  </xsl:template>

  <xsl:template match="a" default-mode="a">
    <xsl:text>element-mode-a:</xsl:text>
    <xsl:apply-templates select="@test" default-mode="b"/>
  </xsl:template>

  <xsl:template match="a" default-mode="b">
    <xsl:text>element-mode-b:</xsl:text>
    <xsl:apply-templates select="@test"/>
  </xsl:template>

  <xsl:template match="@*" mode="a">
    <xsl:value-of select="."/>
  </xsl:template>

  <xsl:template match="@*" mode="b">
    <xsl:text>attribute-mode-b</xsl:text>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out>element-mode-a:attribute-mode-b</out>"
    );
}

#[test]
fn test_apply_templates_current_uses_current_mode_inside_template_rule() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><a><b/></a></doc>",
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="/">
    <out>
      <xsl:apply-templates select="doc/a" mode="m"/>
    </out>
  </xsl:template>

  <xsl:template match="a" mode="m">
    <xsl:apply-templates select="b" mode="#current"/>
  </xsl:template>

  <xsl:template match="b" mode="m">
    <m/>
  </xsl:template>

  <xsl:template match="b">
    <u/>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out><m/></out>");
}

#[test]
fn test_apply_templates_current_falls_back_to_unnamed_mode_outside_template_rule() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><a><b>text</b></a></doc>",
        r##"
<xsl:stylesheet xmlns:f="http://example.com/test"
                xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
                version="3.0">
  <xsl:template match="/">
    <out>
      <xsl:apply-templates select="doc/a" mode="m"/>
    </out>
  </xsl:template>

  <xsl:template match="a" mode="m">
    <xsl:sequence select="f:apply(.)"/>
  </xsl:template>

  <xsl:template match="b" mode="m">
    <m/>
  </xsl:template>

  <xsl:template match="b">
    <u/>
  </xsl:template>

  <xsl:function name="f:apply">
    <xsl:param name="node" as="element(a)"/>
    <xsl:apply-templates select="$node/b" mode="#current"/>
  </xsl:function>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out><u/></out>");
}

#[test]
fn test_mode_attributes_accept_whitespace_padded_values() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><a/></doc>",
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
                version=" 3.0 ">
  <xsl:mode name=" Q{}s " on-no-match=" shallow-skip " warning-on-no-match=" no "/>

  <xsl:template match="/">
    <out>
      <xsl:apply-templates select="doc" mode=" Q{}s "/>
    </out>
  </xsl:template>

  <xsl:template match="doc" mode=" s ">
    <xsl:apply-templates select="a" mode=" #current "/>
  </xsl:template>

  <xsl:template match="a" mode="Q{}s">
    <ok/>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out><ok/></out>");
}

#[test]
fn test_mode_typed_no_is_accepted() {
    parse(
        StaticContext::default(),
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:mode name="s" on-no-match="shallow-copy" typed=" no "/>
</xsl:stylesheet>"#,
    )
    .unwrap();
}

#[test]
fn test_mode_on_no_match_shallow_copy_preserves_attributes() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc a=\"1\"><child>text</child></doc>",
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:mode name="s" on-no-match="shallow-copy"/>

  <xsl:template match="/">
    <out>
      <xsl:apply-templates select="doc" mode="s"/>
    </out>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out><doc a=\"1\"><child>text</child></doc></out>"
    );
}

#[test]
fn test_mode_typed_yes_rejects_untyped_nodes() {
    let mut xot = Xot::new();
    let err = evaluate(
        &mut xot,
        "<doc/>",
        r##"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:mode name="s" typed="yes"/>

  <xsl:template match="/">
    <xsl:apply-templates select="doc" mode="s"/>
  </xsl:template>
</xsl:stylesheet>"##,
    )
    .unwrap_err();

    assert_eq!(err.value(), error::Error::XTTE3100);
}

#[test]
fn test_document_instruction_creates_document_node_variable() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:variable name="var1" as="document-node()">
    <xsl:document>
      <item>hello</item>
    </xsl:document>
  </xsl:variable>

  <xsl:template match="/doc">
    <out>
      <xsl:value-of select="$var1 instance of document-node()"/>
    </out>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>true</out>");
}

#[test]
fn test_document_instruction_satisfies_item_return_type() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:my="http://uri.test" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="/">
    <out>
      <xsl:call-template name="a5"/>
    </out>
  </xsl:template>

  <xsl:template name="a5" as="item()">
    <xsl:document>
      <my:item>1</my:item>
    </xsl:document>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        "<out><my:item xmlns:my=\"http://uri.test\">1</my:item></out>"
    );
}

#[test]
fn test_sequence_document_loads_relative_to_stylesheet_base_uri() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let temp_dir = std::env::temp_dir().join(format!(
        "xee-sequence-1202-{}-{}",
        std::process::id(),
        unique
    ));
    fs::create_dir_all(&temp_dir).unwrap();
    fs::write(temp_dir.join("sequence-1202a.xml"), "<doc/>").unwrap();

    let mut xot = Xot::new();
    let stylesheet_path = temp_dir.join("sequence-1202.xsl");
    let output = evaluate_with_stylesheet_base(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
  <xsl:template match="doc">
    <out>
      <xsl:text>(((</xsl:text>
      <xsl:sequence select="document('sequence-1202a.xml')"/>
      <xsl:text>)))</xsl:text>
    </out>
  </xsl:template>
</xsl:transform>"#,
        &stylesheet_path,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>(((<doc/>)))</out>");

    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_for_each_group_group_by_keeps_first_item_per_key() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc><record key='a' n='1'/><record key='a' n='2'/><record key='b' n='3'/></doc>",
        r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
  <xsl:template match="/">
    <out>
      <xsl:for-each-group select="doc/record" group-by="string(@key)">
        <xsl:value-of select="@n"/>
        <xsl:text>|</xsl:text>
      </xsl:for-each-group>
    </out>
  </xsl:template>
</xsl:stylesheet>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>1|3|</out>");
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
fn test_missing_name_attribute_reports_xtse0010() {
    let output = parse(
        StaticContext::default(),
        r#"
  <xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
    <xsl:variable select="'ABC'"/>
  </xsl:transform>"#,
    );

    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::XTSE0010,
            span: _
        })
    ));
}

#[test]
fn test_disallowed_with_param_attribute_reports_xtse0090() {
    let output = parse(
        StaticContext::default(),
        r#"
  <xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="2.0">
    <xsl:template match="/">
    <xsl:call-template name="temp1">
      <xsl:with-param name="par" select="'xyz'" required="yes"/>
    </xsl:call-template>
    </xsl:template>

    <xsl:template name="temp1">
    <xsl:param name="par"/>
    </xsl:template>
  </xsl:transform>"#,
    );

    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::XTSE0090,
            span: _
        })
    ));
}

#[test]
fn test_invalid_required_attribute_value_reports_xtse0020() {
    let output = parse(
        StaticContext::default(),
        r#"
  <xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0">
    <xsl:template name="foo">
    <xsl:param name="par1" required="TRUE"/>
    </xsl:template>
  </xsl:transform>"#,
    );

    assert!(matches!(
        output,
        error::SpannedResult::Err(error::SpannedError {
            error: error::Error::XTSE0020,
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
fn test_unused_local_variable_does_not_trigger_global_circularity() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        "<doc/>",
        r#"
<xsl:transform xmlns:my="http://www.my.com"
               xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
               version="2.0">
  <xsl:variable name="x" select="my:func(1)"/>

  <xsl:function name="my:func">
    <xsl:param name="a"/>
    <xsl:variable name="b" select="$x"/>
    <xsl:sequence select="$a + 2"/>
  </xsl:function>

  <xsl:template match="/doc">
    <out>
      <xsl:value-of select="$x"/>
    </out>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(xml(&xot, output), "<out>3</out>");
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

#[test]
fn test_xsl_element_with_prefixed_name_uses_static_namespace() {
    let mut xot = Xot::new();
    let output = evaluate(
        &mut xot,
        r#"<doc/>"#,
        r#"
<xsl:transform expand-text="true" xmlns:my="http://www.mytest.net" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="/">
    <o><xsl:element name="my:elem">content</xsl:element></o>
  </xsl:template>
</xsl:transform>"#,
    )
    .unwrap();

    assert_eq!(
        xml(&xot, output),
        r#"<o><my:elem xmlns:my="http://www.mytest.net">content</my:elem></o>"#
    );
}

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
