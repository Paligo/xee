use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use ahash::HashMap;
use clap::{CommandFactory, Parser};
use rustyline::error::ReadlineError;
use xee_xpath::{DocumentHandle, Documents, Itemable, Query};

use crate::{
    error::{render_error, render_parse_error},
    repl_cmd::{ArgumentDefinition, CommandDefinition, CommandDefinitions},
    Cli,
};

#[derive(Debug, Parser)]
pub(crate) struct Repl {
    /// Optional input file.
    pub(crate) infile: Option<PathBuf>,
    /// Namespace URI to use in XPath for element names without a namespace
    /// prefix.
    ///
    /// If omitted, the default namespace is the empty string (i.e. the
    /// names are not in a namespace).
    #[arg(long)]
    pub(crate) default_namespace_uri: Option<String>,
    /// Namespace declaration to make available in XPath (can be repeated)
    /// The format is prefix=uri.
    #[arg(long)]
    pub(crate) namespace: Vec<String>,
}

pub(crate) struct RunContext {
    documents: Documents,
    document_handle: Option<DocumentHandle>,
    default_namespace_uri: Option<String>,
    namespaces: HashMap<String, String>,
}

impl RunContext {
    fn new() -> Self {
        Self {
            documents: Documents::new(),
            document_handle: None,
            default_namespace_uri: None,
            namespaces: HashMap::default(),
        }
    }

    fn set_default_namespace_uri(&mut self, default_namespace_uri: String) {
        self.default_namespace_uri = Some(default_namespace_uri);
    }

    fn add_namespace_declaration(&mut self, prefix: String, uri: String) {
        self.namespaces.insert(prefix, uri);
    }

    fn set_context_document(&mut self, path: &Path) {
        let mut reader = match File::open(path) {
            Ok(file) => BufReader::new(file),
            Err(e) => {
                eprintln!("Error opening file: {}", e);
                return;
            }
        };
        let mut input_xml = String::new();
        match reader.read_to_string(&mut input_xml) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                return;
            }
        }

        let document_handle = match self.documents.add_string_without_uri(&input_xml) {
            Ok(doc) => Some(doc),
            Err(e) => {
                match e {
                    xee_xpath::error::DocumentsError::Parse(e) => render_parse_error(&input_xml, e),
                    xee_xpath::error::DocumentsError::DuplicateUri(uri) => {
                        eprintln!("Duplicate URI: {}", uri);
                    }
                }
                return;
            }
        };
        self.document_handle = document_handle;
    }

    fn queries(&self) -> xee_xpath::Queries<'_> {
        let mut static_context_builder = xee_xpath::context::StaticContextBuilder::default();
        if let Some(default_namespace_uri) = &self.default_namespace_uri {
            static_context_builder.default_element_namespace(default_namespace_uri);
        }
        for (prefix, uri) in &self.namespaces {
            static_context_builder.add_namespace(prefix, uri);
        }
        xee_xpath::Queries::new(static_context_builder)
    }

    pub(crate) fn execute(&mut self, xpath: &str) -> xee_xpath::error::Result<()> {
        let queries = self.queries();
        let sequence_query = queries.sequence(xpath);
        let sequence_query = match sequence_query {
            Ok(sequence_query) => sequence_query,
            Err(e) => {
                render_error(xpath, e);
                return Ok(());
            }
        };
        let mut context_builder = sequence_query.dynamic_context_builder(&self.documents);
        if let Some(doc) = self.document_handle {
            context_builder.context_item(doc.to_item(&self.documents)?);
        }
        let context = context_builder.build();

        let sequence = sequence_query.execute_with_context(&mut self.documents, &context);
        let sequence = match sequence {
            Ok(sequence) => sequence,
            Err(e) => {
                render_error(xpath, e);
                return Ok(());
            }
        };
        println!(
            "{}",
            sequence.display_representation(self.documents.xot(), &context)
        );
        Ok(())
    }
}

impl Repl {
    pub(crate) fn run(self) -> anyhow::Result<()> {
        let mut run_context = RunContext::new();
        if let Some(infile) = &self.infile {
            run_context.set_context_document(infile);
        }
        if let Some(default_namespace_uri) = self.default_namespace_uri {
            run_context.set_default_namespace_uri(default_namespace_uri);
        }
        for namespace in self.namespace {
            let parts = namespace.split('=').collect::<Vec<_>>();
            if parts.len() != 2 {
                return Err(anyhow::anyhow!(
                    "Invalid namespace declaration: {}",
                    namespace
                ));
            }
            run_context.add_namespace_declaration(parts[0].to_string(), parts[1].to_string());
        }

        let command_definitions = CommandDefinitions::new(vec![
            CommandDefinition::new(
                "load",
                Some("l"),
                "Load an XML file and make it context",
                vec![ArgumentDefinition::new("path", None)],
                Box::new(|args, run_context, _| {
                    let path: PathBuf = args[0].into();
                    run_context.set_context_document(&path);
                }),
            ),
            CommandDefinition::new(
                "default_namespace",
                Some("d"),
                "Set the default namespace URI for XPath",
                vec![ArgumentDefinition::new("uri", None)],
                Box::new(|args, run_context, _| {
                    run_context.set_default_namespace_uri(args[0].to_string());
                }),
            ),
            CommandDefinition::new(
                "namespace",
                Some("n"),
                "Add a namespace declaration for XPath",
                vec![
                    ArgumentDefinition::new("prefix", None),
                    ArgumentDefinition::new("uri", None),
                ],
                Box::new(|args, run_context, _| {
                    run_context.add_namespace_declaration(args[0].to_string(), args[1].to_string());
                }),
            ),
            CommandDefinition::new(
                "help",
                Some("h"),
                "Display this help",
                vec![],
                Box::new(|_, _, definitions| {
                    println!("Either enter an XPath expression or a special command prefixed by !");
                    println!("Commands:");
                    for definition in &definitions.definitions {
                        println!("  {}", definition.help());
                    }
                    println!("  !quit - Quit the REPL (!q)");
                }),
            ),
        ]);

        println!(
            "Xee XPath REPL {}",
            Cli::command().get_version().unwrap_or_default(),
        );
        println!("Type !help for more information.");
        let mut rl = rustyline::DefaultEditor::new()?;
        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }
                    rl.add_history_entry(line)?;
                    if !line.starts_with("!") {
                        match run_context.execute(line) {
                            Ok(()) => {}
                            Err(e) => {
                                render_error(line, e);
                            }
                        }
                    } else {
                        let command = line[1..].trim();
                        if command == "quit" || command == "q" {
                            break;
                        }
                        command_definitions.execute(command, &mut run_context);
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        Ok(())
    }
}
