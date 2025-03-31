// option parameter conventions
// <https://www.w3.org/TR/xpath-functions-31/#option-parameter-conventions>

use xee_schema_type::Xs;
use xee_xpath_ast::ast;
use xot::xmlname::{NameStrInfo, OwnedName};
use xot::Xot;

use crate::{atomic, context, error, function::Map};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QNameOrString {
    QName(OwnedName),
    String(String),
}

impl QNameOrString {
    // a convenience accessor so we can get access to the localname and/or
    // plain string, helps matching method later
    pub(crate) fn local_name(&self) -> Option<&str> {
        match self {
            Self::QName(name) => {
                if name.namespace().is_empty() {
                    Some(name.local_name())
                } else {
                    None
                }
            }
            Self::String(string) => Some(string),
        }
    }
}

pub(crate) struct OptionParameterConverter<'a> {
    map: &'a Map,
    static_context: &'a context::StaticContext,
    xot: &'a Xot,
}

impl<'a> OptionParameterConverter<'a> {
    pub(crate) fn new(
        map: &'a Map,
        static_context: &'a context::StaticContext,
        xot: &'a Xot,
    ) -> Self {
        Self {
            map,
            static_context,
            xot,
        }
    }

    pub(crate) fn option<V>(&self, name: &str, atomic_type: Xs) -> error::Result<Option<V>>
    where
        V: std::convert::TryFrom<atomic::Atomic, Error = error::Error>,
    {
        let name: atomic::Atomic = name.to_string().into();
        let value = self.map.get_as_type(
            &name,
            ast::Occurrence::Option,
            atomic_type,
            self.static_context,
            self.xot,
        )?;
        let value = if let Some(value) = value {
            value.option()?
        } else {
            return Ok(None);
        };
        let value: Option<V> = if let Some(value) = value {
            Some(value.to_atomic()?.try_into()?)
        } else {
            None
        };
        Ok(value)
    }

    pub(crate) fn option_with_default<V>(
        &self,
        name: &str,
        atomic_type: Xs,
        default: V,
    ) -> error::Result<V>
    where
        V: std::convert::TryFrom<atomic::Atomic, Error = error::Error>,
    {
        Ok(if let Some(value) = self.option(name, atomic_type)? {
            value
        } else {
            default
        })
    }

    pub(crate) fn many<V>(&self, name: &str, atomic_type: Xs) -> error::Result<Vec<V>>
    where
        V: std::convert::TryFrom<atomic::Atomic, Error = error::Error>,
    {
        let name: atomic::Atomic = name.to_string().into();
        let value = self.map.get_as_type(
            &name,
            ast::Occurrence::Many,
            atomic_type,
            self.static_context,
            self.xot,
        )?;
        let values = if let Some(value) = value {
            value
                .iter()
                .map(|item| item.to_atomic()?.try_into())
                .collect::<Result<Vec<V>, _>>()?
        } else {
            Vec::new()
        };
        Ok(values)
    }

    pub(crate) fn qname_or_string(
        &self,
        name: &str,
        default: QNameOrString,
    ) -> error::Result<QNameOrString> {
        let qname_value = self.option(name, Xs::QName);
        let string_value = self.option(name, Xs::String);
        match (qname_value, string_value) {
            (Err(_), Ok(Some(string_value))) => Ok(QNameOrString::String(string_value)),
            (Ok(Some(qname_value)), Err(_)) => Ok(QNameOrString::QName(qname_value)),
            (Ok(None), Ok(None)) => Ok(default),
            (Err(e), Err(_)) => Err(e),
            (Err(e), Ok(None)) => Err(e),
            (Ok(None), Err(e)) => Err(e),
            _ => unreachable!(),
        }
    }
}
