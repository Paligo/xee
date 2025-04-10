use std::{
    fs::File,
    path::PathBuf,
};

use clap::Parser;
use xot::{
    output::{
        xml::{Declaration, DocType, Parameters},
        Indentation,
    },
    NameId,
};
use xot::{xmlname::OwnedName, Xot};
use crate::common::input_xml;
use crate::error::render_parse_error;

static URI_QUALIFIED_NAME_REGEX: std::sync::LazyLock<regex::Regex> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"^Q\{(?P<ns>.*)\}(?P<name>.*)$").unwrap());

#[derive(Debug, Parser)]
pub(crate) struct Format {
    /// input xml file (default stdin)
    pub(crate) infile: Option<PathBuf>,
    /// output xml file (default stdout)
    pub(crate) outfile: Option<PathBuf>,
    /// Indent the output
    #[arg(long)]
    pub(crate) indent: bool,
    /// Element name to exclude from indentation (can be repeated)
    /// To specify a namespaced element, use Q{namespace}name
    /// If --indent is not specified, using this option is an error.
    #[arg(long)]
    pub(crate) suppress_indent: Vec<String>,
    /// Element name to output as a CDATA section (can be repeated)
    /// To specify a namespaced element, use Q{namespace}name
    #[arg(long)]
    pub(crate) cdata_element: Vec<String>,
    /// doctype public identifier.
    /// A system identifier has to be specified as well, otherwise this is an
    /// error.
    #[arg(long)]
    pub(crate) doctype_public: Option<String>,
    /// doctype system identifier.
    /// Can be used by itself or with --doctype-public.
    #[arg(long)]
    pub(crate) doctype_system: Option<String>,
    /// Output the XML declaration (without encoding).
    #[arg(long)]
    pub(crate) declaration: bool,
    /// Encoding for the XML declaration
    /// If not specified, the encoding is UTF-8.
    /// Implies --declaration.
    #[arg(long)]
    pub(crate) declaration_encoding: Option<String>,
    /// Standalone declaration for the XML declaration
    /// If not specified, the standalone declaration is omitted.
    /// Implies --declaration. Can be used in combination with
    /// --declaration-encoding.
    #[arg(long)]
    pub(crate) declaration_standalone: Option<bool>,
    /// Escape gt (>) characters in text content. By default this is false.
    #[arg(long)]
    pub(crate) escape_gt: bool,
}

impl Format {
    // TODO: much better error handling
    // - file not found
    // - invalid XML (should show line number, context)
    // - invalid XML name given as option
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        // open outfile from path unless it's not given, in which case
        // we want to use stdout
        let mut writer: Box<dyn std::io::Write> = if let Some(outfile) = &self.outfile {
            Box::new(File::create(outfile)?)
        } else {
            Box::new(std::io::stdout())
        };

        let mut xot = Xot::new();

        let indentation = if self.indent {
            let suppress = name_ids(&self.suppress_indent, &mut xot);
            if !suppress.is_empty() {
                Some(Indentation { suppress })
            } else {
                Some(Indentation::default())
            }
        } else {
            if !self.suppress_indent.is_empty() {
                return Err(anyhow::anyhow!(
                    "Cannot use --suppress-indent without --indent"
                ));
            }
            None
        };

        let cdata_section_elements = name_ids(&self.cdata_element, &mut xot);

        let doctype_public = self.doctype_public.as_deref();
        let doctype_system = self.doctype_system.as_deref();

        let doctype = match (doctype_public, doctype_system) {
            (Some(public), Some(system)) => Some(DocType::Public {
                public: public.to_string(),
                system: system.to_string(),
            }),
            (None, Some(system)) => Some(DocType::System {
                system: system.to_string(),
            }),
            (Some(_public), None) => {
                return Err(anyhow::anyhow!(
                    "Cannot use --doctype-public without --doctype-system"
                ));
            }
            (None, None) => None,
        };

        let has_declaration = self.declaration
            || self.declaration_encoding.is_some()
            || self.declaration_standalone.is_some();

        let declaration = if has_declaration {
            let encoding = self.declaration_encoding.as_deref();
            let standalone = self.declaration_standalone;
            Some(Declaration {
                encoding: encoding.map(|s| s.to_string()),
                standalone,
            })
        } else {
            None
        };

        let unescaped_gt = !self.escape_gt;

        let parameters = Parameters {
            indentation,
            cdata_section_elements,
            doctype,
            declaration,
            unescaped_gt,
        };

        let input_xml = input_xml(&self.infile)?;

        let root = match xot.parse(&input_xml) {
            Ok(root) => root,
            Err(e) => {
                render_parse_error(&input_xml, e);
                return Ok(());
            }
        };

        xot.serialize_xml_write(parameters, root, &mut writer)?;

        Ok(())
    }
}

// TODO: what if the name is not a valid XML name?
fn name_ids(names: &[String], xot: &mut Xot) -> Vec<NameId> {
    let mut converted = Vec::with_capacity(names.len());
    for name in names {
        let name = owned_name(name);
        converted.push(name.to_ref(xot).name_id())
    }
    converted
}

fn owned_name(name: &str) -> OwnedName {
    let captures = URI_QUALIFIED_NAME_REGEX.captures(name);
    if let Some(captures) = captures {
        let ns = captures.name("ns").unwrap().as_str().to_string();
        let name = captures.name("name").unwrap().as_str().to_string();
        OwnedName::new(name, ns, "".to_string())
    } else {
        OwnedName::new(name.to_string(), "".to_string(), "".to_string())
    }
}
