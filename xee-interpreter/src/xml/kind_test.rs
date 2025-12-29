use xee_schema_type::Xs;
use xot::Xot;

use xee_xpath_ast::ast;

use super::TypeTable;

pub(crate) fn kind_test(
    kind_test: &ast::KindTest,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
) -> bool {
    match kind_test {
        ast::KindTest::Document(dt) => document_test(dt.as_ref(), xot, type_table, node),
        ast::KindTest::Element(et) => element_test(et.as_ref(), xot, type_table, node),
        ast::KindTest::SchemaElement(_set) => {
            todo!()
        }
        ast::KindTest::Attribute(at) => attribute_test(at.as_ref(), xot, type_table, node),
        ast::KindTest::SchemaAttribute(_sat) => {
            todo!()
        }
        ast::KindTest::Any => true,
        // text() matches any text node
        ast::KindTest::Text => xot.is_text(node),
        // comment() matches any comment node
        ast::KindTest::Comment => xot.is_comment(node),
        ast::KindTest::NamespaceNode => xot.is_namespace_node(node),
        ast::KindTest::PI(pi_test) => {
            if !xot.is_processing_instruction(node) {
                return false;
            }
            if let Some(_pi_test) = pi_test {
                // processing-instruction N matches any processing-instruction node
                // whose PITarget is equal to fn:normalize-space(N)
                // TODO
                return false;
            }
            true
        }
    }
}

fn element_test(
    test: Option<&ast::ElementOrAttributeTest>,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
) -> bool {
    element_or_attribute_test(test, xot, type_table, node, |node, xot| xot.is_element(node))
}

fn attribute_test(
    test: Option<&ast::ElementOrAttributeTest>,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
) -> bool {
    element_or_attribute_test(
        test,
        xot,
        type_table,
        node,
        |node, xot| xot.is_attribute_node(node),
    )
}

fn document_test(
    test: Option<&ast::DocumentTest>,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
) -> bool {
    if !xot.is_document(node) {
        return false;
    }

    if let Some(document_test) = test {
        // get document element node

        // will always succeed as node is the root node
        let document_element_node = xot.document_element(node).unwrap();

        match document_test {
            ast::DocumentTest::Element(et) => {
                element_test(et.as_ref(), xot, type_table, document_element_node)
            }
            ast::DocumentTest::SchemaElement(_set) => {
                todo!()
            }
        }
    } else {
        true
    }
}

fn element_or_attribute_test(
    test: Option<&ast::ElementOrAttributeTest>,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
    node_type_match: impl Fn(xot::Node, &Xot) -> bool,
) -> bool {
    // if we're not the right node type (element, or attribute) then we
    // bail out
    if !node_type_match(node, xot) {
        return false;
    }

    if let Some(test) = test {
        // the name has to match first
        let name_matches = match &test.name_or_wildcard {
            ast::NameOrWildcard::Name(name) => {
                // TODO: what if we can't find a prefix here?
                if let Some(node_name) = xot.node_name_ref(node).unwrap() {
                    name.maybe_to_ref(xot) == Some(node_name)
                } else {
                    false
                }
            }
            ast::NameOrWildcard::Wildcard => true,
        };
        if !name_matches {
            return false;
        }
        // the type also has to match
        if let Some(type_name) = &test.type_name {
            type_annotation(xot, type_table, node).derives_from(type_name.name)
            // ignoring can_be_nilled for now
        } else {
            true
        }
    } else {
        // there is further test, so we're done
        true
    }
}

fn type_annotation(xot: &Xot, type_table: &TypeTable, node: xot::Node) -> Xs {
    // for now we don't know any types of nodes yet
    if let Some(xs) = type_table.get(node) {
        return xs;
    }
    if xot.is_element(node) {
        Xs::Untyped
    } else if xot.is_attribute_node(node) {
        Xs::UntypedAtomic
    } else {
        Xs::UntypedAtomic
    }
}

#[cfg(test)]
mod tests {
    use xee_xpath_ast::parse_kind_test;

    use super::*;

    #[test]
    fn test_kind_test_any() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a/><b/></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("node()").unwrap();
        assert!(kind_test(&kt, &xot, &type_table, doc));
        assert!(kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, a));
    }

    #[test]
    fn test_kind_test_text() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>content</a><b/></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let a_text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("text()").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(!kind_test(&kt, &xot, &type_table, a));
        assert!(kind_test(&kt, &xot, &type_table, a_text));
    }

    #[test]
    fn test_kind_test_comment() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><!-- comment --></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let comment = xot.first_child(doc_el).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("comment()").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, comment));
    }

    #[test]
    fn test_kind_test_document() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let type_table = TypeTable::new();
        let kt = parse_kind_test("document-node()").unwrap();
        assert!(kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
    }

    #[test]
    fn test_kind_test_element_without_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("element()").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, a));
        assert!(!kind_test(&kt, &xot, &type_table, text));
    }

    #[test]
    fn test_kind_test_element_with_wildcard() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("element(*)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, a));
        assert!(!kind_test(&kt, &xot, &type_table, text));
    }

    #[test]
    fn test_kind_test_element_with_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("element(a)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, a));
        assert!(!kind_test(&kt, &xot, &type_table, text));
    }

    #[test]
    fn test_kind_test_element_with_type_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("element(a, xs:untyped)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(kind_test(&kt, &xot, &type_table, a));
        assert!(!kind_test(&kt, &xot, &type_table, text));

        let kt = parse_kind_test("element(a, xs:untypedAtomic)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, a));

        // but we're not an xs:string
        let kt = parse_kind_test("element(a, xs:string)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, a));
    }

    #[test]
    fn test_kind_test_attribute_without_name() {
        let mut xot = Xot::new();
        let alpha = xot.add_name("alpha");
        let beta = xot.add_name("beta");
        let doc = xot
            .parse(r#"<root><a alpha="Alpha" beta="Beta">text</a></root>"#)
            .unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let alpha = xot.attributes(a).get_node(alpha).unwrap();
        let beta = xot.attributes(a).get_node(beta).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("attribute()").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(!kind_test(&kt, &xot, &type_table, a));
        assert!(kind_test(&kt, &xot, &type_table, alpha));
        assert!(kind_test(&kt, &xot, &type_table, beta));
        assert!(!kind_test(&kt, &xot, &type_table, text));
    }

    #[test]
    fn test_kind_test_attribute_with_name() {
        let mut xot = Xot::new();
        let alpha = xot.add_name("alpha");
        let beta = xot.add_name("beta");
        let doc = xot
            .parse(r#"<root><a alpha="Alpha" beta="Beta">text</a></root>"#)
            .unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let alpha = xot.attributes(a).get_node(alpha).unwrap();
        let beta = xot.attributes(a).get_node(beta).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("attribute(alpha)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(!kind_test(&kt, &xot, &type_table, a));
        assert!(kind_test(&kt, &xot, &type_table, alpha));
        assert!(!kind_test(&kt, &xot, &type_table, beta));
        assert!(!kind_test(&kt, &xot, &type_table, text));
    }

    #[test]
    fn test_kind_test_attribute_with_type_name() {
        let mut xot = Xot::new();
        let alpha = xot.add_name("alpha");
        let beta = xot.add_name("beta");
        let doc = xot
            .parse(r#"<root><a alpha="Alpha" beta="Beta">text</a></root>"#)
            .unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let alpha = xot.attributes(a).get_node(alpha).unwrap();
        let beta = xot.attributes(a).get_node(beta).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("attribute(alpha, xs:untypedAtomic)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(!kind_test(&kt, &xot, &type_table, a));
        assert!(kind_test(&kt, &xot, &type_table, alpha));
        assert!(!kind_test(&kt, &xot, &type_table, beta));
        assert!(!kind_test(&kt, &xot, &type_table, text));

        let kt = parse_kind_test("attribute(alpha, xs:string)").unwrap();
        assert!(!kind_test(&kt, &xot, &type_table, alpha));
    }

    #[test]
    fn test_kind_test_document_with_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();
        let type_table = TypeTable::new();

        let kt = parse_kind_test("document-node(element(root))").unwrap();
        assert!(kind_test(&kt, &xot, &type_table, doc));
        assert!(!kind_test(&kt, &xot, &type_table, doc_el));
        assert!(!kind_test(&kt, &xot, &type_table, a));
        assert!(!kind_test(&kt, &xot, &type_table, text));

        let kt = parse_kind_test("document-node(element(a))").unwrap();
        // the document doesn't match as its root node isn't 'a'
        assert!(!kind_test(&kt, &xot, &type_table, doc));
        // the 'a' node doesn't match either as it's not a document node
        assert!(!kind_test(&kt, &xot, &type_table, a));
    }
}
