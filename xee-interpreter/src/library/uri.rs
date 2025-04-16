use iri_string::types::{IriAbsoluteStr, IriReferenceStr};

use xee_xpath_macros::xpath_fn;

use crate::{atomic, function::StaticFunctionDescription, wrap_xpath_fn};
use crate::{context, error};

const IRI: percent_encoding::AsciiSet = percent_encoding::CONTROLS
    .add(b'<')
    .add(b'>')
    .add(b'"')
    .add(b' ')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'\\')
    .add(b'^')
    .add(b'`');

// the only things not encoded are the upper and lower case letters, the
// digits, '-', '_', '.' and '~'.
const ENCODE_FOR_URI: percent_encoding::AsciiSet = percent_encoding::CONTROLS
    .add(b' ')
    .add(b'!')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'\'')
    .add(b'(')
    .add(b')')
    .add(b'*')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}');

#[xpath_fn("fn:resolve-uri($relative as xs:string?) as xs:anyURI?")]
fn resolve_uri1(
    context: &context::DynamicContext,
    relative: Option<&str>,
) -> error::Result<Option<atomic::Atomic>> {
    let base = context.static_context().static_base_uri();
    if let Some(base) = base {
        Ok(resolve_uri(relative, base.as_str())?
            .map(|resolved| atomic::Atomic::String(atomic::StringType::AnyURI, resolved.into())))
    } else {
        Err(error::Error::new(error::ErrorCode::FONS0005))
    }
}

#[xpath_fn("fn:resolve-uri($relative as xs:string?, $base as xs:string) as xs:anyURI?")]
fn resolve_uri2(relative: Option<&str>, base: &str) -> error::Result<Option<atomic::Atomic>> {
    Ok(resolve_uri(relative, base)?
        .map(|resolved| atomic::Atomic::String(atomic::StringType::AnyURI, resolved.into())))
}

pub(crate) fn resolve_uri(relative: Option<&str>, base: &str) -> error::Result<Option<String>> {
    if let Some(relative) = relative {
        let iri_reference: &IriReferenceStr = relative
            .try_into()
            .map_err(|_e| error::Error::new(error::ErrorCode::FORG0002))?;
        // a shortcut here: if iri_reference is an absolute IRI, we can
        // just return it.
        let relative_iri = match iri_reference.to_iri() {
            Ok(iri) => {
                return Ok(Some(iri.to_string()));
            }
            Err(iri) => {
                // iri is relative, so continue with that
                iri
            }
        };

        // note that this means base isn't validated if it's not needed
        let base: &IriAbsoluteStr = base
            .try_into()
            .map_err(|_| error::Error::new(error::ErrorCode::FORG0002))?;
        // now resolve the iri against base
        let resolved_iri = relative_iri.resolve_against(base);

        Ok(Some(resolved_iri.to_string()))
    } else {
        Ok(None)
    }
}

#[xpath_fn("fn:encode-for-uri($uripart as xs:string?) as xs:string")]
fn encode_for_uri(uripart: Option<&str>) -> String {
    if let Some(uri_part) = uripart {
        percent_encoding::utf8_percent_encode(uri_part, &ENCODE_FOR_URI).to_string()
    } else {
        "".to_string()
    }
}

#[xpath_fn("fn:iri-to-uri($iri as xs:string?) as xs:string")]
fn iri_to_uri(iri: Option<&str>) -> String {
    if let Some(iri) = iri {
        percent_encoding::utf8_percent_encode(iri, &IRI).to_string()
    } else {
        "".to_string()
    }
}

#[xpath_fn("fn:escape-html-uri($uri as xs:string?) as xs:string")]
fn escape_html_uri(uri: Option<&str>) -> String {
    if let Some(uri) = uri {
        percent_encoding::utf8_percent_encode(uri, percent_encoding::CONTROLS).to_string()
    } else {
        "".to_string()
    }
}

pub(crate) fn static_function_descriptions() -> Vec<StaticFunctionDescription> {
    vec![
        wrap_xpath_fn!(resolve_uri1),
        wrap_xpath_fn!(resolve_uri2),
        wrap_xpath_fn!(encode_for_uri),
        wrap_xpath_fn!(iri_to_uri),
        wrap_xpath_fn!(escape_html_uri),
    ]
}
