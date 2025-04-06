// https://www.w3.org/TR/2017/REC-xpath-functions-31-20170321/#context

use xee_name::{Name, Namespaces, FN_NAMESPACE};
use xee_xpath_ast::ast;
use xee_xpath_macros::xpath_fn;

use crate::atomic;
use crate::atomic::NaiveDateWithOffset;
use crate::atomic::NaiveTimeWithOffset;
use crate::context::DynamicContext;
use crate::error;
use crate::function::FunctionKind;
use crate::function::StaticFunctionDescription;
use crate::interpreter;
use crate::sequence;
use crate::wrap_xpath_fn;

use super::datetime::offset_to_duration;

fn bound_position(
    _context: &DynamicContext,
    _interpreter: &mut interpreter::Interpreter,
    arguments: &[sequence::Sequence],
) -> error::Result<sequence::Sequence> {
    // position should be the context value
    Ok(arguments[0].clone())
}

fn bound_last(
    _context: &DynamicContext,
    _interpreter: &mut interpreter::Interpreter,
    arguments: &[sequence::Sequence],
) -> error::Result<sequence::Sequence> {
    // size should be the context value
    Ok(arguments[0].clone())
}

#[xpath_fn("fn:current-dateTime() as xs:dateTimeStamp")]
fn current_date_time(context: &DynamicContext) -> chrono::DateTime<chrono::offset::FixedOffset> {
    context.current_datetime()
}

#[xpath_fn("fn:current-date() as xs:date")]
fn current_date(context: &DynamicContext) -> NaiveDateWithOffset {
    NaiveDateWithOffset {
        date: context.current_datetime().naive_local().date(),
        offset: Some(context.implicit_timezone()),
    }
}

#[xpath_fn("fn:current-time() as xs:time")]
fn current_time(context: &DynamicContext) -> NaiveTimeWithOffset {
    NaiveTimeWithOffset {
        time: context.current_datetime().time(),
        offset: Some(context.implicit_timezone()),
    }
}

#[xpath_fn("fn:implicit-timezone() as xs:dayTimeDuration")]
fn implicit_timezone(context: &DynamicContext) -> chrono::Duration {
    offset_to_duration(context.implicit_timezone())
}

#[xpath_fn("fn:default-collation() as xs:string")]
fn default_collation(context: &DynamicContext) -> String {
    context.static_context().default_collation_uri().to_string()
}

#[xpath_fn("fn:static-base-uri() as xs:anyURI?")]
fn static_base_uri(context: &DynamicContext) -> Option<atomic::Atomic> {
    context
        .static_context()
        .static_base_uri()
        .map(|uri| atomic::Atomic::String(atomic::StringType::AnyURI, uri.to_string().into()))
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        StaticFunctionDescription {
            name: Name::new(
                "position".to_string(),
                FN_NAMESPACE.to_string(),
                String::new(),
            ).into(),
            signature: ast::Signature::parse("fn:position() as xs:integer", &Namespaces::default())
                .unwrap()
                .into(),
            function_kind: Some(FunctionKind::Position),
            func: bound_position,
        },
        StaticFunctionDescription {
            name: Name::new("last".to_string(), FN_NAMESPACE.to_string(), String::new()).into(),
            signature: ast::Signature::parse("fn:last() as xs:integer", &Namespaces::default())
                .unwrap()
                .into(),
            function_kind: Some(FunctionKind::Size),
            func: bound_last,
        },
        wrap_xpath_fn!(current_date_time),
        wrap_xpath_fn!(current_date),
        wrap_xpath_fn!(current_time),
        wrap_xpath_fn!(implicit_timezone),
        wrap_xpath_fn!(default_collation),
        wrap_xpath_fn!(static_base_uri),
    ]
}
