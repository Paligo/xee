use xee_schema_type::Xs;
use xot::Xot;
use xot::xmlname::NameStrInfo;

use xee_xpath_ast::ast;

pub(crate) fn kind_test(kind_test: &ast::KindTest, xot: &Xot, node: xot::Node) -> bool {
    match kind_test {
        ast::KindTest::Document(dt) => document_test(dt.as_ref(), xot, node),
        ast::KindTest::Element(et) => element_test(et.as_ref(), xot, node),
        ast::KindTest::SchemaElement(set) => schema_element_test(set, xot, node),
        ast::KindTest::Attribute(at) => attribute_test(at.as_ref(), xot, node),
        ast::KindTest::SchemaAttribute(sat) => schema_attribute_test(sat, xot, node),
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
            if let Some(pi_test) = pi_test {
                // processing-instruction N matches any processing-instruction node
                // whose PITarget is equal to fn:normalize-space(N)
                return processing_instruction_name_test(pi_test, xot, node);
            }
            true
        }
    }
}

fn processing_instruction_name_test(
    pi_test: &ast::PITest,
    xot: &Xot,
    node: xot::Node,
) -> bool {
    let processing_instruction = xot.processing_instruction(node).unwrap();
    let (target, _) = xot.name_ns_str(processing_instruction.target());
    let expected = match pi_test {
        ast::PITest::Name(name) | ast::PITest::StringLiteral(name) => normalize_space(name),
    };
    target == expected
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn element_test(test: Option<&ast::ElementOrAttributeTest>, xot: &Xot, node: xot::Node) -> bool {
    element_or_attribute_test(test, xot, node, |node, xot| xot.is_element(node))
}

fn attribute_test(test: Option<&ast::ElementOrAttributeTest>, xot: &Xot, node: xot::Node) -> bool {
    element_or_attribute_test(test, xot, node, |node, xot| xot.is_attribute_node(node))
}

fn schema_element_test(test: &ast::SchemaElementTest, xot: &Xot, node: xot::Node) -> bool {
    named_node_test(&test.name, xot, node, |node, xot| xot.is_element(node))
}

fn schema_attribute_test(test: &ast::SchemaAttributeTest, xot: &Xot, node: xot::Node) -> bool {
    named_node_test(&test.name, xot, node, |node, xot| xot.is_attribute_node(node))
}

fn document_test(test: Option<&ast::DocumentTest>, xot: &Xot, node: xot::Node) -> bool {
    if !xot.is_document(node) {
        return false;
    }

    if let Some(document_test) = test {
        // get document element node

        // will always succeed as node is the root node
        let document_element_node = xot.document_element(node).unwrap();

        match document_test {
            ast::DocumentTest::Element(et) => element_test(et.as_ref(), xot, document_element_node),
            ast::DocumentTest::SchemaElement(set) => {
                schema_element_test(set, xot, document_element_node)
            }
        }
    } else {
        true
    }
}

fn named_node_test(
    expected_name: &ast::Name,
    xot: &Xot,
    node: xot::Node,
    node_type_match: impl Fn(xot::Node, &Xot) -> bool,
) -> bool {
    if !node_type_match(node, xot) {
        return false;
    }

    xot.node_name(node).is_some_and(|node_name| {
        expected_name.local_name() == xot.local_name_str(node_name)
            && expected_name.namespace() == xot.uri_str(node_name)
    })
}

fn element_or_attribute_test(
    test: Option<&ast::ElementOrAttributeTest>,
    xot: &Xot,
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
                if let Some(node_name) = xot.node_name(node) {
                    name.local_name() == xot.local_name_str(node_name)
                        && name.namespace() == xot.uri_str(node_name)
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
            type_annotation(xot, node).derives_from(type_name.name)
            // ignoring can_be_nilled for now
        } else {
            true
        }
    } else {
        // there is further test, so we're done
        true
    }
}

fn type_annotation(_xot: &Xot, _node: xot::Node) -> Xs {
    if _xot.is_attribute_node(_node) {
        Xs::UntypedAtomic
    } else {
        Xs::Untyped
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

        let kt = parse_kind_test("node()").unwrap();
        assert!(kind_test(&kt, &xot, doc));
        assert!(kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, a));
    }

    #[test]
    fn test_kind_test_text() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>content</a><b/></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let a_text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("text()").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(!kind_test(&kt, &xot, a));
        assert!(kind_test(&kt, &xot, a_text));
    }

    #[test]
    fn test_kind_test_comment() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><!-- comment --></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let comment = xot.first_child(doc_el).unwrap();

        let kt = parse_kind_test("comment()").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, comment));
    }

    #[test]
    fn test_kind_test_processing_instruction_with_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><?Process fun?></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let pi = xot.first_child(doc_el).unwrap();

        let kt = parse_kind_test("processing-instruction(Process)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, pi));

        let kt = parse_kind_test("processing-instruction(Other)").unwrap();
        assert!(!kind_test(&kt, &xot, pi));
    }

    #[test]
    fn test_kind_test_processing_instruction_with_string_literal_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><?Process fun?></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let pi = xot.first_child(doc_el).unwrap();

        let kt = parse_kind_test("processing-instruction('  Process  ')").unwrap();
        assert!(kind_test(&kt, &xot, pi));
    }

    #[test]
    fn test_kind_test_document() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let kt = parse_kind_test("document-node()").unwrap();
        assert!(kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
    }

    #[test]
    fn test_kind_test_element_without_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("element()").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, a));
        assert!(!kind_test(&kt, &xot, text));
    }

    #[test]
    fn test_kind_test_element_with_wildcard() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("element(*)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, a));
        assert!(!kind_test(&kt, &xot, text));
    }

    #[test]
    fn test_kind_test_element_with_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("element(a)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, a));
        assert!(!kind_test(&kt, &xot, text));
    }

    #[test]
    fn test_kind_test_element_with_type_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("element(a, xs:untyped)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(kind_test(&kt, &xot, a));
        assert!(!kind_test(&kt, &xot, text));

        let kt = parse_kind_test("element(a, xs:untypedAtomic)").unwrap();
        assert!(!kind_test(&kt, &xot, a));

        // but we're not an xs:string
        let kt = parse_kind_test("element(a, xs:string)").unwrap();
        assert!(!kind_test(&kt, &xot, a));
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

        let kt = parse_kind_test("attribute()").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(!kind_test(&kt, &xot, a));
        assert!(kind_test(&kt, &xot, alpha));
        assert!(kind_test(&kt, &xot, beta));
        assert!(!kind_test(&kt, &xot, text));
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

        let kt = parse_kind_test("attribute(alpha)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(!kind_test(&kt, &xot, a));
        assert!(kind_test(&kt, &xot, alpha));
        assert!(!kind_test(&kt, &xot, beta));
        assert!(!kind_test(&kt, &xot, text));
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

        let kt = parse_kind_test("attribute(alpha, xs:untypedAtomic)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(!kind_test(&kt, &xot, a));
        assert!(kind_test(&kt, &xot, alpha));
        assert!(!kind_test(&kt, &xot, beta));
        assert!(!kind_test(&kt, &xot, text));

        let kt = parse_kind_test("attribute(alpha, xs:string)").unwrap();
        assert!(!kind_test(&kt, &xot, alpha));
    }

    #[test]
    fn test_kind_test_document_with_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();
        let doc_el = xot.document_element(doc).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let text = xot.first_child(a).unwrap();

        let kt = parse_kind_test("document-node(element(root))").unwrap();
        assert!(kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, doc_el));
        assert!(!kind_test(&kt, &xot, a));
        assert!(!kind_test(&kt, &xot, text));

        let kt = parse_kind_test("document-node(element(a))").unwrap();
        // the document doesn't match as its root node isn't 'a'
        assert!(!kind_test(&kt, &xot, doc));
        // the 'a' node doesn't match either as it's not a document node
        assert!(!kind_test(&kt, &xot, a));
    }

    #[test]
    fn test_kind_test_document_with_element_type_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a>text</a></root>"#).unwrap();

        let kt = parse_kind_test("document-node(element(root, xs:untyped))").unwrap();
        assert!(kind_test(&kt, &xot, doc));

        let kt = parse_kind_test("document-node(element(root, xs:untypedAtomic))").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
    }

    #[test]
    fn test_kind_test_schema_element_matches_by_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a/></root>"#).unwrap();
        let root = xot.document_element(doc).unwrap();
        let a = xot.first_child(root).unwrap();

        let kt = parse_kind_test("schema-element(a)").unwrap();
        assert!(!kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, root));
        assert!(kind_test(&kt, &xot, a));
    }

    #[test]
    fn test_kind_test_schema_attribute_matches_by_name() {
        let mut xot = Xot::new();
        let alpha_name = xot.add_name("alpha");
        let doc = xot.parse(r#"<root><a alpha="1"/></root>"#).unwrap();
        let root = xot.document_element(doc).unwrap();
        let a = xot.first_child(root).unwrap();
        let alpha = xot.attributes(a).get_node(alpha_name).unwrap();

        let kt = parse_kind_test("schema-attribute(alpha)").unwrap();
        assert!(!kind_test(&kt, &xot, a));
        assert!(kind_test(&kt, &xot, alpha));
    }

    #[test]
    fn test_kind_test_document_schema_element_matches_document_element_name() {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a/></root>"#).unwrap();
        let root = xot.document_element(doc).unwrap();

        let kt = parse_kind_test("document-node(schema-element(root))").unwrap();
        assert!(kind_test(&kt, &xot, doc));
        assert!(!kind_test(&kt, &xot, root));
    }
}
