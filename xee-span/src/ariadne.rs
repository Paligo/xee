use std::{borrow::Cow, collections::hash_map::Entry, fmt, fs, io};

use ahash::{HashMap, HashMapExt};

use crate::{span::SourceIdCacheBuilder, SourceId, SourceOrigin, SourceSpan};

impl ariadne::Span for SourceSpan {
    type SourceId = SourceId;

    fn source(&self) -> &Self::SourceId {
        &self.source_id
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

/// We keep a cache around
///
/// This cache does two things: map source origin to source id
/// and map source id to ariadne source
pub struct SourceIdCache {
    origins: Vec<SourceOrigin>,
    sources: HashMap<SourceId, ariadne::Source>,
}

impl SourceIdCache {
    pub fn new(builder: SourceIdCacheBuilder) -> Self {
        Self {
            origins: builder.origins.into_iter().cloned().collect(),
            sources: HashMap::new(),
        }
    }

    // This cannot take self as self as already get a mutable reference of sources later
    fn source<'a>(origins: &'a [SourceOrigin], id: &SourceId) -> Result<Cow<'a, str>, io::Error> {
        let origin = &origins[id.index()];
        match origin {
            SourceOrigin::Inline(s) => Ok(Cow::Borrowed(s)),
            SourceOrigin::Path(path) => Ok(Cow::Owned(fs::read_to_string(path)?)),
        }
    }
}

impl ariadne::Cache<SourceId> for SourceIdCache {
    type Storage = String;

    fn fetch(&mut self, id: &SourceId) -> Result<&ariadne::Source, impl fmt::Debug> {
        let origins = &self.origins;

        Ok::<_, io::Error>(match self.sources.entry(*id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let source = Self::source(origins, id)?;
                entry.insert(ariadne::Source::from(source.into_owned()))
            }
        })
    }

    fn display<'a>(&self, id: &'a SourceId) -> Option<impl fmt::Display + 'a> {
        let origin = &self.origins[id.index()];
        match origin {
            SourceOrigin::Path(path) => Some(path.to_string_lossy().into_owned()),
            SourceOrigin::Inline(_) => None,
        }
    }
}
