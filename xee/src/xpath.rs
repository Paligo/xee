use clap::Parser;
use std::path::PathBuf;
use xee_xpath::context::StaticContextBuilder;
use xee_xpath::Itemable;
use xee_xpath::Query;
use crate::common::input_xml;
use crate::error::render_error;

#[derive(Debug, Parser)]
pub(crate) struct XPath {
    /// xpath expression
    pub(crate) xpath: String,
    /// input xml file (default stdin)
    pub(crate) infile: Option<PathBuf>,
    /// Namespace URI to use in XPath for element names without a namespace
    /// prefix.
    ///
    /// If omitted, the default namespace is the empty string (i.e. the
    /// names are not in a namespace).
    #[arg(long)]
    pub(crate) default_namespace_uri: Option<String>,
    /// Namespace declaration to make available in XPath (can be repeated)
    /// The format is prefix=uri.
    #[arg(long)]
    pub(crate) namespace: Vec<String>,
}

impl XPath {
    pub(crate) fn run(&self) -> Result<(), anyhow::Error> {
        let input_xml = input_xml(&self.infile)?;

        let mut documents = xee_xpath::Documents::new();
        let doc = documents.add_string_without_uri(&input_xml)?;

        let static_context_builder = make_static_context_builder(
            self.default_namespace_uri.as_deref(),
            self.namespace.as_slice(),
        )?;

        let queries = xee_xpath::Queries::new(static_context_builder);
        execute_query(&self.xpath, &queries, &mut documents, Some(doc))
    }
}

pub(crate) fn execute_query(
    xpath: &str,
    queries: &xee_xpath::Queries<'_>,
    documents: &mut xee_xpath::Documents,
    doc: Option<xee_xpath::DocumentHandle>,
) -> Result<(), anyhow::Error> {
    let sequence_query = queries.sequence(xpath);
    let sequence_query = match sequence_query {
        Ok(sequence_query) => sequence_query,
        Err(e) => {
            render_error(xpath, e);
            return Ok(());
        }
    };
    let mut context_builder = sequence_query.dynamic_context_builder(documents);
    if let Some(doc) = doc {
        context_builder.context_item(doc.to_item(documents)?);
    }
    let context = context_builder.build();

    let sequence = sequence_query.execute_with_context(documents, &context);
    let sequence = match sequence {
        Ok(sequence) => sequence,
        Err(e) => {
            render_error(xpath, e);
            return Ok(());
        }
    };
    println!(
        "{}",
        sequence.display_representation(documents.xot(), &context)
    );
    Ok(())
}

pub(crate) fn make_static_context_builder<'a>(
    default_namespace_uri: Option<&'a str>,
    namespaces: &'a [String],
) -> anyhow::Result<StaticContextBuilder<'a>> {
    let mut static_context_builder = xee_xpath::context::StaticContextBuilder::default();
    if let Some(default_namespace_uri) = default_namespace_uri {
        static_context_builder.default_element_namespace(default_namespace_uri);
    }
    let namespaces = namespaces
        .iter()
        .map(|declaration| {
            let mut parts = declaration.splitn(2, '=');
            let prefix = parts.next().ok_or(anyhow::anyhow!("missing prefix"))?;
            let uri = parts.next().ok_or(anyhow::anyhow!("missing uri"))?;
            Ok((prefix, uri))
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;

    static_context_builder.namespaces(namespaces);
    Ok(static_context_builder)
}
