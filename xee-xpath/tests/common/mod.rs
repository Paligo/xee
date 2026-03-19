// disable dead code warning for this module as for some reason
// it thinks these functions are not used, even though they are
#![allow(dead_code)]

use xee_xpath::{
    context::{StaticContextBuilder, Variables},
    error, Documents, Item, Queries, Query, Sequence,
};
use xot::Xot;

pub(crate) fn run(s: &str) -> error::Result<Sequence> {
    let mut documents = Documents::new();
    let queries = Queries::default();
    let q = queries.sequence(s)?;
    q.execute_build_context(&mut documents, |_builder| ())
}

pub(crate) fn run_with_variables(s: &str, variables: Variables) -> error::Result<Sequence> {
    let mut documents = Documents::new();
    let queries = Queries::default();
    let mut static_context_builder = StaticContextBuilder::default();
    static_context_builder.variable_names(variables.keys().cloned());
    let static_context = static_context_builder.build();
    let q = queries.sequence_with_context(s, static_context)?;
    q.execute_build_context(&mut documents, |builder| {
        builder.variables(variables);
    })
}

pub(crate) fn run_xml(xml: &str, xpath: &str) -> error::Result<Sequence> {
    let mut documents = Documents::new();
    let handle = documents.add_string_without_uri(xml).unwrap();
    let queries = Queries::default();
    let q = queries.sequence(xpath)?;
    q.execute(&mut documents, handle)
}

pub(crate) fn run_xml_default_ns(xml: &str, xpath: &str, ns: &str) -> error::Result<Sequence> {
    let mut documents = Documents::new();
    let handle = documents.add_string_without_uri(xml).unwrap();
    let mut static_context_builder = StaticContextBuilder::default();
    static_context_builder.default_element_namespace(ns);
    let queries = Queries::new(static_context_builder);
    let q = queries.sequence(xpath)?;
    q.execute(&mut documents, handle)
}

pub(crate) fn assert_nodes<S>(xml: &str, xpath: &str, get_nodes: S) -> error::Result<()>
where
    S: Fn(&Xot, xot::Node) -> Vec<xot::Node>,
{
    let mut documents = Documents::new();
    let handle = documents.add_string_without_uri(xml).unwrap();
    let root = documents.document_node(handle).unwrap();
    let nodes = get_nodes(documents.xot(), root);

    let queries = Queries::default();
    let q = queries.sequence(xpath)?;

    let result = q.execute(&mut documents, handle)?;

    assert_eq!(result, xot_nodes_to_items(&nodes));
    Ok(())
}

pub(crate) fn run_xml_with_ns(
    xml: &str,
    xpath: &str,
    namespaces: &[(&str, &str)],
) -> error::Result<Sequence> {
    let mut documents = Documents::new();
    let handle = documents.add_string_without_uri(xml).unwrap();
    let mut static_context_builder = StaticContextBuilder::default();
    for (prefix, uri) in namespaces {
        static_context_builder.add_namespace(prefix, uri);
    }
    let queries = Queries::new(static_context_builder);
    let q = queries.sequence(xpath)?;
    q.execute(&mut documents, handle)
}

fn xot_nodes_to_items(node: &[xot::Node]) -> Sequence {
    Sequence::from(
        node.iter()
            .map(|&node| Item::from(node))
            .collect::<Vec<_>>(),
    )
}
