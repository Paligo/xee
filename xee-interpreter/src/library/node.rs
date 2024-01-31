// https://www.w3.org/TR/xpath-functions-31/#node-functions

use ahash::HashSet;
use ahash::HashSetExt;
use std::rc::Rc;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::context::DynamicContext;
use crate::function::StaticFunctionDescription;
use crate::wrap_xpath_fn;
use crate::xml;

#[xpath_fn("fn:name($arg as node()?) as xs:string", context_first)]
fn name(context: &DynamicContext, arg: Option<xml::Node>) -> String {
    if let Some(node) = arg {
        let name = node.node_name(context.xot());
        if let Some(name) = name {
            name.to_full_name()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    }
}

#[xpath_fn("fn:local-name($arg as node()?) as xs:string", context_first)]
fn local_name(context: &DynamicContext, arg: Option<xml::Node>) -> String {
    if let Some(arg) = arg {
        arg.local_name(context.xot())
    } else {
        "".to_string()
    }
}

#[xpath_fn("fn:namespace-uri($arg as node()?) as xs:anyURI", context_first)]
fn namespace_uri(context: &DynamicContext, arg: Option<xml::Node>) -> atomic::Atomic {
    if let Some(arg) = arg {
        atomic::Atomic::String(
            atomic::StringType::AnyURI,
            Rc::new(arg.namespace_uri(context.xot())),
        )
    } else {
        atomic::Atomic::String(atomic::StringType::AnyURI, "".to_string().into())
    }
}

#[xpath_fn("fn:root($arg as node()?) as node()?", context_first)]
fn root(context: &DynamicContext, arg: Option<xml::Node>) -> Option<xml::Node> {
    if let Some(arg) = arg {
        let xot_node = match arg {
            xml::Node::Xot(node) => node,
            xml::Node::Attribute(node, _) => node,
            xml::Node::Namespace(node, _) => node,
        };
        // XXX there should be a xot.root() to obtain this in one step
        let top = context.xot().top_element(xot_node);
        let root = context.xot().parent(top).unwrap();

        Some(xml::Node::Xot(root))
    } else {
        None
    }
}

#[xpath_fn("fn:has-children($node as node()?) as xs:boolean", context_first)]
fn has_children(context: &DynamicContext, node: Option<xml::Node>) -> bool {
    if let Some(node) = node {
        match node {
            xml::Node::Xot(node) => context.xot().first_child(node).is_some(),
            xml::Node::Attribute(_, _) => false,
            xml::Node::Namespace(_, _) => false,
        }
    } else {
        false
    }
}

#[xpath_fn("fn:innermost($nodes as node()*) as node()*")]
fn innermost(context: &DynamicContext, nodes: &[xml::Node]) -> Vec<xml::Node> {
    // get sequence of ancestors
    let mut ancestors = HashSet::new();
    for node in nodes {
        let mut parent_node = *node;
        // insert all parents into ancestors
        while let Some(parent) = parent_node.parent(context.xot()) {
            ancestors.insert(parent);
            parent_node = parent;
        }
    }
    // now find all nodes that are not in ancestors
    let mut innermost = Vec::new();
    for node in nodes {
        if !ancestors.contains(node) {
            innermost.push(*node);
        }
    }
    innermost
}

#[xpath_fn("fn:outermost($nodes as node()*) as node()*")]
fn outermost(context: &DynamicContext, nodes: &[xml::Node]) -> Vec<xml::Node> {
    let node_set = nodes.iter().collect::<HashSet<_>>();
    // now find all nodes that don't have an ancestor in the set
    let mut outermost = Vec::new();
    'outer: for node in nodes {
        let mut parent_node = *node;
        // if we find an ancestor in node_set, then we don't add this node
        while let Some(parent) = parent_node.parent(context.xot()) {
            if node_set.contains(&parent) {
                continue 'outer;
            }
            parent_node = parent;
        }
        outermost.push(*node);
    }
    outermost
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(name),
        wrap_xpath_fn!(local_name),
        wrap_xpath_fn!(namespace_uri),
        wrap_xpath_fn!(root),
        wrap_xpath_fn!(has_children),
        wrap_xpath_fn!(innermost),
        wrap_xpath_fn!(outermost),
    ]
}
