use anyhow::Result;

use xee_xpath::{context, Queries, Query};
use xee_xpath_load::{convert_string, ContextLoadable};

use crate::catalog::LoadContext;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Metadata {
    pub(crate) description: Option<String>,
    pub(crate) created: Option<Attribution>,
    pub(crate) modified: Vec<Modification>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Modification {
    pub(crate) attribution: Attribution,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Attribution {
    pub(crate) by: String,
    pub(crate) on: String, // should be a date
}

impl ContextLoadable<LoadContext> for Metadata {
    fn static_context_builder(context: &LoadContext) -> context::StaticContextBuilder {
        let mut builder = context::StaticContextBuilder::default();
        builder.default_element_namespace(context.catalog_ns);
        builder
    }

    fn load_with_context(
        queries: &Queries,
        _context: &LoadContext,
    ) -> Result<impl Query<Metadata>> {
        let description_query = queries.option("description/string()", convert_string)?;
        let by_query = queries.one("@by/string()", convert_string)?;
        let on_query = queries.one("@on/string()", convert_string)?;
        let by_query2 = by_query.clone();
        let on_query2 = on_query.clone();
        let created_query = queries.option("created", move |session, item| {
            {
                {
                    Ok(Attribution {
                        by: by_query.execute(session, item)?,
                        on: on_query.execute(session, item)?,
                    })
                }
            }
        })?;

        let change_query = queries.option("@change/string()", convert_string)?;
        let modified_query = queries.many("modified", move |session, item| {
            let attribution = Attribution {
                by: by_query2.execute(session, item)?,
                on: on_query2.execute(session, item)?,
            };
            let description = change_query.execute(session, item)?;
            Ok(Modification {
                attribution,
                description: description.unwrap_or("".to_string()),
            })
        })?;

        let metadata_query = queries.one(".", move |session, item| {
            let description = description_query.execute(session, item)?;
            let created = created_query.execute(session, item)?;
            let modified = modified_query.execute(session, item)?;
            Ok(Metadata {
                description,
                created,
                modified,
            })
        })?;

        Ok(metadata_query)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::language::XPathLanguage;

    use super::*;

    #[test]
    fn test_load() {
        let xml = r#"
<container xmlns="http://www.w3.org/2010/09/qt-fots-catalog">
  <description>Description</description>
  <created by="Foo Barson" on="2024-01-01"/>
</container>"#;
        let context = LoadContext::new::<XPathLanguage>(PathBuf::new());
        let metadata = Metadata::load_from_xml_with_context(xml, &context).unwrap();
        assert_eq!(
            metadata,
            Metadata {
                description: Some("Description".to_string()),
                created: Some(Attribution {
                    by: "Foo Barson".to_string(),
                    on: "2024-01-01".to_string(),
                }),
                modified: vec![],
            }
        );
        // Metadata::load_fom_xml(&mut xot, xml, XPATH_NS).unwrap();
    }
}
