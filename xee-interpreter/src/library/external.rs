use iri_string::types::{IriReferenceStr, IriString};
use xee_xpath_macros::xpath_fn;

use crate::{
    context::DynamicContext, error, function::StaticFunctionDescription, sequence::Sequence,
    wrap_xpath_fn,
};

#[xpath_fn("fn:doc($uri as xs:string?) as document-node()?")]
fn doc(context: &DynamicContext, uri: Option<&str>) -> error::Result<Option<xot::Node>> {
    if let Some(uri) = uri {
        document_node(context, uri)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:document($uri as xs:string?) as document-node()?")]
fn document(context: &DynamicContext, uri: Option<&str>) -> error::Result<Option<xot::Node>> {
    doc(context, uri)
}

#[xpath_fn("fn:doc-available($uri as xs:string?) as xs:boolean")]
fn doc_available(context: &DynamicContext, uri: Option<&str>) -> bool {
    if let Some(uri) = uri {
        document_node(context, uri).is_ok()
    } else {
        false
    }
}

fn document_node(context: &DynamicContext, uri: &str) -> error::Result<Option<xot::Node>> {
    let iri_reference: &IriReferenceStr = uri.try_into().map_err(|_| error::Error::FODC0005)?;
    let uri = absolute_uri(context, iri_reference)?;

    // first check whether a document is there at all, if so, return it
    let documents = context.documents();
    let documents = documents.borrow();
    let document = documents.get_by_uri(&uri);

    if let Some(document) = document {
        Ok(Some(document.root()))
    } else {
        // The document doesn't exist, so return an error
        Err(error::Error::FODC0002)
    }
}

#[xpath_fn("fn:collection() as item()*")]
fn collection(context: &DynamicContext) -> error::Result<Sequence> {
    if let Some(collection) = context.default_collection() {
        Ok(collection.clone())
    } else {
        Err(error::Error::FODC0002)
    }
}

#[xpath_fn("fn:collection($uri as xs:string?) as item()*")]
fn collection_by_uri(context: &DynamicContext, uri: Option<&str>) -> error::Result<Sequence> {
    if let Some(uri) = uri {
        let iri_reference: &IriReferenceStr = uri.try_into().map_err(|_| error::Error::FODC0004)?;
        let uri = absolute_uri(context, iri_reference)?;
        if let Some(collection) = context.collection(&uri) {
            Ok(collection.clone())
        } else {
            Err(error::Error::FODC0002)
        }
    } else if let Some(collection) = context.default_collection() {
        Ok(collection.clone())
    } else {
        Err(error::Error::FODC0002)
    }
}

#[xpath_fn("fn:uri-collection() as xs:anyURI*")]
fn uri_collection(context: &DynamicContext) -> error::Result<Sequence> {
    if let Some(collection) = context.default_uri_collection() {
        Ok(collection.clone())
    } else {
        Err(error::Error::FODC0002)
    }
}

#[xpath_fn("fn:uri-collection($uri as xs:string?) as xs:anyURI*")]
fn uri_collection_by_uri(context: &DynamicContext, uri: Option<&str>) -> error::Result<Sequence> {
    if let Some(uri) = uri {
        let iri_reference: &IriReferenceStr = uri.try_into().map_err(|_| error::Error::FODC0004)?;
        let uri = absolute_uri(context, iri_reference)?;
        if let Some(collection) = context.uri_collection(&uri) {
            Ok(collection.clone())
        } else {
            Err(error::Error::FODC0002)
        }
    } else if let Some(collection) = context.default_uri_collection() {
        Ok(collection.clone())
    } else {
        Err(error::Error::FODC0002)
    }
}

fn absolute_uri(context: &DynamicContext, uri: &IriReferenceStr) -> error::Result<IriString> {
    let uri: IriString = match uri.to_iri() {
        Ok(iri) => iri.into(),
        Err(relative_iri) => {
            let base = context.static_context().static_base_uri();
            if let Some(base) = base {
                relative_iri.resolve_against(base).into()
            } else {
                return Err(error::Error::FODC0002);
            }
        }
    };
    Ok(uri)
}

#[xpath_fn("fn:environment-variable($name as xs:string) as xs:string?")]
fn environment_variable(context: &DynamicContext, name: &str) -> Option<String> {
    context.environment_variable(name).map(|s| s.to_string())
}

#[xpath_fn("fn:available-environment-variables() as xs:string*")]
fn available_environment_variables(context: &DynamicContext) -> Vec<String> {
    context
        .environment_variable_names()
        .map(|s| s.to_string())
        .collect()
}

// https://www.w3.org/TR/xpath-functions-31/#fns-on-docs
pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(doc),
        wrap_xpath_fn!(document),
        wrap_xpath_fn!(doc_available),
        wrap_xpath_fn!(collection),
        wrap_xpath_fn!(collection_by_uri),
        wrap_xpath_fn!(uri_collection),
        wrap_xpath_fn!(uri_collection_by_uri),
        wrap_xpath_fn!(environment_variable),
        wrap_xpath_fn!(available_environment_variables),
    ]
}
