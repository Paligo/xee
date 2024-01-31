use ahash::AHashMap;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use xot::Xot;

use xee_xpath_ast::ast;

use crate::error::Error;
use crate::sequence;
use crate::xml;

use super::static_context::StaticContext;

pub type Variables = AHashMap<ast::Name, sequence::Sequence>;

pub struct DynamicContext<'a> {
    xot: Xot,
    pub static_context: &'a StaticContext<'a>,
    pub documents: Cow<'a, xml::Documents>,
    pub(crate) variables: Cow<'a, Variables>,
    current_datetime: chrono::DateTime<chrono::offset::FixedOffset>,
}

impl<'a> Debug for DynamicContext<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Context")
            .field("static_context", &self.static_context)
            .field("documents", &self.documents)
            .finish()
    }
}

impl<'a> DynamicContext<'a> {
    pub fn new(
        xot: Xot,
        static_context: &'a StaticContext<'a>,
        documents: Cow<'a, xml::Documents>,
        variables: Cow<'a, Variables>,
    ) -> Self {
        Self {
            xot,
            static_context,
            documents,
            variables,
            current_datetime: Self::create_current_datetime(),
        }
    }

    pub fn empty(xot: Xot, static_context: &'a StaticContext<'a>) -> Self {
        let documents = xml::Documents::new();
        Self::new(
            xot,
            static_context,
            Cow::Owned(documents),
            Cow::Owned(Variables::default()),
        )
    }

    pub fn xot(&self) -> &Xot {
        &self.xot
    }

    pub fn from_documents(
        xot: Xot,
        static_context: &'a StaticContext<'a>,
        documents: &'a xml::Documents,
    ) -> Self {
        Self::new(
            xot,
            static_context,
            Cow::Borrowed(documents),
            Cow::Owned(Variables::default()),
        )
    }

    pub fn from_variables(
        xot: Xot,
        static_context: &'a StaticContext<'a>,
        variables: &'a Variables,
    ) -> Self {
        Self::new(
            xot,
            static_context,
            Cow::Owned(xml::Documents::new()),
            Cow::Borrowed(variables),
        )
    }

    fn create_current_datetime() -> chrono::DateTime<chrono::offset::FixedOffset> {
        chrono::offset::Local::now().into()
    }

    pub fn arguments(&self) -> Result<Vec<sequence::Sequence>, Error> {
        let mut arguments = Vec::new();
        for variable_name in &self.static_context.parser_context.variable_names {
            let items = self.variables.get(variable_name).ok_or(Error::XPDY0002)?;
            arguments.push(items.clone());
        }
        Ok(arguments)
    }

    pub(crate) fn current_datetime(&self) -> chrono::DateTime<chrono::offset::FixedOffset> {
        self.current_datetime
    }

    pub(crate) fn implicit_timezone(&self) -> chrono::FixedOffset {
        self.current_datetime.timezone()
    }

    pub fn documents(&self) -> &xml::Documents {
        &self.documents
    }
}
