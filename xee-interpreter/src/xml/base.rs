use iri_string::types::{IriAbsoluteString, IriReferenceStr, IriStr, IriString};
use xot;

use crate::error;

pub(crate) struct BaseUriResolver<'a> {
    document_base_uri: Option<&'a IriStr>,
    xml_base_name: xot::NameId,
    xot: &'a xot::Xot,
}

impl<'a> BaseUriResolver<'a> {
    pub(crate) fn new(document_base_uri: Option<&'a IriStr>, xot: &'a mut xot::Xot) -> Self {
        let xml_base_name = xot.add_name_ns("base", xot.xml_namespace());
        Self {
            document_base_uri,
            xml_base_name,
            xot,
        }
    }

    // NOTE: the specification is rather silent about errors - when relative base
    // URIs cannot be resolved one way or another. Here I handle such errors,
    // but it's not clear what to do with the error message - the fn:base-uri
    // operation defines no such errors.
    // FIXME: the XML base spec has a whole thing about the URLs not being
    // proper URIs but LEIRIs https://www.w3.org/TR/leiri/ which are supposed
    // to handle character data differently. I haven't done any work to verify
    // that behavior.
    pub(crate) fn base_uri(&self, node: xot::Node) -> Result<Option<IriString>, error::Error> {
        Ok(match self.xot.value(node) {
            xot::Value::Document => self.document_base_uri.map(|u| u.to_owned()),
            xot::Value::Element(_) => {
                let base = self.xot.attributes(node).get(self.xml_base_name);

                if let Some(base) = base {
                    let base: &IriReferenceStr = base
                        .as_str()
                        .try_into()
                        .map_err(|_| error::Error::new(error::ErrorCode::FORG0002))?;
                    match base.to_iri() {
                        Ok(iri) => Some(iri.to_owned()),
                        Err(iri) => {
                            // iri is relative, so resolve against the
                            // parent's base uri
                            if let Some(parent) = self.xot.parent(node) {
                                let base = self.base_uri(parent)?;
                                if let Some(base) = base {
                                    let base: IriAbsoluteString =
                                        base.try_into().map_err(|_| {
                                            error::Error::new(error::ErrorCode::FORG0002)
                                        })?;
                                    let iri = iri.resolve_against(&base);
                                    Some(iri.into())
                                } else {
                                    // no base URI, so how to resolve?
                                    return Err(error::Error::new(error::ErrorCode::FORG0009));
                                }
                            } else {
                                // no parent, so how to resolve?
                                return Err(error::Error::new(error::ErrorCode::FORG0002));
                            }
                        }
                    }
                } else if let Some(parent) = self.xot.parent(node) {
                    self.base_uri(parent)?
                } else {
                    None
                }
            }
            // NOTE: Processing instruction is defined to have a base URI by
            // itself, but it's based on either its parent or the document, and
            // that is the same behavior as attribute, text and comment nodes.
            // Maybe I got something wrong?
            xot::Value::Attribute(_)
            | xot::Value::Comment(_)
            | xot::Value::Text(_)
            | xot::Value::ProcessingInstruction(_) => {
                if let Some(parent) = self.xot.parent(node) {
                    self.base_uri(parent)?
                } else {
                    None
                }
            }
            xot::Value::Namespace(_) => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use xot::Xot;

    use super::*;

    // unfortunately almost all the tests in fn/base-uri.xml require
    // XQuery to work. Since we don't have xquery, we implement some
    // tests here to make sure we have the basic behavior right.

    #[test]
    fn test_base_uri_comment() {
        // Evaluation of base-uri function with argument set to a directly
        // constructed comment
        let mut xot = Xot::new();
        let comment = xot.new_comment("comment");
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(comment).unwrap();
        assert_eq!(base_uri, None);
    }

    #[test]
    fn test_base_uri_text() {
        // Evaluation of base-uri function with argument set to a computed
        // constructed Text node.
        let mut xot = Xot::new();
        let text = xot.new_text("text");
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(text).unwrap();
        assert_eq!(base_uri, None);
    }

    #[test]
    fn test_base_uri_element_without_base() {
        let mut xot = Xot::new();
        let name = xot.add_name("foo");
        let element = xot.new_element(name);
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(element).unwrap();
        assert_eq!(base_uri, None);
    }

    #[test]
    fn test_base_uri_element_with_base() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/bar"/>"#)
            .unwrap();
        let element = xot.document_element(doc).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(element).unwrap();
        let expected: IriString = "http://example.com/bar".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_element_with_base_inherited() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/bar"><bar/></foo>"#)
            .unwrap();
        let element = xot.document_element(doc).unwrap();
        let inherited = xot.first_child(element).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(inherited).unwrap();
        let expected: IriString = "http://example.com/bar".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_element_with_base_resolve_relative() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/bar"><bar xml:base="qux" /></foo>"#)
            .unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.first_child(foo).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        let expected: IriString = "http://example.com/qux".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_element_with_base_resolve_relative_complicated() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/"><bar xml:base="foo/../xml" /></foo>"#)
            .unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.first_child(foo).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        let expected: IriString = "http://example.com/xml".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_element_with_base_resolve_relative_empty() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/examples"><bar xml:base="" /></foo>"#)
            .unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.first_child(foo).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        let expected: IriString = "http://example.com/examples".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_element_with_base_resolve_absolute() {
        let mut xot = Xot::new();
        let doc = xot
            .parse(r#"<foo xml:base="http://example.com/examples"><bar xml:base="http://completely.different" /></foo>"#)
            .unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.first_child(foo).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        let expected: IriString = "http://completely.different".try_into().unwrap();
        assert_eq!(base_uri, Some(expected));
    }

    #[test]
    fn test_base_uri_document_without_base() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<foo/>"#).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(doc).unwrap();
        assert_eq!(base_uri, None);
    }

    #[test]
    fn test_base_uri_document_with_base() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<foo/>"#).unwrap();
        let base: &IriStr = "http://example.com/bar".try_into().unwrap();
        let resolver = BaseUriResolver::new(Some(base), &mut xot);
        let base_uri = resolver.base_uri(doc).unwrap();
        assert_eq!(base_uri, Some(base.to_owned()));
    }

    #[test]
    fn test_base_uri_element_with_document_base() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<foo/>"#).unwrap();
        let foo = xot.document_element(doc).unwrap();
        let base: &IriStr = "http://example.com/bar".try_into().unwrap();
        let resolver = BaseUriResolver::new(Some(base), &mut xot);
        let base_uri = resolver.base_uri(foo).unwrap();
        assert_eq!(base_uri, Some(base.to_owned()));
    }

    #[test]
    fn test_base_uri_attribute_without_base() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let doc = xot.parse(r#"<foo bar="baz"/>"#).unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.attributes(foo).get_node(bar_name).unwrap();
        let resolver = BaseUriResolver::new(None, &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        assert_eq!(base_uri, None);
    }

    #[test]
    fn test_base_uri_attribute_with_base() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let doc = xot.parse(r#"<foo bar="baz"/>"#).unwrap();
        let foo = xot.document_element(doc).unwrap();
        let bar = xot.attributes(foo).get_node(bar_name).unwrap();
        let base: &IriStr = "http://example.com/bar".try_into().unwrap();
        let resolver = BaseUriResolver::new(Some(base), &mut xot);
        let base_uri = resolver.base_uri(bar).unwrap();
        assert_eq!(base_uri, Some(base.to_owned()));
    }

    #[test]
    fn test_base_namespace_with_base() {
        let mut xot = Xot::new();
        let ns_prefix = xot.add_prefix("ns");
        let doc = xot
            .parse(r#"<ns:foo xmlns:ns="http://example.com"/>"#)
            .unwrap();
        let foo = xot.document_element(doc).unwrap();
        let ns_node = xot.namespaces(foo).get_node(ns_prefix).unwrap();
        let base: &IriStr = "http://example.com/bar".try_into().unwrap();
        let resolver = BaseUriResolver::new(Some(base), &mut xot);
        // even if supplied with a base namespace nodes never have one
        let base_uri = resolver.base_uri(ns_node).unwrap();
        assert_eq!(base_uri, None);
    }
}
