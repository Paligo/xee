use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

#[derive(Debug)]
pub(crate) enum Mode {
    XPath,
    Xslt,
}

#[derive(Debug)]
pub(crate) struct PathInfo {
    pub(crate) catalog_path: PathBuf,
    pub(crate) filter_path: PathBuf,
    pub(crate) relative_path: PathBuf,
    pub(crate) mode: Mode,
}

pub(crate) fn paths(path: &Path) -> Result<PathInfo> {
    // look for a directory which contains a `catalog.xml`. This
    // is the first path buf. any remaining path components are
    // a relative path
    for ancestor in path.ancestors() {
        let catalog = ancestor.join("catalog.xml");
        if catalog.exists() {
            let catalog = std::fs::canonicalize(catalog).unwrap();
            let relative = path.strip_prefix(ancestor).unwrap();
            // filter file sits next to catalog.xml
            let filter_path = ancestor.join("filters");
            // hacky way to determine mode from path
            let mode = if catalog.to_str().unwrap().contains("xslt") {
                Mode::Xslt
            } else {
                Mode::XPath
            };
            let path_info = PathInfo {
                catalog_path: catalog,
                filter_path: filter_path.to_path_buf(),
                relative_path: relative.to_path_buf(),
                mode,
            };
            return Ok(path_info);
        }
    }
    Err(Error::NoCatalogFound)
}

impl PathInfo {
    pub(crate) fn whole_catalog(&self) -> bool {
        self.relative_path.components().count() == 0
    }

    pub(crate) fn test_file(&self) -> PathBuf {
        // take base of catalog path and join relative path
        self.catalog_path
            .parent()
            .unwrap()
            .join(&self.relative_path)
    }
}
