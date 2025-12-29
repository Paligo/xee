use xot::{ValueType, Xot};

use xee_xpath_ast::ast;

use crate::sequence;

use super::kind_test::kind_test;
use super::TypeTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub axis: ast::Axis,
    pub node_test: ast::NodeTest,
}

pub(crate) fn resolve_step(
    step: &Step,
    node: xot::Node,
    xot: &Xot,
    type_table: &TypeTable,
) -> sequence::Sequence {
    let mut new_items = Vec::new();
    for axis_node in node_take_axis(&step.axis, xot, node) {
        if node_test(&step.node_test, &step.axis, xot, type_table, axis_node) {
            new_items.push(sequence::Item::Node(axis_node));
        }
    }
    new_items.into()
}

fn convert_axis(axis: &ast::Axis) -> xot::Axis {
    match axis {
        ast::Axis::Child => xot::Axis::Child,
        ast::Axis::Descendant => xot::Axis::Descendant,
        ast::Axis::Parent => xot::Axis::Parent,
        ast::Axis::Ancestor => xot::Axis::Ancestor,
        ast::Axis::FollowingSibling => xot::Axis::FollowingSibling,
        ast::Axis::PrecedingSibling => xot::Axis::PrecedingSibling,
        ast::Axis::Following => xot::Axis::Following,
        ast::Axis::Preceding => xot::Axis::Preceding,
        ast::Axis::DescendantOrSelf => xot::Axis::DescendantOrSelf,
        ast::Axis::AncestorOrSelf => xot::Axis::AncestorOrSelf,
        ast::Axis::Self_ => xot::Axis::Self_,
        ast::Axis::Attribute => xot::Axis::Attribute,
        ast::Axis::Namespace => unreachable!("Namespace axis should be forbidden at compile time"),
    }
}

fn node_take_axis<'a>(
    axis: &ast::Axis,
    xot: &'a Xot,
    node: xot::Node,
) -> Box<dyn Iterator<Item = xot::Node> + 'a> {
    let axis = convert_axis(axis);
    xot.axis(axis, node)
}

fn node_test(
    node_test: &ast::NodeTest,
    axis: &ast::Axis,
    xot: &Xot,
    type_table: &TypeTable,
    node: xot::Node,
) -> bool {
    match node_test {
        ast::NodeTest::KindTest(kt) => kind_test(kt, xot, type_table, node),
        ast::NodeTest::NameTest(name_test) => {
            if xot.value_type(node) != principal_node_kind(axis) {
                return false;
            }
            match name_test {
                ast::NameTest::Name(name) => {
                    let name = name.value.maybe_to_ref(xot);
                    if let Some(name) = name {
                        let name_id = name.name_id();
                        match xot.value(node) {
                            xot::Value::Element(element) => element.name() == name_id,
                            xot::Value::Attribute(attribute) => attribute.name() == name_id,
                            _ => false,
                        }
                    } else {
                        // if name isn't present in any XML document it's certainly
                        // false
                        false
                    }
                }
                ast::NameTest::Star => true,
                ast::NameTest::LocalName(local_name) => match xot.value(node) {
                    xot::Value::Element(element) => {
                        let name_id = element.name();
                        let (name_str, _) = xot.name_ns_str(name_id);
                        name_str == local_name
                    }
                    xot::Value::Attribute(attribute) => {
                        xot.local_name_str(attribute.name()) == local_name
                    }
                    _ => false,
                },
                ast::NameTest::Namespace(uri) => match xot.value(node) {
                    xot::Value::Element(element) => {
                        let name_id = element.name();
                        let namespace_str = xot.uri_str(name_id);
                        namespace_str == uri
                    }
                    xot::Value::Attribute(attribute) => {
                        let namespace_str = xot.uri_str(attribute.name());
                        namespace_str == uri
                    }
                    _ => false,
                },
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrincipalNodeKind {
    Element,
    Attribute,
    Namespace,
}

fn principal_node_kind(axis: &ast::Axis) -> ValueType {
    match axis {
        ast::Axis::Attribute => ValueType::Attribute,
        ast::Axis::Namespace => ValueType::Namespace,
        _ => ValueType::Element,
    }
}

#[cfg(test)]
mod tests {
    use xee_xpath_ast::{ast, WithSpan};

    use super::*;

    fn xot_nodes_to_value(node: &[xot::Node]) -> sequence::Sequence {
        node.iter()
            .map(|&node| sequence::Item::Node(node))
            .collect::<Vec<_>>()
            .into()
    }

    #[test]
    fn test_child_axis_star() -> Result<(), xot::Error> {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a/><b/></root>"#).unwrap();
        let doc_el = xot.document_element(doc)?;
        let a = xot.first_child(doc_el).unwrap();
        let b = xot.next_sibling(a).unwrap();
        let type_table = TypeTable::new();

        let step = Step {
            axis: ast::Axis::Child,
            node_test: ast::NodeTest::NameTest(ast::NameTest::Star),
        };
        let value = resolve_step(&step, doc_el, &xot, &type_table);
        assert_eq!(value, xot_nodes_to_value(&[a, b]));
        Ok(())
    }

    #[test]
    fn test_child_axis_name() -> Result<(), xot::Error> {
        let mut xot = Xot::new();
        let doc = xot.parse(r#"<root><a/><b/></root>"#).unwrap();
        let doc_el = xot.document_element(doc)?;
        let a = xot.first_child(doc_el).unwrap();
        let type_table = TypeTable::new();

        let step = Step {
            axis: ast::Axis::Child,
            node_test: ast::NodeTest::NameTest(ast::NameTest::Name(
                ast::Name::name("a").with_empty_span(),
            )),
        };
        let value = resolve_step(&step, doc_el, &xot, &type_table);
        assert_eq!(value, xot_nodes_to_value(&[a]));
        Ok(())
    }
}
