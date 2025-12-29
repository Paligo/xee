// XSLT has template rules that match based on patterns, a subset of XPath.
// This module contains the runtime to match items with patterns.

mod mode;
mod pattern_core;
mod pattern_lookup;

pub use mode::{ModeId, RuleEntry};
pub(crate) use pattern_core::PredicateMatcher;
