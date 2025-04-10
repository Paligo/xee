use xee_xslt_ast::{ast, error, parse_transform};

// TODO: currently only applies to default mode, no array handling yet
const TEXT_ONLY_COPY: &str = r#"
<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3">
<xsl:template match="document-node()">
    <xsl:apply-templates />
</xsl:template>
</xsl:stylesheet>
"#;

pub(crate) fn text_only_copy_declarations() -> Result<Vec<ast::Declaration>, error::ElementError> {
    let transform = parse_transform(TEXT_ONLY_COPY)?;
    Ok(transform.declarations)
}
