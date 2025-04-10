use xee_xslt_ast::{ast, error, parse_transform};

// TODO: currently only applies to default mode, no array handling yet
// TODO: we can't do | yet in a a pattern yet so
// define multiple template rules for now
const TEXT_ONLY_COPY: &str = r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
  <xsl:template match="document-node()">
    <xsl:apply-templates />
  </xsl:template>
  <xsl:template match="element()">
    <xsl:apply-templates />
  </xsl:template>
  <xsl:template match="text()">
    <xsl:value-of select="string(.)"/>
  </xsl:template>
  <xsl:template match="@*">
    <xsl:value-of select="string(.)"/>
  </xsl:template>
  <xsl:template match="processing-instruction()"/>
  <xsl:template match="comment()"/>
</xsl:stylesheet>
"#;

pub(crate) fn text_only_copy_declarations() -> Result<Vec<ast::Declaration>, error::ElementError> {
    let transform = parse_transform(TEXT_ONLY_COPY)?;
    Ok(transform.declarations)
}
