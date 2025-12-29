use xee_schema_type::Xs;
use xee_xpath_macros::xpath_fn;
use xot::Xot;

use crate::{
    atomic, context, error, function, interpreter::Interpreter, sequence, wrap_xpath_fn,
    xml::TypeTable,
};

use super::StaticFunctionDescription;

#[xpath_fn("fn:parse-json($json_text as xs:string?) as item()?")]
fn parse_json1(json_text: Option<&str>) -> error::Result<Option<sequence::Item>> {
    if let Some(json_text) = json_text {
        let value = json::parse(json_text).map_err(|_| error::Error::FOJS0001)?;
        // the spec seems to imply escape should be true by default, but then
        // various tests fail (and escape false by default seems more
        // reasonable) See https://github.com/w3c/qt3tests/issues/65
        Ok(parse_json_value(&value, false)?)
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:parse-json($json_text as xs:string?, $options as map(*)) as item()?")]
fn parse_json2(
    context: &context::DynamicContext,
    interpreter: &mut Interpreter,
    json_text: Option<&str>,
    options: function::Map,
) -> error::Result<Option<sequence::Item>> {
    let parameters =
        ParseJsonParameters::from_map(
            &options,
            context.static_context(),
            interpreter.xot(),
            &interpreter.state.type_table.borrow(),
        )?;

    if let Some(json_text) = json_text {
        let value = json::parse(json_text).map_err(|_| error::Error::FOJS0001)?;
        Ok(parse_json_value(&value, parameters.escape)?)
    } else {
        Ok(None)
    }
}

enum Duplicates {
    Reject,
    UseFirst,
    UseLast,
}

struct ParseJsonParameters {
    // liberal is entirely ignored. we don't have a more liberal JSON parser
    liberal: bool,
    // We cannot actually handle duplicates, as the Rust json crate
    // does not report duplicate information and effectively implements
    // `use-last` semantics (most common according to the JSON RFC)
    duplicates: Duplicates,
    // I don't understand why escape=true even exists, as it imports JSON
    // escaping rules into XML land where they have no meaning? But it's the
    // default! we implement it by re-escaping...
    escape: bool,
    // TODO: fallback
}

impl ParseJsonParameters {
    fn from_map(
        map: &function::Map,
        static_context: &context::StaticContext,
        xot: &Xot,
        type_table: &TypeTable,
    ) -> error::Result<Self> {
        let c = sequence::OptionParameterConverter::new(map, static_context, xot, type_table);

        let liberal = c
            .option_with_default("liberal", Xs::Boolean, false)
            .map_err(|_| error::Error::FOJS0005)?;
        let duplicates = c
            .option_with_default("duplicates", Xs::String, "use-first".to_string())
            .map_err(|_| error::Error::FOJS0005)?;
        let duplicates = match duplicates.as_str() {
            "reject" => Duplicates::Reject,
            "use-first" => Duplicates::UseFirst,
            "use-last" => Duplicates::UseLast,
            _ => return Err(error::Error::FOJS0005),
        };
        // the escape is true by default, weird but following the spec.
        // weirdly enough the default is `false` (contrary to the spec) in
        // case the map isn't passed at all.
        // See https://github.com/w3c/qt3tests/issues/65
        let escape = c
            .option_with_default("escape", Xs::Boolean, true)
            .map_err(|_| error::Error::FOJS0005)?;

        Ok(Self {
            liberal,
            duplicates,
            escape,
        })
    }
}

fn parse_json_value(
    value: &json::JsonValue,
    escape: bool,
) -> error::Result<Option<sequence::Item>> {
    match value {
        json::JsonValue::Null => Ok(None),
        json::JsonValue::Short(s) => Ok(Some(parse_json_string(s.to_string(), escape).into())),
        json::JsonValue::String(s) => Ok(Some(parse_json_string(s.to_string(), escape).into())),
        json::JsonValue::Number(n) => {
            let f: f64 = (*n).into();
            let atomic: atomic::Atomic = f.into();
            Ok(Some(atomic.into()))
        }
        json::JsonValue::Boolean(b) => {
            let atomic = atomic::Atomic::Boolean(*b);
            Ok(Some(atomic.into()))
        }
        json::JsonValue::Array(a) => {
            let mut entries = Vec::with_capacity(a.len());
            for value in a.iter() {
                let value = parse_json_value(value, escape)?;
                let sequence: sequence::Sequence = value.into();
                entries.push(sequence);
            }
            let array = function::Array::new(entries);
            let function = function::Function::Array(array);
            Ok(Some(function.into()))
        }
        json::JsonValue::Object(o) => {
            let mut entries = Vec::with_capacity(o.len());

            for (key, value) in o.iter() {
                let key = parse_json_string(key.to_string(), escape);
                let value = parse_json_value(value, escape)?;
                let sequence: sequence::Sequence = value.into();
                entries.push((key.clone(), sequence));
            }
            let map = function::Map::new(entries)?;
            let function = function::Function::Map(map);
            Ok(Some(function.into()))
        }
    }
}

fn parse_json_string(s: String, escape: bool) -> atomic::Atomic {
    let s = s.to_string();
    let s = if escape {
        v_jsonescape::escape(&s).to_string()
    } else {
        s
    };
    let atomic: atomic::Atomic = s.into();
    atomic
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![wrap_xpath_fn!(parse_json1), wrap_xpath_fn!(parse_json2)]
}
