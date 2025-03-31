use std::fs;
use std::path::Path;
use std::str::FromStr;

use anyhow::Error;
use fxhash::{FxHashMap, FxHashSet};

use crate::language::Language;
use crate::outcomes::{CatalogOutcomes, TestSetOutcomes};
use crate::testcase::TestCase;
use crate::testset::TestSet;

pub(crate) trait TestFilter<L: Language> {
    fn is_included(&self, test_set: &TestSet<L>, test_case: &TestCase<L>) -> bool;
}

pub(crate) struct IncludeAllFilter {}

impl IncludeAllFilter {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl<L: Language> TestFilter<L> for IncludeAllFilter {
    fn is_included(&self, _test_set: &TestSet<L>, _test_case: &TestCase<L>) -> bool {
        true
    }
}

pub(crate) struct NameFilter {
    name_filter: Option<String>,
}

impl NameFilter {
    pub(crate) fn new(name_filter: Option<String>) -> Self {
        Self { name_filter }
    }
}

impl<L: Language> TestFilter<L> for NameFilter {
    fn is_included(&self, _test_set: &TestSet<L>, test_case: &TestCase<L>) -> bool {
        if let Some(name_filter) = &self.name_filter {
            test_case.name.contains(name_filter)
        } else {
            true
        }
    }
}

pub(crate) struct ExcludedNamesFilter {
    names: FxHashMap<String, FxHashSet<String>>,
    comments: FxHashMap<String, FxHashMap<String, String>>,
}

impl<L: Language> TestFilter<L> for ExcludedNamesFilter {
    fn is_included(&self, test_set: &TestSet<L>, test_case: &TestCase<L>) -> bool {
        let test_set_name = &test_set.name;
        let test_case_name = &test_case.name;
        if let Some(excluded_names) = self.names.get(test_set_name) {
            !excluded_names.contains(test_case_name)
        } else {
            true
        }
    }
}

#[derive(Debug)]
pub(crate) enum UpdateResult {
    NoChange,
    Shrank,
    NotSubset,
}

// The format to load exclude names filters is
// = test_set_name
// excluded_test_case_name
// another # with a comment
impl ExcludedNamesFilter {
    pub(crate) fn new() -> Self {
        Self {
            names: FxHashMap::default(),
            comments: FxHashMap::default(),
        }
    }

    pub(crate) fn load_from_file(filter_path: &Path) -> Result<Self, Error> {
        if filter_path.exists() {
            // we have an existing test filter file, we read it
            // read file into string
            let filter_data = fs::read_to_string(filter_path)?;
            // parse string into filter
            filter_data.parse()
        } else {
            // we don't have a test filter file yet
            Ok(ExcludedNamesFilter::new())
        }
    }

    pub(crate) fn from_outcomes(catalog_outcomes: &CatalogOutcomes) -> Self {
        let mut filter = Self::new();
        filter.initialize_with_catalog_outcomes(catalog_outcomes);
        filter
    }

    pub(crate) fn initialize_with_test_set_outcomes(
        &mut self,
        test_set_outcomes: &TestSetOutcomes,
    ) {
        let failing_names: FxHashSet<String> =
            test_set_outcomes.failing_names().into_iter().collect();
        self.names
            .insert(test_set_outcomes.test_set_name.clone(), failing_names);
    }

    pub(crate) fn update_with_test_set_outcomes(
        &mut self,
        test_set_outcomes: &TestSetOutcomes,
    ) -> UpdateResult {
        let failing_names: FxHashSet<String> =
            test_set_outcomes.failing_names().into_iter().collect();
        // remove the previous entry
        let old_names = self.names.remove(&test_set_outcomes.test_set_name);
        // normalize to set
        let old_names = old_names.unwrap_or_default();

        if old_names.is_empty() {
            // we add back old names so we don't remove the whole
            // entry
            self.names
                .insert(test_set_outcomes.test_set_name.clone(), old_names);

            // we don't want to add any entries if there wasn't even
            // an entry for this test set name
            if failing_names.is_empty() {
                return UpdateResult::NoChange;
            } else {
                return UpdateResult::NotSubset;
            }
        }

        if !failing_names.is_subset(&old_names) {
            self.names
                .insert(test_set_outcomes.test_set_name.clone(), old_names);
            return UpdateResult::NotSubset;
        }
        self.names
            .insert(test_set_outcomes.test_set_name.clone(), failing_names);
        // there may be extra comments left for names that aren't relevant anymore,
        // but that's okay: we won't serialize them later

        // now we're done and shrank the amount of tests
        UpdateResult::Shrank
    }

    pub(crate) fn initialize_with_catalog_outcomes(&mut self, catalog_outcomes: &CatalogOutcomes) {
        for test_set_outcomes in catalog_outcomes.outcomes.iter() {
            self.initialize_with_test_set_outcomes(test_set_outcomes);
        }
    }

    pub(crate) fn update_with_catalog_outcomes(
        &mut self,
        catalog_outcomes: &CatalogOutcomes,
    ) -> Vec<UpdateResult> {
        let mut update_results = Vec::new();
        for test_set_outcomes in catalog_outcomes.outcomes.iter() {
            update_results.push(self.update_with_test_set_outcomes(test_set_outcomes));
        }
        update_results
    }
}

impl FromStr for ExcludedNamesFilter {
    type Err = Error;

    fn from_str(source: &str) -> Result<Self, Self::Err> {
        let mut names = FxHashMap::default();
        let mut comments = FxHashMap::default();
        let mut test_set_name: Option<String> = None;
        let mut test_case_names = FxHashSet::default();
        let mut test_case_comments = FxHashMap::default();
        for line in source.lines() {
            let line = line.trim();
            if line.starts_with('=') {
                if let Some(test_set_name) = test_set_name {
                    names.insert(test_set_name.clone(), test_case_names);
                    comments.insert(test_set_name, test_case_comments);
                }
                test_set_name = Some(line.strip_prefix('=').unwrap().trim().to_string());
                test_case_names = FxHashSet::default();
                test_case_comments = FxHashMap::default();
            } else if !line.is_empty() {
                let mut parts = line.split('#');
                let test_case_name = parts.next().unwrap();
                let test_case_name = test_case_name.trim();
                let comment = parts.next().map(|s| s.trim().to_string());
                test_case_names.insert(test_case_name.to_string());
                if let Some(comment) = comment {
                    test_case_comments.insert(test_case_name.to_string(), comment);
                }
            }
        }
        if let Some(test_set_name) = test_set_name {
            names.insert(test_set_name.clone(), test_case_names);
            comments.insert(test_set_name, test_case_comments);
        }
        Ok(Self { names, comments })
    }
}

impl std::fmt::Display for ExcludedNamesFilter {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        let mut sorted_names = self.names.keys().collect::<Vec<_>>();
        sorted_names.sort();
        for test_set_name in sorted_names {
            let excluded_names = self.names.get(test_set_name).unwrap();
            let comments = self.comments.get(test_set_name);
            fmt.write_str(&format!("= {}\n", test_set_name))?;
            let mut sorted_excluded_names = excluded_names.iter().collect::<Vec<_>>();
            sorted_excluded_names.sort();
            for excluded_name in sorted_excluded_names {
                fmt.write_str(excluded_name)?;
                if let Some(comments) = comments {
                    let comment = comments.get(excluded_name);
                    if let Some(comment) = comment {
                        let comment = comment.trim();
                        if !comment.is_empty() {
                            fmt.write_str(&format!(" # {}", comment))?;
                        }
                    }
                }
                fmt.write_str("\n")?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::testcase::TestOutcome;

    use super::*;

    #[test]
    fn test_parse() {
        let source = r#"
= test_set_1
test_case_1
test_case_2 # with a comment
= test_set_2
test_case_3
"#;
        let filter: ExcludedNamesFilter = source.parse().unwrap();
        assert_eq!(filter.names.len(), 2);
        assert!(filter.names.contains_key("test_set_1"));
        assert!(filter.names.contains_key("test_set_2"));
        assert!(filter
            .names
            .get("test_set_1")
            .unwrap()
            .contains("test_case_1"));
        assert!(filter
            .names
            .get("test_set_1")
            .unwrap()
            .contains("test_case_2"));
        assert!(filter
            .names
            .get("test_set_2")
            .unwrap()
            .contains("test_case_3"));
        assert_eq!(filter.comments.len(), 2);
        assert!(filter.comments.contains_key("test_set_1"));
        assert!(filter.comments.contains_key("test_set_2"));
        assert_eq!(
            filter
                .comments
                .get("test_set_1")
                .unwrap()
                .get("test_case_2")
                .unwrap(),
            "with a comment"
        );
    }

    #[test]
    fn test_parse_serialize() {
        let source = r#"
= test_set_1
test_case_1
test_case_2 # with a comment
= test_set_2
test_case_3
"#
        .trim_start();
        let filter: ExcludedNamesFilter = source.parse().unwrap();
        let serialized = filter.to_string();
        assert_eq!(source, serialized);
    }

    #[test]
    fn test_update_test_set() {
        let mut filter = ExcludedNamesFilter::new();

        let mut outcomes = TestSetOutcomes::new("test_set_1");
        // unsupported is the easiest non-passed outcome we can construct
        outcomes.add_outcome("test_case_1", TestOutcome::Unsupported);
        outcomes.add_outcome("test_case_2", TestOutcome::Unsupported);
        filter.initialize_with_test_set_outcomes(&outcomes);
        // now one test passes
        let mut outcomes = TestSetOutcomes::new("test_set_1");
        outcomes.add_outcome("test_case_1", TestOutcome::Passed);
        outcomes.add_outcome("test_case_2", TestOutcome::Unsupported);
        // and we do an update
        let r = filter.update_with_test_set_outcomes(&outcomes);
        assert!(matches!(r, UpdateResult::Shrank));

        let serialized = filter.to_string();
        let expected = r#"= test_set_1
test_case_2
"#;
        assert_eq!(serialized, expected.trim_start());
    }

    #[test]
    fn test_update_test_set_not_subset() {
        let mut filter = ExcludedNamesFilter::new();

        let mut outcomes = TestSetOutcomes::new("test_set_1");
        outcomes.add_outcome("test_case_1", TestOutcome::Passed);
        outcomes.add_outcome("test_case_2", TestOutcome::Unsupported);
        filter.initialize_with_test_set_outcomes(&outcomes);
        // now one test passes
        let mut outcomes = TestSetOutcomes::new("test_set_1");
        outcomes.add_outcome("test_case_1", TestOutcome::Unsupported);
        outcomes.add_outcome("test_case_2", TestOutcome::Unsupported);
        // and we do an update
        let r = filter.update_with_test_set_outcomes(&outcomes);
        assert!(matches!(r, UpdateResult::NotSubset));

        let serialized = filter.to_string();
        let expected = r#"= test_set_1
test_case_2
"#;
        assert_eq!(serialized, expected.trim_start());
    }
}
