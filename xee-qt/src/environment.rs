use ahash::AHashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use xee_xpath::{
    context::DynamicContext, context::StaticContext, context::Variables, parse, sequence::Item,
    xml::Documents, xml::Node, xml::Uri, Name,
};
use xot::Xot;

use crate::collection::FxIndexMap;
use crate::error::{Error, Result};
use crate::qt;
use crate::qt::EnvironmentSpec;
use crate::qt::Source;

impl EnvironmentSpec {
    pub(crate) fn context_item(
        &self,
        xot: &mut Xot,
        documents: &mut Documents,
    ) -> Result<Option<Item>> {
        for source in &self.sources {
            if let qt::SourceRole::Context = source.role {
                let node = source.node(xot, &self.base_dir, documents)?;
                return Ok(Some(Item::from(node)));
            }
        }
        Ok(None)
    }

    pub(crate) fn variables(&self, mut xot: Xot, documents: &mut Documents) -> Result<Variables> {
        let mut variables = Variables::new();
        for source in &self.sources {
            if let qt::SourceRole::Var(name) = &source.role {
                let name = &name[1..]; // without $
                let node = source.node(&mut xot, &self.base_dir, documents)?;
                variables.insert(Name::unprefixed(name), Item::from(node).into());
            }
        }
        for param in &self.params {
            let static_context = StaticContext::default();
            let select = (param.select.as_ref()).expect("param: missing select not supported");
            let program = parse(&static_context, select);
            if program.is_err() {
                println!("param: select xpath parse failed: {}", select);
                continue;
            }
            let program = program.unwrap();
            let dynamic_context = DynamicContext::empty(xot, &static_context);
            let runnable = program.runnable(&dynamic_context);
            let result = runnable.many(None).map_err(|e| e.error)?;
            variables.insert(param.name.clone(), result);
        }
        Ok(variables)
    }

    pub(crate) fn namespace_pairs(&self) -> Vec<(&str, &str)> {
        self.namespaces
            .iter()
            .map(|ns| (ns.prefix.as_ref(), ns.uri.as_ref()))
            .collect()
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct SharedEnvironments {
    environments: FxIndexMap<String, EnvironmentSpec>,
}

impl SharedEnvironments {
    pub(crate) fn new(mut environments: FxIndexMap<String, EnvironmentSpec>) -> Self {
        // there is always an empty environment
        if !environments.contains_key("empty") {
            let empty = EnvironmentSpec::empty();
            environments.insert("empty".to_string(), empty);
        }
        Self { environments }
    }

    pub(crate) fn get(&self, environment_ref: &qt::EnvironmentRef) -> Option<&EnvironmentSpec> {
        self.environments.get(&environment_ref.ref_)
    }
}

pub(crate) struct EnvironmentSpecIterator<'a> {
    pub(crate) catalog_shared_environments: &'a SharedEnvironments,
    pub(crate) test_set_shared_environments: &'a SharedEnvironments,
    pub(crate) environments: &'a [qt::TestCaseEnvironment],
    pub(crate) index: usize,
}

impl<'a> Iterator for EnvironmentSpecIterator<'a> {
    type Item = Result<&'a qt::EnvironmentSpec>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.environments.len() {
            return None;
        }
        let environment = &self.environments[self.index];
        self.index += 1;
        match environment {
            qt::TestCaseEnvironment::Local(local_environment_spec) => {
                Some(Ok(local_environment_spec))
            }
            qt::TestCaseEnvironment::Ref(environment_ref) => {
                for shared_environments in [
                    self.test_set_shared_environments,
                    self.catalog_shared_environments,
                ] {
                    let environment_spec = shared_environments.get(environment_ref);
                    if let Some(environment_spec) = environment_spec {
                        return Some(Ok(environment_spec));
                    }
                }
                Some(Err(Error::UnknownEnvironmentReference(
                    environment_ref.clone(),
                )))
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SourceCache {
    nodes: AHashMap<PathBuf, Node>,
}

impl SourceCache {
    pub(crate) fn new() -> Self {
        Self {
            nodes: AHashMap::default(),
        }
    }

    pub(crate) fn cleanup(&self, xot: &mut Xot) {
        for node in self.nodes.values() {
            if let Node::Xot(root) = node {
                xot.remove(*root).unwrap();
            }
        }
    }
}

impl Source {
    pub(crate) fn node(
        &self,
        xot: &mut Xot,
        base_dir: &Path,
        documents: &mut Documents,
    ) -> Result<Node> {
        let full_path = base_dir.join(&self.file);
        // construct a Uri
        // TODO: this is not really a proper URI but
        // what matters is that it's unique here
        let uri = Uri::new(&full_path.to_string_lossy());

        // try to get the cached version of the document
        let document = documents.get(&uri);
        if let Some(document) = document {
            let root = document.root();
            return Ok(root);
        }

        // could not get cached version, so load up document
        let xml_file = File::open(&full_path)?;
        let mut buf_reader = BufReader::new(xml_file);
        let mut xml = String::new();
        buf_reader.read_to_string(&mut xml)?;

        documents.add(xot, &uri, &xml)?;
        // now obtain what we just added
        Ok(documents.get(&uri).unwrap().root())
    }
}
