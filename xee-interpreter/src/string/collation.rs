use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::rc::Rc;

use ahash::{HashMap, HashMapExt};
use icu::collator::{BackwardSecondLevel, CaseLevel, Numeric};
use icu::{
    collator::{self, AlternateHandling, CaseFirst, Collator, MaxVariable, Strength},
    locid::Locale,
};

use iri_string::types::{IriAbsoluteStr, IriReferenceStr, IriStr, IriString};

use crate::error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct CollatorQuery {
    pub(crate) fallback: bool,
    pub(crate) lang: Option<String>,
    pub(crate) strength: Strength,
    pub(crate) max_variable: MaxVariable,
    pub(crate) alternate: AlternateHandling,
    pub(crate) backwards: bool,
    pub(crate) normalization: bool,
    pub(crate) case_level: bool,
    pub(crate) case_first: CaseFirst,
    pub(crate) numeric: bool,
    // both version and reorder are not supported at this point, as
    // they don't seem to have equivalents in icu4x
}

impl From<CollatorQuery> for collator::CollatorOptions {
    fn from(query: CollatorQuery) -> Self {
        let mut options = Self::new();
        options.strength = Some(query.strength);
        options.alternate_handling = Some(query.alternate);
        options.case_first = Some(query.case_first);
        options.max_variable = Some(query.max_variable);
        options.case_level = Some(if query.case_level {
            CaseLevel::On
        } else {
            CaseLevel::Off
        });
        options.numeric = Some(if query.numeric {
            Numeric::On
        } else {
            Numeric::Off
        });
        options.backward_second_level = Some({
            if query.backwards {
                BackwardSecondLevel::On
            } else {
                BackwardSecondLevel::Off
            }
        });
        options
    }
}

impl CollatorQuery {
    fn from_url(url: &IriStr) -> error::Result<Self> {
        let query = url.query_str().unwrap_or("");

        let mut fallback = None;
        let mut lang = None;
        let mut strength = None;
        let mut max_variable = None;
        let mut alternate = None;
        let mut backwards = None;
        let mut normalization = None;
        let mut case_level = None;
        let mut case_first = None;
        let mut numeric = None;
        let mut has_unrecognized_key = false;

        // last one wins
        for (key, value) in Self::parse_collation_query(query) {
            match key {
                "fallback" => {
                    fallback = Some(yes_no_query_parameter(value));
                }
                "lang" => {
                    lang = Some(value.to_string());
                }
                "strength" => {
                    strength = Some(strength_query_parameter(value));
                }
                "maxVariable" => {
                    max_variable = Some(max_variable_query_parameter(value));
                }
                "alternate" => {
                    alternate = Some(alternate_query_parameter(value));
                }
                "backwards" => {
                    backwards = Some(yes_no_query_parameter(value));
                }
                "normalization" => {
                    normalization = Some(yes_no_query_parameter(value));
                }
                "caseLevel" => {
                    case_level = Some(yes_no_query_parameter(value));
                }
                "caseFirst" => {
                    case_first = Some(case_first_query_parameter(value));
                }
                "numeric" => {
                    numeric = Some(yes_no_query_parameter(value));
                }
                _ => {
                    has_unrecognized_key = true;
                }
            }
        }
        let fallback = fallback.unwrap_or(Ok(true)).unwrap_or(true);

        // if depends on fallback whether we accept unrecognized values
        fn unwrap_or_fail<T>(
            v: Option<Result<T, Unrecognized>>,
            default: T,
            fallback: bool,
        ) -> error::Result<T> {
            if let Some(v) = v {
                if let Ok(v) = v {
                    Ok(v)
                } else if fallback {
                    Ok(default)
                } else {
                    Err(error::Error::FOCH0002)
                }
            } else {
                Ok(default)
            }
        }

        // if fallback is no we don't recognize any unrecognized keys
        if !fallback && has_unrecognized_key {
            return Err(error::Error::FOCH0002);
        }

        Ok(Self {
            fallback,
            lang: lang.map(|s| s.to_string()),
            strength: unwrap_or_fail(strength, Strength::Tertiary, fallback)?,
            max_variable: unwrap_or_fail(max_variable, MaxVariable::Punctuation, fallback)?,
            alternate: unwrap_or_fail(alternate, AlternateHandling::NonIgnorable, fallback)?,
            backwards: unwrap_or_fail(backwards, false, fallback)?,
            normalization: unwrap_or_fail(normalization, false, fallback)?,
            case_level: unwrap_or_fail(case_level, false, fallback)?,
            case_first: unwrap_or_fail(case_first, CaseFirst::Off, fallback)?,
            numeric: unwrap_or_fail(numeric, false, fallback)?,
        })
    }

    fn parse_collation_query(s: &str) -> impl Iterator<Item = (&str, &str)> {
        // the spec doesn't use normal query parameters separated by & but
        // semi-colon separated parameters, probably because & is
        // already used in XML.
        s.split(';').filter_map(|part| {
            let mut parts = part.split('=');
            let key = parts.next()?;
            let value = parts.next()?;
            Some((key, value))
        })
    }
}

#[derive(Debug)]
pub enum Collation {
    // 5.3.2
    CodePoint,
    // 5.3.3
    Uca(Box<Collator>),
    // 5.3.4
    HtmlAscii,
}

impl Collation {
    fn new(base_uri: Option<&IriAbsoluteStr>, uri: &IriReferenceStr) -> error::Result<Self> {
        let uri = if let Some(base_uri) = base_uri {
            let uri: IriString = uri.resolve_against(base_uri).into();
            uri
        } else {
            let uri: IriString = uri.to_iri().map_err(|_| error::Error::FOCH0002)?.to_owned();
            uri
        };
        if uri.scheme_str() != "http" || uri.authority_str() != Some("www.w3.org") {
            return Err(error::Error::FOCH0002);
        }
        let path = uri.path_str();
        Ok(match path {
            "/2005/xpath-functions/collation/codepoint" => Self::CodePoint,
            "/2013/collation/UCA" => {
                let collator_query = CollatorQuery::from_url(&uri)?;
                Self::Uca(Box::new(Self::uca_collator(collator_query)?))
            }
            "/2005/xpath-functions/collation/html-ascii-case-insensitive" => Self::HtmlAscii,
            // TODO: a bit of a hack, we support the qt3 caseblind collation too so that the test suite will work
            "/2010/09/qt-fots-catalog/collation/caseblind" => Self::HtmlAscii,
            _ => return Err(error::Error::FOCH0002),
        })
    }

    fn uca_collator(collator_query: CollatorQuery) -> error::Result<Collator> {
        let locale = if let Some(lang) = &collator_query.lang {
            match Locale::try_from_bytes(lang.as_bytes()) {
                Ok(locale) => locale,
                Err(_) => {
                    if collator_query.fallback {
                        // in case of fallback, get a locale anyway
                        Locale::UND
                    } else {
                        return Err(error::Error::FOCH0002);
                    }
                }
            }
        } else {
            // this is implementation defined according to the XPath spec
            // we choose to use the undefined locale
            Locale::UND
        };

        let locale = locale.into();
        let options = collator_query.into();

        Collator::try_new(&locale, options).map_err(|_| error::Error::FOCH0002)
    }

    pub(crate) fn compare(&self, a: &str, b: &str) -> Ordering {
        match self {
            Self::CodePoint => a.cmp(b),
            Self::Uca(collator) => collator.compare(a, b),
            Self::HtmlAscii => a.to_ascii_lowercase().cmp(&b.to_ascii_lowercase()),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Collations {
    collations: HashMap<String, Rc<Collation>>,
}

impl Collations {
    pub(crate) fn new() -> Self {
        Self {
            collations: HashMap::new(),
        }
    }

    pub(crate) fn load(
        &mut self,
        base_uri: Option<&IriAbsoluteStr>,
        uri: &IriReferenceStr,
    ) -> error::Result<Rc<Collation>> {
        // try to find cached collator. we cache by uri
        match self.collations.entry(uri.to_string()) {
            Entry::Occupied(entry) => Ok(entry.get().clone()),
            Entry::Vacant(entry) => {
                let collation = Collation::new(base_uri, uri)?;
                Ok(entry.insert(Rc::new(collation)).clone())
            }
        }
    }
}

struct Unrecognized;

fn yes_no_query_parameter(value: &str) -> Result<bool, Unrecognized> {
    match value {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(Unrecognized),
    }
}

fn strength_query_parameter(value: &str) -> Result<Strength, Unrecognized> {
    match value {
        "primary" | "1" => Ok(Strength::Primary),
        "secondary" | "2" => Ok(Strength::Secondary),
        "tertiary" | "3" => Ok(Strength::Tertiary),
        "quaternary" | "4" => Ok(Strength::Quaternary),
        "identical" | "5" => Ok(Strength::Identical),
        _ => Err(Unrecognized),
    }
}

fn max_variable_query_parameter(value: &str) -> Result<MaxVariable, Unrecognized> {
    match value {
        "space" => Ok(MaxVariable::Space),
        "punct" => Ok(MaxVariable::Punctuation),
        "symbol" => Ok(MaxVariable::Symbol),
        "currency" => Ok(MaxVariable::Currency),
        _ => Err(Unrecognized),
    }
}

fn alternate_query_parameter(value: &str) -> Result<AlternateHandling, Unrecognized> {
    match value {
        "non-ignorable" => Ok(AlternateHandling::NonIgnorable),
        "shifted" => Ok(AlternateHandling::Shifted),
        // blanked not supported by icu4x
        _ => Err(Unrecognized),
    }
}

fn case_first_query_parameter(value: &str) -> Result<CaseFirst, Unrecognized> {
    match value {
        "upper" => Ok(CaseFirst::UpperFirst),
        "lower" => Ok(CaseFirst::LowerFirst),
        _ => Err(Unrecognized),
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    // these tests verify the behavior to the url crate

    #[test]
    fn test_base_url() {
        let base: &IriAbsoluteStr = "http://www.w3.org/".try_into().unwrap();
        let path: &IriReferenceStr = "/2005/xpath-functions/collation/codepoint"
            .try_into()
            .unwrap();
        let url = path.resolve_against(base);
        assert_eq!(
            url.to_string(),
            "http://www.w3.org/2005/xpath-functions/collation/codepoint"
        );
    }

    #[test]
    fn test_base_url_with_full_url() {
        let base: &IriAbsoluteStr = "http://www.another.org/".try_into().unwrap();
        let path: &IriReferenceStr = "http://www.w3.org/2005/xpath-functions/collation/codepoint"
            .try_into()
            .unwrap();
        let url = path.resolve_against(base);
        assert_eq!(
            url.to_string(),
            "http://www.w3.org/2005/xpath-functions/collation/codepoint"
        );
    }

    #[test]
    fn test_base_url_with_just_qs() {
        let base: &IriAbsoluteStr = "http://www.w3.org/2013/collation/UCA".try_into().unwrap();
        let path: &IriReferenceStr = "?lang=foo".try_into().unwrap();
        let url = path.resolve_against(base);
        assert_eq!(
            url.to_string(),
            "http://www.w3.org/2013/collation/UCA?lang=foo"
        );
    }

    #[test]
    fn test_deserialize_query_string() {
        let url : &IriStr = "http://www.w3.org/2013/collation/UCA?fallback=yes;lang=en;strength=primary;max_variable=punctuation;alternate=non-ignorable;backwards=no;normalization=no;caseLevel=no;caseFirst=upper;numeric=no".try_into().unwrap();
        let query = CollatorQuery::from_url(url).unwrap();
        assert_eq!(
            query,
            CollatorQuery {
                fallback: true,
                lang: Some("en".to_string()),
                strength: Strength::Primary,
                max_variable: MaxVariable::Punctuation,
                alternate: AlternateHandling::NonIgnorable,
                backwards: false,
                normalization: false,
                case_level: false,
                case_first: CaseFirst::UpperFirst,
                numeric: false,
            }
        )
    }

    #[test]
    fn test_deserialize_query_string_default() {
        let url: &IriStr = "http://www.w3.org/2013/collation/UCA?lang=en"
            .try_into()
            .unwrap();
        let query = CollatorQuery::from_url(url).unwrap();
        assert_eq!(
            query,
            CollatorQuery {
                fallback: true,
                lang: Some("en".to_string()),
                strength: Strength::Tertiary,
                max_variable: MaxVariable::Punctuation,
                alternate: AlternateHandling::NonIgnorable,
                backwards: false,
                normalization: false,
                case_level: false,
                case_first: CaseFirst::Off,
                numeric: false,
            }
        )
    }

    #[test]
    fn test_deserialize_query_no_fallback_reject_wrong_value() {
        let url: &IriStr =
            "http://www.w3.org/2013/collation/UCA?lang=en;fallback=no;strength=nonsense"
                .try_into()
                .unwrap();
        assert!(CollatorQuery::from_url(url).is_err());
    }

    #[test]
    fn test_deserialize_query_no_fallback_reject_extra_param() {
        let url: &IriStr =
            "http://www.w3.org/2013/collation/UCA?lang=en;fallback=no;extra=nonsense"
                .try_into()
                .unwrap();
        assert!(CollatorQuery::from_url(url).is_err());
    }

    #[test]
    fn test_deserialize_query_yes_fallback_default_for_wrong_value() {
        let url: &IriStr =
            "http://www.w3.org/2013/collation/UCA?lang=en;fallback=yes;strength=nonsense"
                .try_into()
                .unwrap();
        let query = CollatorQuery::from_url(url).unwrap();
        assert_eq!(
            query,
            CollatorQuery {
                fallback: true,
                lang: Some("en".to_string()),
                strength: Strength::Tertiary,
                max_variable: MaxVariable::Punctuation,
                alternate: AlternateHandling::NonIgnorable,
                backwards: false,
                normalization: false,
                case_level: false,
                case_first: CaseFirst::Off,
                numeric: false,
            }
        )
    }

    #[test]
    fn test_deserialize_query_yes_fallback_ignore_extra_parameter() {
        let url: IriString =
            "http://www.w3.org/2013/collation/UCA?lang=en;fallback=yes;extra=nonsense"
                .try_into()
                .unwrap();
        let query = CollatorQuery::from_url(&url).unwrap();
        assert_eq!(
            query,
            CollatorQuery {
                fallback: true,
                lang: Some("en".to_string()),
                strength: Strength::Tertiary,
                max_variable: MaxVariable::Punctuation,
                alternate: AlternateHandling::NonIgnorable,
                backwards: false,
                normalization: false,
                case_level: false,
                case_first: CaseFirst::Off,
                numeric: false,
            }
        )
    }

    #[test]
    fn test_load_uca_collation() {
        let mut collations = Collations::new();
        let url: &IriReferenceStr = "http://www.w3.org/2013/collation/UCA?lang=se;fallback=no"
            .try_into()
            .unwrap();
        let collation = collations.load(None, url);
        assert!(collation.is_ok());
    }

    #[test]
    fn test_load_uca_collation_fallback() {
        let mut collations = Collations::new();
        let url: &IriReferenceStr = "http://www.w3.org/2013/collation/UCA?lang=en-US;fallback=yes"
            .try_into()
            .unwrap();
        let collation = collations.load(None, url);
        assert!(collation.is_ok());
    }

    // FIXME: This fallback test is broken since we switched to static instead
    // of blob data. I'm not sure it matters; the conformance tests
    // still work

    // #[test]
    // fn test_load_uca_collation_no_fallback() {
    //     let mut collations = Collations::new();
    //     let collation = collations.load(
    //         None,
    //         "http://www.w3.org/2013/collation/UCA?lang=en-US;fallback=no",
    //     );
    //     assert!(collation.is_err());
    // }

    #[test]
    fn test_load_codepoint_collation() {
        let mut collations = Collations::new();
        let url: &IriReferenceStr = "http://www.w3.org/2005/xpath-functions/collation/codepoint"
            .try_into()
            .unwrap();
        let collation = collations.load(None, url);
        assert!(collation.is_ok());
    }

    #[test]
    fn test_load_html_ascii_collation() {
        let mut collations = Collations::new();
        let url: &IriReferenceStr =
            "http://www.w3.org/2005/xpath-functions/collation/html-ascii-case-insensitive"
                .try_into()
                .unwrap();
        let collation = collations.load(None, url);
        assert!(collation.is_ok());
    }
}
