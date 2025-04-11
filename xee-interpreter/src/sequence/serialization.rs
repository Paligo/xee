use ahash::HashMap;
use rust_decimal::Decimal;
use xot::{xmlname::OwnedName, Xot};

use xee_schema_type::Xs;

use crate::{
    atomic, context, error,
    function::{self, Map},
};

use super::{
    core::Sequence,
    item::Item,
    opc::{OptionParameterConverter, QNameOrString},
};

pub struct SerializationParameters {
    pub allow_duplicate_names: bool,
    pub byte_order_mark: bool,
    pub cdata_section_elements: Vec<OwnedName>,
    pub doctype_public: Option<String>,
    pub doctype_system: Option<String>,
    pub encoding: String,
    pub escape_uri_attributes: bool,
    pub html_version: Decimal,
    pub include_content_type: bool,
    pub indent: bool,
    pub item_separator: String,
    pub json_node_output_method: QNameOrString,
    pub media_type: Option<String>,
    pub method: QNameOrString,
    pub normalization_form: Option<String>,
    pub omit_xml_declaration: bool,
    pub standalone: Option<bool>,
    pub suppress_indentation: Vec<OwnedName>,
    pub undeclare_prefixes: bool,
    pub use_character_maps: HashMap<char, String>,
    pub version: String,
}

impl SerializationParameters {
    // default values are as used in XSLT 3.0
    pub fn new() -> Self {
        Self {
            allow_duplicate_names: false,
            byte_order_mark: false,
            cdata_section_elements: Vec::new(),
            doctype_public: None,
            doctype_system: None,
            encoding: "utf-8".to_string(),
            escape_uri_attributes: true,
            html_version: Decimal::from_str_exact("5.0").unwrap(),
            include_content_type: true,
            indent: false,
            item_separator: " ".to_string(),
            json_node_output_method: QNameOrString::String("xml".to_string()),
            media_type: Some("text/xml".to_string()),
            method: QNameOrString::String("xml".to_string()),
            normalization_form: None,
            omit_xml_declaration: false,
            standalone: None,
            suppress_indentation: Vec::new(),
            undeclare_prefixes: false,
            use_character_maps: HashMap::default(),
            version: "1.0".to_string(),
        }
    }

    pub(crate) fn from_map(
        map: Map,
        static_context: &context::StaticContext,
        xot: &Xot,
    ) -> error::Result<Self> {
        let c = OptionParameterConverter::new(&map, static_context, xot);
        let allow_duplicate_names =
            c.option_with_default("allow-duplicate-names", Xs::Boolean, false)?;

        let byte_order_mark = c.option_with_default("byte-order-mark", Xs::Boolean, false)?;

        let cdata_section_elements = c.many("cdata-section-elements", Xs::QName)?;

        let doctype_public = c.option("doctype-public", Xs::String)?;

        let doctype_system = c.option("doctype-system", Xs::String)?;

        let encoding = c.option_with_default("encoding", Xs::String, "utf-8".to_string())?;

        let escape_uri_attributes =
            c.option_with_default("escape-uri-attributes", Xs::Boolean, true)?;

        let html_version = c.option_with_default(
            "html-version",
            Xs::Decimal,
            Decimal::from_str_exact("5.0").unwrap(),
        )?;

        let include_content_type =
            c.option_with_default("include-content-type", Xs::Boolean, true)?;

        let indent = c.option_with_default("indent", Xs::Boolean, false)?;

        let item_separator =
            c.option_with_default("item-separator", Xs::String, " ".to_string())?;

        let json_node_output_method = c.qname_or_string(
            "json-node-output-method",
            QNameOrString::String("xml".to_string()),
        )?;

        let media_type = c.option("media-type", Xs::String)?;

        let method = c.qname_or_string("method", QNameOrString::String("xml".to_string()))?;

        let normalization_form = c.option("normalization-form", Xs::String)?;

        let omit_xml_declaration =
            c.option_with_default("omit-xml-declaration", Xs::Boolean, true)?;

        let standalone = c.option("standalone", Xs::Boolean)?;

        let suppress_indentation = c.many("suppress-indentation", Xs::QName)?;

        let undeclare_prefixes = c.option_with_default("undeclare-prefixes", Xs::Boolean, false)?;

        // TODO: use-character-maps

        let version = c.option_with_default("version", Xs::String, "1.0".to_string())?;

        Ok(Self {
            allow_duplicate_names,
            byte_order_mark,
            cdata_section_elements,
            doctype_public,
            doctype_system,
            encoding,
            escape_uri_attributes,
            html_version,
            include_content_type,
            indent,
            item_separator,
            json_node_output_method,
            media_type,
            method,
            normalization_form,
            omit_xml_declaration,
            standalone,
            suppress_indentation,
            undeclare_prefixes,
            use_character_maps: HashMap::default(),
            version,
        })
    }

    pub(crate) fn xml_in_json_serialization(method: &QNameOrString) -> Self {
        Self {
            // use the method given
            method: method.clone(),
            // the only thing set according to the specification
            omit_xml_declaration: true,
            // keep this around just in case, though I don't think we
            // can end up in json output from XML output
            json_node_output_method: method.clone(),
            allow_duplicate_names: false,
            byte_order_mark: false,
            cdata_section_elements: Vec::new(),
            doctype_public: None,
            doctype_system: None,
            encoding: "utf-8".to_string(),
            escape_uri_attributes: false,
            html_version: Decimal::from_str_exact("5.0").unwrap(),
            include_content_type: false,
            indent: false,
            item_separator: " ".to_string(),
            media_type: None,
            normalization_form: None,
            standalone: None,
            suppress_indentation: Vec::new(),
            undeclare_prefixes: false,
            use_character_maps: HashMap::default(),
            version: "1.0".to_string(),
        }
    }
}

impl Default for SerializationParameters {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn serialize_sequence(
    arg: &Sequence,
    parameters: SerializationParameters,
    xot: &mut Xot,
) -> error::Result<String> {
    if let Some(local_name) = parameters.method.local_name() {
        match local_name {
            "xml" => serialize_xml(arg, parameters, xot),
            "html" => serialize_html(arg, parameters, xot),
            "json" => serialize_json(arg, parameters, xot),
            _ => Err(error::Error::SEPM0016),
        }
    } else {
        Err(error::Error::SEPM0016)
    }
}

fn serialize_xml(
    arg: &Sequence,
    parameters: SerializationParameters,
    xot: &mut Xot,
) -> Result<String, error::Error> {
    let node = arg.normalize(&parameters.item_separator, xot)?;
    let indentation = xot_indentation(&parameters, xot);
    let cdata_section_elements = xot_names(&parameters.cdata_section_elements, xot);
    let declaration = if !parameters.omit_xml_declaration {
        Some(xot::output::xml::Declaration {
            encoding: Some(parameters.encoding.to_string()),
            standalone: parameters.standalone,
        })
    } else {
        None
    };
    let doctype = match (parameters.doctype_public, parameters.doctype_system) {
        (Some(public), Some(system)) => Some(xot::output::xml::DocType::Public { public, system }),
        (None, Some(system)) => Some(xot::output::xml::DocType::System { system }),
        // TODO: this should really not happen?
        (Some(public), None) => Some(xot::output::xml::DocType::Public {
            public,
            system: "".to_string(),
        }),
        (None, None) => None,
    };
    let output_parameters = xot::output::xml::Parameters {
        indentation,
        cdata_section_elements,
        declaration,
        doctype,
        ..Default::default()
    };

    Ok(xot.serialize_xml_string(output_parameters, node)?)
}

fn serialize_html(
    arg: &Sequence,
    parameters: SerializationParameters,
    xot: &mut Xot,
) -> Result<String, error::Error> {
    let node = arg.normalize(&parameters.item_separator, xot)?;
    // TODO: no check yet for html version rejecting versions that aren't 5
    let cdata_section_elements = xot_names(&parameters.cdata_section_elements, xot);
    let indentation = xot_indentation(&parameters, xot);
    let html5 = xot.html5();
    let output_parameters = xot::output::html5::Parameters {
        indentation,
        cdata_section_elements,
    };
    Ok(html5.serialize_string(output_parameters, node)?)
}

fn serialize_json(
    arg: &Sequence,
    parameters: SerializationParameters,
    xot: &mut Xot,
) -> Result<String, error::Error> {
    let r = serialize_json_sequence(arg, &parameters, xot)?;
    Ok(r.dump())
}

fn serialize_json_sequence(
    arg: &Sequence,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    match arg {
        Sequence::One(item) => serialize_json_item(item.item(), parameters, xot),
        Sequence::Empty(_) => Ok(json::JsonValue::Null),
        Sequence::Many(_) | Sequence::Range(_) => Err(error::Error::SERE0023),
    }
}

fn serialize_json_item(
    item: &Item,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    match item {
        Item::Atomic(atomic) => serialize_json_atomic(atomic, parameters),
        Item::Node(node) => serialize_json_node(*node, parameters, xot),
        Item::Function(function) => serialize_json_function(function, parameters, xot),
    }
}

fn serialize_json_atomic(
    atomic: &atomic::Atomic,
    parameters: &SerializationParameters,
) -> Result<json::JsonValue, error::Error> {
    match atomic {
        atomic::Atomic::Float(float) => {
            let f = float.into_inner();
            if f.is_infinite() || f.is_nan() {
                return Err(error::Error::SERE0020);
            }
            Ok(json::JsonValue::Number(f.into()))
        }
        atomic::Atomic::Double(double) => {
            let d = double.into_inner();
            if d.is_infinite() || d.is_nan() {
                return Err(error::Error::SERE0020);
            }
            Ok(json::JsonValue::Number(d.into()))
        }
        atomic::Atomic::Decimal(decimal) => {
            let d: f64 = (*decimal.as_ref())
                .try_into()
                .map_err(|_| error::Error::SERE0020)?;
            Ok(json::JsonValue::Number(d.into()))
        }
        atomic::Atomic::Integer(_t, integer) => {
            let i: f64 = integer.to_f64();
            Ok(json::JsonValue::Number(i.into()))
        }
        atomic::Atomic::Boolean(b) => Ok(json::JsonValue::Boolean(*b)),
        _ => {
            let s = atomic.string_value();
            Ok(serialize_json_string(s, parameters))
        }
    }
}

fn serialize_json_string(s: String, _parameters: &SerializationParameters) -> json::JsonValue {
    // TODO: normalization-form

    // NOTE: tests serialize-json-127 and serialize-json-128 fail because
    // the forward slash (solidus) character is not escaped. This is because
    // the json crate does not do so. This is consistent with the JSON RFC
    // https://softwareengineering.stackexchange.com/questions/444480/json-rfc8259-escape-forward-slash-or-not
    // but not consistent with the serialization spec which wrongfully manadates
    // it anyway.
    // https://www.w3.org/TR/xslt-xquery-serialization-31/#json-output
    json::JsonValue::String(s)
}

fn serialize_json_node(
    node: xot::Node,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    match parameters.json_node_output_method.local_name() {
        Some("xml") | Some("html") => {
            let xml_parameters = SerializationParameters::xml_in_json_serialization(
                &parameters.json_node_output_method,
            );
            let sequence: Sequence = vec![node].into();
            let s = serialize_sequence(&sequence, xml_parameters, xot)?;
            Ok(serialize_json_string(s, parameters))
        }
        _ => todo!(),
    }
}

fn serialize_json_function(
    function: &function::Function,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    match function {
        function::Function::Array(array) => serialize_json_array(array, parameters, xot),
        function::Function::Map(map) => serialize_json_map(map, parameters, xot),
        _ => Err(error::Error::SERE0021),
    }
}

fn serialize_json_array(
    array: &function::Array,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    let mut result = Vec::with_capacity(array.len());
    for entry in array.iter() {
        let serialized = serialize_json_sequence(entry, parameters, xot)?;
        result.push(serialized);
    }
    Ok(json::JsonValue::Array(result))
}

fn serialize_json_map(
    map: &function::Map,
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Result<json::JsonValue, error::Error> {
    let mut result = json::object::Object::new();
    for key in map.keys() {
        let key_s = key.string_value();
        let value = map.get(key).unwrap();
        let value = serialize_json_sequence(value, parameters, xot)?;
        result.insert(&key_s, value);
    }
    Ok(json::JsonValue::Object(result))
}

fn xot_indentation(
    parameters: &SerializationParameters,
    xot: &mut Xot,
) -> Option<xot::output::Indentation> {
    if !parameters.indent {
        return None;
    }
    let suppress = xot_names(&parameters.suppress_indentation, xot);
    Some(xot::output::Indentation { suppress })
}

fn xot_names(names: &[xot::xmlname::OwnedName], xot: &mut Xot) -> Vec<xot::NameId> {
    names
        .iter()
        .map(|owned_name| owned_name.to_ref(xot).name_id())
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{atomic, sequence};

    use super::*;

    #[test]
    fn test_allow_duplicate_names_true() {
        let map = Map::new(vec![(
            "allow-duplicate-names".to_string().into(),
            sequence::Sequence::from(vec![atomic::Atomic::Boolean(true)]),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert!(params.allow_duplicate_names);
    }

    #[test]
    fn test_allow_duplicate_names_false() {
        let map = Map::new(vec![(
            "allow-duplicate-names".to_string().into(),
            sequence::Sequence::from(vec![atomic::Atomic::Boolean(false)]),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert!(!params.allow_duplicate_names);
    }

    #[test]
    fn test_allow_duplicate_names_default_empty_sequence() {
        let map = Map::new(vec![(
            "allow-duplicate-names".to_string().into(),
            sequence::Sequence::default(),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert!(!params.allow_duplicate_names);
    }

    #[test]
    fn test_allow_duplicate_names_missing() {
        let map = Map::new(vec![]).unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert!(!params.allow_duplicate_names);
    }

    #[test]
    fn test_cdata_section_elements() {
        let html = OwnedName::new("html".to_string(), "".to_string(), "".to_string());
        let script = OwnedName::new("script".to_string(), "".to_string(), "".to_string());
        let map = Map::new(vec![(
            "cdata-section-elements".to_string().into(),
            sequence::Sequence::from(vec![
                atomic::Atomic::QName(html.clone().into()),
                atomic::Atomic::QName(script.clone().into()),
            ]),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert_eq!(params.cdata_section_elements.len(), 2);
        assert_eq!(params.cdata_section_elements[0], html);
        assert_eq!(params.cdata_section_elements[1], script);
    }

    #[test]
    fn test_qname_or_string_string() {
        let json: atomic::Atomic = "json".to_string().into();
        let map = Map::new(vec![(
            "json-node-output-method".to_string().into(),
            sequence::Sequence::from(vec![json]),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert_eq!(
            params.json_node_output_method,
            QNameOrString::String("json".to_string())
        );
    }

    #[test]
    fn test_qname_or_string_qname() {
        let owned_name = OwnedName::new("json".to_string(), "".to_string(), "".to_string());
        let json: atomic::Atomic = owned_name.clone().into();
        let map = Map::new(vec![(
            "json-node-output-method".to_string().into(),
            sequence::Sequence::from(vec![json]),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert_eq!(
            params.json_node_output_method,
            QNameOrString::QName(owned_name)
        );
    }

    #[test]
    fn test_qname_or_string_default_empty_sequence() {
        let map = Map::new(vec![(
            "json-node-output-method".to_string().into(),
            sequence::Sequence::default(),
        )])
        .unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert_eq!(
            params.json_node_output_method,
            QNameOrString::String("xml".to_string())
        );
    }

    #[test]
    fn test_qname_or_string_default_missing() {
        let map = Map::new(vec![]).unwrap();
        let static_context = context::StaticContext::default();
        let xot = Xot::new();
        let params = SerializationParameters::from_map(map, &static_context, &xot).unwrap();
        assert_eq!(
            params.json_node_output_method,
            QNameOrString::String("xml".to_string())
        );
    }
}
