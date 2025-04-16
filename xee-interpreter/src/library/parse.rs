// https://www.w3.org/TR/xpath-functions-31/#parsing-and-serializing

use xee_xpath_macros::xpath_fn;

use crate::{context, error, function, interpreter::Interpreter, sequence, wrap_xpath_fn};

use super::StaticFunctionDescription;

#[xpath_fn("fn:parse-xml($arg as xs:string?) as document-node(element(*))?")]
fn parse_xml(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: Option<&str>,
) -> error::Result<Option<xot::Node>> {
    if let Some(arg) = arg {
        let documents = context.documents();
        let handle = documents
            .borrow_mut()
            .add_string(interpreter.xot_mut(), None, arg)
            .map_err(|_| error::Error::new(error::ErrorCode::FODC0006))?;
        let doc = documents
            .borrow()
            .get_node_by_handle(handle)
            .ok_or(error::Error::new(error::ErrorCode::FODC0006))?;
        Ok(Some(doc))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:parse-xml-fragment($arg as xs:string?) as document-node()?")]
fn parse_xml_fragment(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: Option<&str>,
) -> error::Result<Option<xot::Node>> {
    if let Some(arg) = arg {
        let documents = context.documents();
        let handle = documents
            .borrow_mut()
            .add_fragment_string(interpreter.xot_mut(), arg)
            .map_err(|_| error::Error::new(error::ErrorCode::FODC0006))?;
        let doc = documents
            .borrow()
            .get_node_by_handle(handle)
            .ok_or(error::Error::new(error::ErrorCode::FODC0006))?;
        Ok(Some(doc))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:serialize($arg as item()*) as xs:string")]
fn serialize1(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: &sequence::Sequence,
) -> error::Result<String> {
    let map = function::Map::new(vec![])?;
    let serialization_parameters = sequence::SerializationParameters::from_map(
        map,
        context.static_context(),
        interpreter.xot_mut(),
    )?;
    arg.serialize(serialization_parameters, interpreter.xot_mut())
}

#[xpath_fn("fn:serialize($arg as item()*, $params as item()?) as xs:string")]
fn serialize2(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    arg: &sequence::Sequence,
    params: Option<sequence::Item>,
) -> error::Result<String> {
    let map = if let Some(params) = params {
        if let sequence::Item::Function(function::Function::Map(map)) = params {
            map.clone()
        } else {
            // TODO: handle element(output::serialization-parameters)
            return Err(error::Error::new(error::ErrorCode::XPTY0004));
        }
    } else {
        function::Map::new(vec![])?
    };
    let serialization_parameters = sequence::SerializationParameters::from_map(
        map,
        context.static_context(),
        interpreter.xot_mut(),
    )?;
    arg.serialize(serialization_parameters, interpreter.xot_mut())
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(parse_xml),
        wrap_xpath_fn!(parse_xml_fragment),
        wrap_xpath_fn!(serialize1),
        wrap_xpath_fn!(serialize2),
    ]
}
