// https://www.w3.org/TR/xpath-functions-31/#QName-funcs

use ahash::HashMap;
use std::rc::Rc;
use xot::Xot;

use xee_name::{Name, Namespaces};
use xee_xpath_ast::parse_name;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::context::DynamicContext;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::wrap_xpath_fn;
use crate::xml;

#[xpath_fn("fn:resolve-QName($qname as xs:string?, $element as element()) as xs:QName?")]
fn resolve_qname(
    context: &DynamicContext,
    qname: Option<&str>,
    node: xml::Node,
) -> error::Result<Option<atomic::Atomic>> {
    if let Some(qname) = qname {
        // TODO: we could make this more efficient if we could have a parser state
        // that used NamespaceLookup instead of Namespaces, but that requires a lot
        // of generics we're not ready for at this point.
        let namespaces = element_namespaces(node, context.xot());
        let name = parse_name(qname, &namespaces)?.value;
        Ok(Some(name.into()))
    } else {
        Ok(None)
    }
}

fn element_namespaces(node: xml::Node, xot: &Xot) -> Namespaces {
    let node = node.xot_node();
    let pairs = xot
        .namespaces(node)
        .map(|(prefix_id, namespace_id)| {
            (xot.prefix_str(prefix_id), xot.namespace_str(namespace_id))
        })
        .collect::<HashMap<_, _>>();

    Namespaces::new(pairs, None, None)
}

#[xpath_fn("fn:QName($paramURI as xs:string?, $paramQName as xs:string) as xs:QName")]
fn qname(param_uri: Option<&str>, param_qname: &str) -> error::Result<atomic::Atomic> {
    let param_uri = param_uri.unwrap_or("");

    // without doing the full parse, get the prefix so we can put it in
    // namespaces so it's looked up during the parse
    let mut prefix_split = param_qname.split(':');
    let pairs = if let Some(prefix) = prefix_split.next() {
        if prefix_split.next().is_some() {
            if param_uri.is_empty() {
                return Err(error::Error::FOCA0002);
            }
            vec![(prefix, param_uri)]
        } else {
            // no prefix,will be parse error later
            vec![("", param_uri)]
        }
    } else {
        // no prefix, so default namespace
        vec![("", param_uri)]
    };
    let pairs = HashMap::from_iter(pairs);
    // TODO: see efficiency note for resolve-QName
    let namespaces = Namespaces::new(pairs, None, None);
    let name = parse_name(param_qname, &namespaces)
        .map_err(|_| error::Error::FOCA0002)?
        .value;
    // TODO: the parser should do this already
    // put in default namespace if required
    if name.namespace().is_none() && !param_uri.is_empty() {
        Ok(name.with_default_namespace(Some(param_uri)).into())
    } else {
        Ok(name.into())
    }
}

#[xpath_fn("fn:prefix-from-QName($arg as xs:QName?) as xs:NCName?")]
fn prefix_from_qname(arg: Option<Name>) -> error::Result<Option<atomic::Atomic>> {
    if let Some(arg) = arg {
        if let Some(prefix) = arg.prefix() {
            Ok(Some(atomic::Atomic::String(
                atomic::StringType::NCName,
                Rc::new(prefix.to_string()),
            )))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:local-name-from-QName($arg as xs:QName?) as xs:NCName?")]
fn local_name_from_qname(arg: Option<Name>) -> error::Result<Option<atomic::Atomic>> {
    if let Some(arg) = arg {
        Ok(Some(atomic::Atomic::String(
            atomic::StringType::NCName,
            Rc::new(arg.local_name().to_string()),
        )))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:namespace-uri-from-QName($arg as xs:QName?) as xs:anyURI?")]
fn namespace_uri_from_qname(arg: Option<Name>) -> error::Result<Option<atomic::Atomic>> {
    if let Some(arg) = arg {
        if let Some(namespace) = arg.namespace() {
            Ok(Some(atomic::Atomic::String(
                atomic::StringType::AnyURI,
                Rc::new(namespace.to_string()),
            )))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

#[xpath_fn(
    "fn:namespace-uri-for-prefix($prefix as xs:string?, $element as element()) as xs:anyURI?"
)]
fn namespace_uri_for_prefix(
    context: &DynamicContext,
    prefix: Option<&str>,
    node: xml::Node,
) -> error::Result<Option<atomic::Atomic>> {
    if let Some(prefix) = prefix {
        // TODO: efficiency could be made faster if we used NameSpaceLookup, see
        // resolve-QName

        let namespaces = element_namespaces(node, context.xot());
        Ok(namespaces
            .by_prefix(prefix)
            .map(|s| atomic::Atomic::String(atomic::StringType::AnyURI, Rc::new(s.to_string()))))
    } else {
        Ok(None)
    }
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(resolve_qname),
        wrap_xpath_fn!(qname),
        wrap_xpath_fn!(prefix_from_qname),
        wrap_xpath_fn!(local_name_from_qname),
        wrap_xpath_fn!(namespace_uri_from_qname),
        wrap_xpath_fn!(namespace_uri_for_prefix),
    ]
}
