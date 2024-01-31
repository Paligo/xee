// https://www.w3.org/TR/xpath-functions-31/#accessors
use xee_xpath_ast::ast;
use xee_xpath_macros::xpath_fn;

use crate::context::DynamicContext;
use crate::error;
use crate::function::StaticFunctionDescription;
use crate::sequence;
use crate::wrap_xpath_fn;
use crate::xml;

#[xpath_fn("fn:node-name($arg as node()?) as xs:QName?", context_first)]
fn node_name(context: &DynamicContext, arg: Option<xml::Node>) -> Option<ast::Name> {
    if let Some(node) = arg {
        node.node_name(context.xot())
    } else {
        None
    }
}

#[xpath_fn("fn:string($arg as item()?) as xs:string", context_first)]
fn string(context: &DynamicContext, arg: Option<sequence::Item>) -> error::Result<String> {
    if let Some(arg) = arg {
        arg.string_value(context.xot())
    } else {
        Ok("".to_string())
    }
}

#[xpath_fn("fn:data($arg as item()*) as xs:anyAtomicType*", context_first)]
fn data(context: &DynamicContext, arg: &sequence::Sequence) -> error::Result<Vec<sequence::Item>> {
    let data = arg
        .atomized(context.xot())
        .map(|atom| atom.map(|a| a.into()))
        .collect::<error::Result<Vec<sequence::Item>>>()?;
    Ok(data)
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(node_name),
        wrap_xpath_fn!(string),
        wrap_xpath_fn!(data),
    ]
}
