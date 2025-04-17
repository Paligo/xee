use std::path::{Path, PathBuf};

use ahash::{HashMap, HashMapExt};

/// The source origin is either inline or in a file
///
/// Inline is handy for isolated XPath expressions, files are
/// handy for XSLT
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceOrigin {
    /// Inline source, e.g. a string
    Inline(String),
    /// Path to a file which contains the source
    Path(PathBuf),
}

impl From<PathBuf> for SourceOrigin {
    fn from(path: PathBuf) -> Self {
        SourceOrigin::Path(path)
    }
}

impl From<&Path> for SourceOrigin {
    fn from(path: &Path) -> Self {
        SourceOrigin::Path(path.to_path_buf())
    }
}

impl From<String> for SourceOrigin {
    fn from(string: String) -> Self {
        SourceOrigin::Inline(string)
    }
}

impl From<&str> for SourceOrigin {
    fn from(string: &str) -> Self {
        SourceOrigin::Inline(string.to_string())
    }
}

/// A source id is a cheap copyable identifier
/// TODO: we could make this an u16 probably
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceId(usize);

impl SourceId {
    pub(crate) fn index(&self) -> usize {
        self.0
    }
}

pub struct SourceIdCacheBuilder<'a> {
    source_ids: HashMap<&'a SourceOrigin, SourceId>,
    pub(crate) origins: Vec<&'a SourceOrigin>,
}

impl<'a> SourceIdCacheBuilder<'a> {
    pub fn new() -> Self {
        Self {
            source_ids: HashMap::new(),
            origins: Vec::new(),
        }
    }

    pub fn add_source_origin(&mut self, origin: &'a SourceOrigin) -> SourceId {
        if let Some(id) = self.source_ids.get(origin) {
            *id
        } else {
            let id = SourceId(self.origins.len());
            self.origins.push(origin);
            self.source_ids.insert(origin, id);
            id
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SourceSpan {
    pub(crate) source_id: SourceId,
    pub(crate) start: usize,
    pub(crate) end: usize,
}
