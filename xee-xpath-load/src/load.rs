use anyhow::Result;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::Path,
};

use xee_xpath_compiler::sequence::Item;

use xee_xpath::{context::StaticContextBuilder, error::Result as XPathResult};
use xee_xpath::{DocumentHandle, Documents, Queries, Query};

pub fn convert_string(_: &mut Documents, item: &Item) -> XPathResult<String> {
    Ok(item.to_atomic()?.try_into()?)
}

pub fn convert_boolean(documents: &mut Documents, item: &Item) -> XPathResult<bool> {
    Ok(convert_string(documents, item)? == "true")
}

pub trait ContextLoadable<C: ?Sized>: Sized {
    fn static_context_builder<'namespaces>(context: &C) -> StaticContextBuilder<'namespaces>;

    fn load_with_context(queries: &Queries, context: &C) -> Result<impl Query<Self>>;

    fn load_from_xml_with_context(xml: &str, context: &C) -> Result<Self> {
        let mut documents = Documents::new();
        // TODO: default document URI is just hardcoded
        let document_id = documents.add_string("http://example.com".try_into().unwrap(), xml)?;

        Self::load_from_node_with_context(documents, document_id, context)
    }

    fn load_from_node_with_context(
        mut documents: Documents,
        document_id: DocumentHandle,
        context: &C,
    ) -> Result<Self> {
        let static_context_builder = Self::static_context_builder(context);
        let queries = Queries::new(static_context_builder);

        let query = Self::load_with_context(&queries, context)?;
        let root = documents.document_node(document_id).unwrap();
        // the test runner needs to work from the document element
        let document_element = documents.xot().document_element(root).unwrap();
        Ok(query.execute(&mut documents, document_element)?)
    }
}

pub trait Loadable: Sized {
    fn static_context_builder<'namespaces>() -> StaticContextBuilder<'namespaces>;

    fn load(queries: &Queries) -> Result<impl Query<Self>>;

    fn load_from_xml(xml: &str) -> Result<Self> {
        Self::load_from_xml_with_context(xml, &())
    }

    fn load_from_node(documents: Documents, document_id: DocumentHandle) -> Result<Self> {
        Self::load_from_node_with_context(documents, document_id, &())
    }
}

impl<T: Loadable> ContextLoadable<()> for T {
    fn static_context_builder<'namespaces>(_context: &()) -> StaticContextBuilder<'namespaces> {
        T::static_context_builder()
    }

    fn load_with_context(queries: &Queries, _context: &()) -> Result<impl Query<Self>> {
        Self::load(queries)
    }
}

pub trait PathLoadable: ContextLoadable<Path> {
    fn load_from_file(path: &Path) -> Result<Self> {
        let xml_file = File::open(path)?;
        let mut buf_reader = BufReader::new(xml_file);
        let mut xml = String::new();
        buf_reader.read_to_string(&mut xml)?;
        Self::load_from_xml_with_context(&xml, path)
    }
}

impl<T: ContextLoadable<Path>> PathLoadable for T {}
