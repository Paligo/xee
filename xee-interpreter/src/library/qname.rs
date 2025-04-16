// https://www.w3.org/TR/xpath-functions-31/#QName-funcs

use ahash::HashMap;
use xot::xmlname::NameStrInfo;
use xot::Xot;

use xee_name::{Name, Namespaces};
use xee_xpath_ast::parse_name;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::wrap_xpath_fn;

#[xpath_fn("fn:resolve-QName($qname as xs:string?, $element as element()) as xs:QName?")]
fn resolve_qname(
    interpreter: &Interpreter,
    qname: Option<&str>,
    node: xot::Node,
) -> error::Result<Option<atomic::Atomic>> {
    if let Some(qname) = qname {
        // TODO: we could make this more efficient if we could have a parser state
        // that used NamespaceLookup instead of Namespaces, but that requires a lot
        // of generics we're not ready for at this point.
        let namespaces = element_namespaces(node, interpreter.xot());
        let name = parse_name(qname, &namespaces)?.value;
        // parse_name doesn't put in the default namespace if necessary, so we do it here
        let name = name.with_default_namespace(namespaces.default_element_namespace());
        Ok(Some(name.into()))
    } else {
        Ok(None)
    }
}

fn element_namespaces(node: xot::Node, xot: &Xot) -> Namespaces {
    let mut m = HashMap::default();

    let mut default_element_namespace = "";

    for (prefix_id, namespace_id) in xot.namespaces_in_scope(node) {
        let prefix = xot.prefix_str(prefix_id).to_string();
        if m.contains_key(&prefix) {
            continue;
        }
        if xot.empty_prefix() == prefix_id {
            default_element_namespace = xot.namespace_str(namespace_id);
            // we don't continue as we want the empty prefix to be in the map
        }
        let namespace = xot.namespace_str(namespace_id).to_string();
        m.insert(prefix, namespace);
    }

    Namespaces::new(m, default_element_namespace.to_string(), "".to_string())
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
                return Err(error::Error::new(error::ErrorCode::FOCA0002));
            }
            vec![(prefix.to_string(), param_uri.to_string())]
        } else {
            // no prefix,will be parse error later
            vec![("".to_string(), param_uri.to_string())]
        }
    } else {
        // no prefix, so default namespace
        vec![("".to_string(), param_uri.to_string())]
    };
    let pairs = HashMap::from_iter(pairs);
    // TODO: see efficiency note for resolve-QName
    let namespaces = Namespaces::new(pairs, "".to_string(), "".to_string());
    let name = parse_name(param_qname, &namespaces)
        .map_err(|_| error::Error::new(error::ErrorCode::FOCA0002))?
        .value;
    // TODO: the parser should do this already
    // put in default namespace if required
    if name.namespace().is_empty() && !param_uri.is_empty() {
        Ok(name.with_default_namespace(param_uri).into())
    } else {
        Ok(name.into())
    }
}

#[xpath_fn("fn:prefix-from-QName($arg as xs:QName?) as xs:NCName?")]
fn prefix_from_qname(arg: Option<Name>) -> error::Result<Option<atomic::Atomic>> {
    if let Some(arg) = arg {
        let prefix = arg.prefix();
        if !prefix.is_empty() {
            Ok(Some(atomic::Atomic::String(
                atomic::StringType::NCName,
                prefix.to_string().into(),
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
            arg.local_name().to_string().into(),
        )))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:namespace-uri-from-QName($arg as xs:QName?) as xs:anyURI?")]
fn namespace_uri_from_qname(arg: Option<Name>) -> error::Result<Option<atomic::Atomic>> {
    if let Some(arg) = arg {
        let namespace = arg.namespace();
        if !namespace.is_empty() {
            Ok(Some(atomic::Atomic::String(
                atomic::StringType::AnyURI,
                namespace.to_string().into(),
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
    interpreter: &Interpreter,
    prefix: Option<&str>,
    node: xot::Node,
) -> error::Result<Option<atomic::Atomic>> {
    if let Some(prefix) = prefix {
        // TODO: efficiency could be made faster if we used NameSpaceLookup, see
        // resolve-QName

        let namespaces = element_namespaces(node, interpreter.xot());
        Ok(namespaces
            .by_prefix(prefix)
            .map(|s| atomic::Atomic::String(atomic::StringType::AnyURI, s.to_string().into())))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:in-scope-prefixes($element as element()) as xs:string*")]
fn in_scope_prefixes(interpreter: &Interpreter, node: xot::Node) -> Vec<atomic::Atomic> {
    let xot = interpreter.xot();
    xot.namespaces_in_scope(node)
        .map(|(prefix, _)| xot.prefix_str(prefix).to_string().into())
        .collect::<Vec<_>>()
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(resolve_qname),
        wrap_xpath_fn!(qname),
        wrap_xpath_fn!(prefix_from_qname),
        wrap_xpath_fn!(local_name_from_qname),
        wrap_xpath_fn!(namespace_uri_from_qname),
        wrap_xpath_fn!(namespace_uri_for_prefix),
        wrap_xpath_fn!(in_scope_prefixes),
    ]
}
