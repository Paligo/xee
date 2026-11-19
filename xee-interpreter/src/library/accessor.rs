// https://www.w3.org/TR/xpath-functions-31/#accessors
use xee_xpath_ast::ast;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::context;
use crate::context::BorrowWith;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::interpreter::Interpreter;
use crate::sequence;
use crate::wrap_xpath_fn;
use crate::xml::BaseUriResolver;

#[xpath_fn("fn:node-name($arg as node()?) as xs:QName?", context_first)]
fn node_name(
    interpreter: &Interpreter,
    arg: Option<xot::Node>,
) -> error::Result<Option<ast::Name>> {
    Ok(if let Some(node) = arg {
        interpreter.xot().node_name_ref(node)?.map(|n| n.to_owned())
    } else {
        None
    })
}

#[xpath_fn("fn:string($arg as item()?) as xs:string", context_first)]
fn string(interpreter: &Interpreter, arg: Option<sequence::Item>) -> error::Result<String> {
    if let Some(arg) = arg {
        arg.string_value(interpreter.xot())
    } else {
        Ok("".to_string())
    }
}

#[xpath_fn("fn:data($arg as item()*) as xs:anyAtomicType*", context_first)]
fn data(interpreter: &Interpreter, arg: &sequence::Sequence) -> error::Result<Vec<sequence::Item>> {
    let data = arg
        .atomized(interpreter.xot())
        .map(|atom| atom.map(|a| a.into()))
        .collect::<error::Result<Vec<sequence::Item>>>()?;
    Ok(data)
}

#[xpath_fn("fn:base-uri($arg as node()?) as xs:anyURI?", context_first)]
fn base_uri(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: Option<xot::Node>,
) -> error::Result<Option<atomic::Atomic>> {
    Ok(if let Some(node) = arg {
        // root node of the document
        let root = interpreter.xot().root(node);

        let base_uri = if matches!(interpreter.xot().value(root), xot::Value::Document) {
            // the base uri of the document is the one we can find registered, if available
            let documents = context.documents();
            let documents = documents.borrow();
            documents.get_uri_by_document_node(root)
        } else {
            None
        };

        // if we don't have a registered URI, use the static base uri
        let base_uri = base_uri.or_else(|| {
            context
                .static_context()
                .static_base_uri()
                .map(|u| u.to_owned().into())
        });
        let resolver = BaseUriResolver::new(base_uri.as_deref(), interpreter.state.xot_mut());
        let base_iri = resolver.base_uri(node)?;
        base_iri.map(|i| i.into())
    } else {
        None
    })
}

#[xpath_fn("fn:document-uri($arg as node()?) as xs:anyURI?", context_first)]
fn document_uri(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: Option<xot::Node>,
) -> error::Result<Option<atomic::Atomic>> {
    Ok(if let Some(node) = arg {
        // root node of the document
        let root = interpreter.xot().root(node);

        if matches!(interpreter.xot().value(root), xot::Value::Document) {
            // the base uri of the document is the one we can find registered, if available
            let documents = context.documents();
            let documents = documents.borrow();
            let document_uri = documents.get_uri_by_document_node(root);
            document_uri.map(|document_uri| document_uri.into())
        } else {
            None
        }
    } else {
        None
    })
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(node_name),
        wrap_xpath_fn!(string),
        wrap_xpath_fn!(data),
        wrap_xpath_fn!(base_uri),
        wrap_xpath_fn!(document_uri),
    ]
}
