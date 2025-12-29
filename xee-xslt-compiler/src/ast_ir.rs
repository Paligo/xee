use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::path::{Path, PathBuf};
use xee_name::{Name, Namespaces, FN_NAMESPACE};

use xee_interpreter::{context::StaticContext, declaration::CatchError, error, interpreter, sequence::QNameOrString};
use xee_ir::{compile_xslt, ir, Bindings, Variables};
use xee_xpath_ast::{ast as xpath_ast, parse_name, pattern::transform_pattern, span::Spanned};
use xee_schema_type::Xs;
use xee_xpath_compiler::UserFunctions;
use xee_xslt_ast::{ast, parse_transform};
use xee_xslt_ast::error::{AttributeError, ElementError};
use xot::xmlname::NameStrInfo;

use crate::{default_declarations::text_only_copy_declarations, priority::default_priority};

struct IrConverter<'a> {
    variables: Variables,
    static_context: &'a StaticContext,
    global_params: Vec<ir::GlobalParam>,
    global_param_lookup: HashMap<Name, usize>,
    function_lookup: HashMap<(Name, u8), usize>,
    user_functions: Option<UserFunctions>,
}

#[derive(Debug, Clone)]
struct DeclarationWithImport {
    declaration: ast::Declaration,
    import_level: u32,
    is_builtin: bool,
}

#[allow(dead_code)]
pub fn compile(
    transform: ast::Transform,
    static_context: StaticContext,
) -> error::SpannedResult<interpreter::Program> {
    let declarations = transform
        .declarations
        .into_iter()
        .map(|declaration| DeclarationWithImport {
            declaration,
            import_level: 0,
            is_builtin: false,
        })
        .collect::<Vec<_>>();
    compile_with_imports(declarations, static_context)
}

pub fn parse(
    static_context: StaticContext,
    xslt: &str,
) -> error::SpannedResult<interpreter::Program> {
    parse_with_base(static_context, xslt, None)
}

pub fn parse_with_base(
    static_context: StaticContext,
    xslt: &str,
    base_path: Option<&Path>,
) -> error::SpannedResult<interpreter::Program> {
    let transform = parse_transform(xslt);
    // TODO: better error handling
    let transform = match transform {
        Ok(transform) => transform,
        Err(err) => {
            return Err(map_parse_error(err).into());
        }
    };
    let mut declarations = if let Some(base_path) = base_path {
        let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));
        let mut visited = HashSet::new();
        resolve_imports(transform, base_dir, &mut visited, 0)?
    } else {
        transform
            .declarations
            .into_iter()
            .map(|declaration| DeclarationWithImport {
                declaration,
                import_level: 0,
                is_builtin: false,
            })
            .collect()
    };
    let max_import_level = declarations
        .iter()
        .map(|decl| decl.import_level)
        .max()
        .unwrap_or(0);
    let default_import_level = max_import_level.saturating_add(1);
    let mut default_declarations = text_only_copy_declarations()
        .unwrap()
        .into_iter()
        .map(|declaration| DeclarationWithImport {
            declaration,
            import_level: default_import_level,
            is_builtin: true,
        })
        .collect::<Vec<_>>();
    default_declarations.append(&mut declarations);
    compile_with_imports(default_declarations, static_context)
}

fn compile_with_imports(
    declarations: Vec<DeclarationWithImport>,
    static_context: StaticContext,
) -> error::SpannedResult<interpreter::Program> {
    let mut ir_converter = IrConverter::new(&static_context);
    let declarations = ir_converter.transform_with_imports(&declarations)?;
    compile_xslt(declarations, static_context)
}

fn resolve_imports(
    transform: ast::Transform,
    base_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    import_level: u32,
) -> error::SpannedResult<Vec<DeclarationWithImport>> {
    let mut import_decls = Vec::new();
    let mut local_decls = Vec::new();

    for decl in transform.declarations {
        match decl {
            ast::Declaration::Import(import) => {
                let import_path = resolve_import_path(&import.href, base_dir)?;
                let canonical =
                    std::fs::canonicalize(&import_path).unwrap_or_else(|_| import_path.clone());
                if !visited.insert(canonical) {
                    return Err(error::Error::Unsupported(
                        "Circular xsl:import detected".to_string(),
                    )
                    .into());
                }
                let xslt = std::fs::read_to_string(&import_path).map_err(|e| {
                    error::Error::Unsupported(format!(
                        "Failed to read xsl:import href '{}': {}",
                        import.href, e
                    ))
                })?;
                let import_transform = parse_transform(&xslt).map_err(map_parse_error)?;
                let import_base_dir = import_path.parent().unwrap_or(base_dir);
                let import_transform =
                    resolve_imports(import_transform, import_base_dir, visited, import_level + 1)?;
                import_decls.extend(import_transform);
            }
            ast::Declaration::Include(include) => {
                let include_path = resolve_import_path(&include.href, base_dir)?;
                let canonical =
                    std::fs::canonicalize(&include_path).unwrap_or_else(|_| include_path.clone());
                if !visited.insert(canonical) {
                    return Err(error::Error::Unsupported(
                        "Circular xsl:include detected".to_string(),
                    )
                    .into());
                }
                let xslt = std::fs::read_to_string(&include_path).map_err(|e| {
                    error::Error::Unsupported(format!(
                        "Failed to read xsl:include href '{}': {}",
                        include.href, e
                    ))
                })?;
                let include_transform = parse_transform(&xslt).map_err(map_parse_error)?;
                let include_base_dir = include_path.parent().unwrap_or(base_dir);
                let include_transform = resolve_imports(
                    include_transform,
                    include_base_dir,
                    visited,
                    import_level,
                )?;
                local_decls.extend(include_transform);
            }
            _ => local_decls.push(DeclarationWithImport {
                declaration: decl,
                import_level,
                is_builtin: false,
            }),
        }
    }

    import_decls.extend(local_decls);
    Ok(import_decls)
}

fn resolve_import_path(href: &str, base_dir: &Path) -> error::SpannedResult<PathBuf> {
    let href = href.strip_prefix("file://").unwrap_or(href);
    let href_path = Path::new(href);
    let path = if href_path.is_absolute() {
        href_path.to_path_buf()
    } else {
        base_dir.join(href_path)
    };
    Ok(path)
}

fn map_parse_error(err: ElementError) -> error::Error {
    match err {
        ElementError::Attribute(attr) => map_attribute_error(attr),
        ElementError::Unexpected { .. } | ElementError::UnexpectedEnd => error::Error::XTSE0010,
        ElementError::ValueTemplate(_) => error::Error::XTSE0020,
        ElementError::XPathRunTime(spanned) => spanned.error,
        ElementError::Unsupported(reason) => error::Error::Unsupported(reason),
        ElementError::Internal => error::Error::Unsupported(String::from("Internal XSLT error")),
    }
}

fn map_attribute_error(err: AttributeError) -> error::Error {
    match err {
        AttributeError::NotFound { .. } => error::Error::XTSE0010,
        AttributeError::Unexpected { .. } => error::Error::XTSE0090,
        AttributeError::Invalid { .. } | AttributeError::InvalidEqName { .. } => {
            error::Error::XTSE0020
        }
        AttributeError::XPathParser(err) => {
            eprintln!("XPath parse error: {err:?}");
            error::Error::XPST0003
        }
        AttributeError::ValueTemplate(_) => error::Error::XPST0003,
        AttributeError::Internal => error::Error::Unsupported(String::from("Internal XSLT error")),
    }
}

impl<'a> IrConverter<'a> {
    fn new(static_context: &'a StaticContext) -> Self {
        IrConverter {
            variables: Variables::new(),
            static_context,
            global_params: Vec::new(),
            global_param_lookup: HashMap::new(),
            function_lookup: HashMap::new(),
            user_functions: None,
        }
    }

    fn main_sequence_constructor(&mut self) -> ast::SequenceConstructor {
        vec![ast::SequenceConstructorItem::Instruction(
            ast::SequenceConstructorInstruction::ApplyTemplates(Box::new(ast::ApplyTemplates {
                // TODO: mode should be configurable from the outside somehow,
                // the XSTL test suite I think requires this.
                mode: ast::ApplyTemplatesModeValue::Unnamed,
                select: ast::Expression {
                    xpath: xee_xpath_ast::ast::XPath::parse(
                        "/",
                        &Namespaces::default(),
                        &xee_name::VariableNames::default(),
                    )
                    .unwrap(),
                    span: xee_xslt_ast::ast::Span::new(0, 0),
                },
                content: vec![],
                span: xee_xslt_ast::ast::Span::new(0, 0),
            })),
        )]
    }

    fn simple_content_atom(&mut self) -> ir::Atom {
        self.static_function_atom("simple-content", FN_NAMESPACE, 2)
    }

    fn concat_atom(&mut self, arity: u8) -> ir::Atom {
        self.static_function_atom("concat", FN_NAMESPACE, arity)
    }

    // fn error_atom(&mut self) -> ir::Atom {
    //     self.static_function_atom("error", Some(FN_NAMESPACE), 0)
    // }

    fn static_function_atom(&mut self, name: &str, namespace: &str, arity: u8) -> ir::Atom {
        ir::Atom::Const(ir::Const::StaticFunctionReference(
            self.static_context
                .function_id_by_name(
                    &Name::new(name.to_string(), namespace.to_string(), String::new()),
                    arity,
                )
                .unwrap(),
            None,
        ))
    }

    fn simple_content_expr(
        &mut self,
        select_atom: ir::AtomS,
        separator_atom: ir::AtomS,
    ) -> ir::Expr {
        ir::Expr::FunctionCall(ir::FunctionCall {
            atom: Spanned::new(self.simple_content_atom(), (0..0).into()),
            args: vec![select_atom, separator_atom],
        })
    }

    fn transform_with_imports(
        &mut self,
        declarations: &[DeclarationWithImport],
    ) -> error::SpannedResult<ir::Declarations> {
        self.collect_global_params(declarations)?;
        self.collect_functions(declarations)?;
        let main_sequence_constructor = self.main_sequence_constructor();
        let main = self.sequence_constructor_function(&main_sequence_constructor, &[])?;
        let mut ir_declarations = ir::Declarations::new(main);

        for declaration in declarations {
            self.declaration_with_import(
                &mut ir_declarations,
                &declaration.declaration,
                declaration.import_level,
                declaration.is_builtin,
            )?;
        }
        ir_declarations.global_params = self.global_params.clone();
        Ok(ir_declarations)
    }

    fn declaration_with_import(
        &mut self,
        declarations: &mut ir::Declarations,
        declaration: &ast::Declaration,
        import_level: u32,
        is_builtin: bool,
    ) -> error::SpannedResult<()> {
        use ast::Declaration::*;
        match declaration {
            Template(template) => self.template(declarations, template, import_level, is_builtin),
            Mode(mode) => self.mode(declarations, mode),
            Output(output) => self.output(declarations, output),
            Param(param) => self.param_declaration(param),
            Variable(variable) => self.variable_declaration(variable),
            Function(function) => self.function_declaration(declarations, function),
            Import(_) | Include(_) => Ok(()),
            _ => Err(error::Error::Unsupported(format!(
                "Declaration not supported: {:?}",
                declaration
            ))
            .into()),
        }
    }

    fn template(
        &mut self,
        declarations: &mut ir::Declarations,
        template: &ast::Template,
        import_level: u32,
        is_builtin: bool,
    ) -> error::SpannedResult<()> {
        if template.match_.is_none() && template.name.is_none() {
            return Err(
                error::Error::Unsupported("Template without match or name".to_string()).into(),
            );
        }

        let context_names = self.variables.push_context();
        let (template_params, template_ir_params) = self.template_params(&template.params)?;
        let function_definition = self.sequence_constructor_function_with_context(
            &context_names,
            &template.sequence_constructor,
            &template_ir_params,
        )?;
        self.variables.pop_context();

        if let Some(pattern) = &template.match_ {
            let priorities = if let Some(priority) = &template.priority {
                vec![(pattern.pattern.clone(), *priority)]
            } else {
                default_priority(&pattern.pattern)
                    .map(|(p, d)| (p.into_owned(), d))
                    .collect()
            };

            let modes = template
                .mode
                .iter()
                .map(Self::ast_mode_value_to_ir_mode_value)
                .collect::<Vec<_>>();

            for (pattern, priority) in priorities {
                declarations.rules.push(ir::Rule {
                    priority,
                    modes: modes.clone(),
                    import_level,
                    is_builtin,
                    pattern: transform_pattern(&pattern, |expr| self.pattern_predicate(expr))?,
                    function_definition: function_definition.clone(),
                    template_params: template_params.clone(),
                });
            }
        }

        if let Some(name) = &template.name {
            declarations.named_templates.push(ir::NamedTemplate {
                name: name.clone(),
                function_definition,
                template_params,
            });
        }

        Ok(())
    }

    fn sequence_constructor_function_with_context(
        &mut self,
        context_names: &ir::ContextNames,
        sequence_constructor: &ast::SequenceConstructor,
        extra_params: &[ir::Param],
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let bindings = self.sequence_constructor(sequence_constructor)?;
        let mut params = vec![
            ir::Param {
                name: context_names.item.clone(),
                type_: None,
            },
            ir::Param {
                name: context_names.position.clone(),
                type_: None,
            },
            ir::Param {
                name: context_names.last.clone(),
                type_: None,
            },
        ];
        params.extend(self.global_param_ir_params());
        params.extend(extra_params.iter().cloned());
        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn function_declaration(
        &mut self,
        declarations: &mut ir::Declarations,
        function: &ast::Function,
    ) -> error::SpannedResult<()> {
        if function.override_
            || function.override_extension_function
            || function.new_each_time.is_some()
            || function.cache
        {
            return Err(error::Error::Unsupported(
                "Overridable or cached functions are not supported".to_string(),
            )
            .into());
        }
        if function.visibility.is_some() {
            return Err(error::Error::Unsupported(
                "Function visibility is not supported".to_string(),
            )
            .into());
        }
        if function.streamability.is_some() {
            return Err(error::Error::Unsupported(
                "Streamable functions are not supported".to_string(),
            )
            .into());
        }

        let function_params = self.function_params(&function.params)?;
        self.variables.push_absent_context();
        let bindings = self.sequence_constructor(&function.sequence_constructor)?;
        self.variables.pop_context();

        let mut params = function_params;
        params.extend(self.global_param_ir_params());

        let function_definition = ir::FunctionDefinition {
            params,
            return_type: function.as_.clone(),
            body: Box::new(bindings.expr()),
        };

        let arity = function.params.len();
        if arity > u8::MAX as usize {
            return Err(error::Error::Unsupported(
                "Function arity too large".to_string(),
            )
            .into());
        }

        declarations.functions.push(ir::FunctionBinding {
            name: function.name.clone(),
            arity: arity as u8,
            main: function_definition,
        });
        Ok(())
    }

    fn mode(
        &mut self,
        declarations: &mut ir::Declarations,
        mode: &ast::Mode,
    ) -> error::SpannedResult<()> {
        let on_no_match = mode.on_no_match.as_ref().map(|m| match m {
            ast::OnNoMatch::DeepCopy => ir::OnNoMatch::DeepCopy,
            ast::OnNoMatch::ShallowCopy => ir::OnNoMatch::ShallowCopy,
            ast::OnNoMatch::DeepSkip => ir::OnNoMatch::DeepSkip,
            ast::OnNoMatch::ShallowSkip => ir::OnNoMatch::ShallowSkip,
            ast::OnNoMatch::TextOnlyCopy => ir::OnNoMatch::TextOnlyCopy,
            ast::OnNoMatch::Fail => ir::OnNoMatch::Fail,
        });
        declarations
            .modes
            .insert(mode.name.clone(), ir::Mode { on_no_match });
        Ok(())
    }

    fn output(
        &mut self,
        declarations: &mut ir::Declarations,
        output: &ast::Output,
    ) -> error::SpannedResult<()> {
        let serialization = &mut declarations.serialization_params;
        if output.name.is_some() {
            return Err(error::Error::Unsupported(String::from(
                "Output: Named outputs are not supported yet",
            ))
            .into());
        }
        if output.parameter_document.is_some() {
            return Err(error::Error::Unsupported(String::from(
                "Output: Parameter documents are not supported yet",
            ))
            .into());
        }
        if !output.use_character_maps.is_empty() {
            return Err(error::Error::Unsupported(String::from(
                "Output: Character maps are not supported yet",
            ))
            .into());
        }
        if output.build_tree {
            return Err(error::Error::Unsupported(String::from(
                "Output: Build tree is not supported yet",
            ))
            .into());
        }
        fn assign_if_some<T>(location: &mut T, value: Option<T>) {
            if let Some(v) = value {
                *location = v;
            }
        }
        serialization.allow_duplicate_names = output.allow_duplicate_names;
        serialization.byte_order_mark = output.byte_order_mark;
        serialization
            .cdata_section_elements
            .extend(output.cdata_section_elements.clone());
        serialization.doctype_public = output.doctype_public.clone();
        serialization.doctype_system = output.doctype_system.clone();
        match &output.method {
            Some(ast::OutputMethod::Xml) => {
                serialization.method = QNameOrString::String("xml".to_string())
            }
            Some(ast::OutputMethod::Html) => {
                serialization.method = QNameOrString::String("html".to_string())
            }
            Some(ast::OutputMethod::Json) => {
                serialization.method = QNameOrString::String("json".to_string())
            }
            None => {}
            method => {
                return Err(error::Error::Unsupported(format!(
                    "Output method {:?} not supported yet",
                    method
                ))
                .into());
            }
        };
        assign_if_some(&mut serialization.encoding, output.encoding.clone());
        serialization.escape_uri_attributes = output.escape_uri_attributes;
        assign_if_some(&mut serialization.html_version, output.html_version);
        serialization.include_content_type = output.include_content_type;
        serialization.indent = output.indent;
        assign_if_some(
            &mut serialization.item_separator,
            output.item_separator.clone(),
        );
        match &output.json_node_output_method {
            Some(ast::JsonNodeOutputMethod::Xml) => {
                serialization.json_node_output_method = QNameOrString::String("xml".to_string())
            }
            Some(ast::JsonNodeOutputMethod::Html) => {
                serialization.json_node_output_method = QNameOrString::String("html".to_string())
            }
            None => {}
            method => {
                return Err(error::Error::Unsupported(format!(
                    "JSON node output method {:?} not supported yet",
                    method
                ))
                .into());
            }
        }
        serialization.media_type = output.media_type.clone();
        serialization.normalization_form =
            output.normalization_form.as_ref().and_then(|nf| match nf {
                ast::NormalizationForm::Nfc => Some(String::from("NFC")),
                ast::NormalizationForm::Nfd => Some(String::from("NFD")),
                ast::NormalizationForm::Nfkc => Some(String::from("NFKC")),
                ast::NormalizationForm::Nfkd => Some(String::from("NFKD")),
                ast::NormalizationForm::FullyNormalized => Some(String::from("fully-normalized")),
                ast::NormalizationForm::NmToken(nm) => Some(nm.clone()),
                ast::NormalizationForm::None => None,
            });
        serialization.omit_xml_declaration = output.omit_xml_declaration;
        assign_if_some(
            &mut serialization.standalone,
            output.standalone.as_ref().map(|s| match s {
                ast::Standalone::Bool(b) => Some(*b),
                ast::Standalone::Omit => None,
            }),
        );
        serialization
            .suppress_indentation
            .extend(output.suppress_indentation.clone());
        serialization.undeclare_prefixes = output.undeclare_prefixes;
        assign_if_some(&mut serialization.version, output.version.clone());
        Ok(())
    }

    fn ast_mode_value_to_ir_mode_value(mode: &ast::ModeValue) -> ir::ModeValue {
        match mode {
            ast::ModeValue::EqName(name) => ir::ModeValue::Named(name.clone()),
            ast::ModeValue::Unnamed => ir::ModeValue::Unnamed,
            ast::ModeValue::All => ir::ModeValue::All,
        }
    }

    fn sequence_constructor_function(
        &mut self,
        sequence_constructor: &ast::SequenceConstructor,
        extra_params: &[ir::Param],
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        let bindings = self.sequence_constructor(sequence_constructor)?;
        self.variables.pop_context();
        let mut params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
            },
        ];
        params.extend(self.global_param_ir_params());
        params.extend(extra_params.iter().cloned());
        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn sequence_constructor(
        &mut self,
        sequence_constructor: &[ast::SequenceConstructorItem],
    ) -> error::SpannedResult<Bindings> {
        if sequence_constructor.is_empty() {
            let empty_sequence = self.empty_sequence();
            return Ok(Bindings::new(
                self.variables
                    .new_binding(empty_sequence.value, empty_sequence.span),
            ));
        }

        for (index, item) in sequence_constructor.iter().enumerate() {
            if let Some((name, var_bindings)) = self.variable(item)? {
                let rest_bindings = self.sequence_constructor(&sequence_constructor[index + 1..])?;
                let expr = ir::Expr::Let(ir::Let {
                    name,
                    var_expr: Box::new(var_bindings.expr()),
                    return_expr: Box::new(rest_bindings.expr()),
                });
                let let_bindings =
                    Bindings::new(self.variables.new_binding(expr, (0..0).into()));
                if index == 0 {
                    return Ok(let_bindings);
                }
                let prefix_bindings = self.sequence_constructor_concat(
                    &sequence_constructor[0],
                    sequence_constructor[1..index].iter(),
                )?;
                let (left_atom, left_bindings) = prefix_bindings.atom_bindings();
                let (right_atom, right_bindings) = let_bindings.atom_bindings();
                let expr = ir::Expr::Binary(ir::Binary {
                    left: left_atom,
                    op: ir::BinaryOperator::Comma,
                    right: right_atom,
                });
                let binding = self.variables.new_binding_no_span(expr);
                return Ok(left_bindings.concat(right_bindings).bind(binding));
            }
        }

        let mut items = sequence_constructor.iter();
        let left = items.next().expect("sequence_constructor not empty");
        self.sequence_constructor_concat(left, items)
    }

    fn sequence_constructor_concat<'b>(
        &mut self,
        left: &ast::SequenceConstructorItem,
        items: impl Iterator<Item = &'b ast::SequenceConstructorItem>,
    ) -> error::SpannedResult<Bindings> {
        let left_bindings = Ok(self.sequence_constructor_item(left)?);
        items.fold(left_bindings, |left, right| {
            let mut left_bindings = left?;
            let mut right_bindings = self.sequence_constructor_item(right)?;
            let expr = ir::Expr::Binary(ir::Binary {
                left: left_bindings.atom(),
                op: ir::BinaryOperator::Comma,
                right: right_bindings.atom(),
            });
            let binding = self.variables.new_binding_no_span(expr);
            Ok(left_bindings.concat(right_bindings).bind(binding))
        })
    }

    fn sequence_constructor_item(
        &mut self,
        item: &ast::SequenceConstructorItem,
    ) -> error::SpannedResult<Bindings> {
        match item {
            ast::SequenceConstructorItem::Instruction(instruction) => {
                self.sequence_constructor_instruction(instruction)
            }
            ast::SequenceConstructorItem::Content(content) => {
                self.sequence_constructor_content(content)
            }
        }
    }

    fn sequence_constructor_instruction(
        &mut self,
        instruction: &ast::SequenceConstructorInstruction,
    ) -> error::SpannedResult<Bindings> {
        use ast::SequenceConstructorInstruction::*;
        match instruction {
            ApplyTemplates(apply_templates) => self.apply_templates(apply_templates),
            ApplyImports(apply_imports) => self.apply_imports(apply_imports),
            NextMatch(next_match) => self.next_match(next_match),
            CallTemplate(call_template) => self.call_template(call_template),
            ValueOf(value_of) => self.value_of(value_of),
            If(if_) => self.if_(if_),
            Choose(choose) => self.choose(choose),
            Assert(assert_) => self.assert_(assert_),
            ForEach(for_each) => self.for_each(for_each),
            Iterate(iterate) => self.iterate(iterate),
            NextIteration(next_iteration) => self.next_iteration(next_iteration),
            Break(break_) => self.break_(break_),
            Copy(copy) => self.copy(copy),
            CopyOf(copy_of) => self.copy_of(copy_of),
            Sequence(sequence) => self.sequence(sequence),
            Document(document) => self.document(document),
            Element(element) => self.element(element),
            Text(text) => self.text(text),
            Try(try_) => self.try_(try_),
            Attribute(attribute) => self.attribute(attribute),
            Namespace(namespace) => self.namespace(namespace),
            Comment(comment) => self.comment(comment),
            ProcessingInstruction(pi) => self.processing_instruction(pi),
            // TODO: xsl:variable does not produce content and is handled
            // earlier already should be unreachable!() but at this point this
            // can be reached so return unsupported
            Variable(_variable) => Err(error::Error::Unsupported(String::from(
                "Internal bug: variable node should have been processed already",
            ))
            .into()),
            _ => Err(error::Error::Unsupported(format!(
                "Instruction not supported: {:?}",
                instruction
            ))
            .into()),
        }
    }

    fn sequence_constructor_content(
        &mut self,
        content: &ast::Content,
    ) -> error::SpannedResult<Bindings> {
        match content {
            ast::Content::Element(element_node) => {
                self.sequence_constructor_content_element(element_node)
            }
            ast::Content::Text(text) => {
                let text_atom = Spanned::new(
                    ir::Atom::Const(ir::Const::String(text.clone())),
                    (0..0).into(),
                );
                let bindings = Bindings::empty();
                Ok(bindings.bind_expr_no_span(
                    &mut self.variables,
                    ir::Expr::XmlText(ir::XmlText { value: text_atom }),
                ))
            }
            ast::Content::Value(expression) => {
                let (atom, bindings) = self.expression(expression)?.atom_bindings();
                let expr = self.simple_content_expr(atom, self.space_separator_atom());
                let (text_atom, bindings) = bindings
                    .bind_expr_no_span(&mut self.variables, expr)
                    .atom_bindings();
                Ok(bindings.bind_expr_no_span(
                    &mut self.variables,
                    ir::Expr::XmlText(ir::XmlText { value: text_atom }),
                ))
            }
        }
    }

    fn sequence_constructor_content_element(
        &mut self,
        element_node: &ast::ElementNode,
    ) -> error::SpannedResult<Bindings> {
        let (name_atom, bindings) = self.xml_name(&element_node.name)?.atom_bindings();
        let name_expr = ir::Expr::XmlElement(ir::XmlElement { name: name_atom });
        let (element_atom, mut bindings) = bindings
            .bind_expr_no_span(&mut self.variables, name_expr)
            .atom_bindings();
        if let Some(type_name) = &element_node.type_ {
            let xs = self.xs_type_from_eqname(type_name, element_node.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: element_atom.clone(),
                xs,
            });
            let set_type_bindings =
                bindings.bind_expr_no_span(&mut self.variables, set_type_expr);
            bindings = bindings.concat(set_type_bindings);
        }
        for (name, value) in &element_node.attributes {
            let (value_atom, value_bindings) =
                self.attribute_value_template(value)?.atom_bindings();
            let (attribute_name_atom, attribute_bindings) = self.xml_name(name)?.atom_bindings();
            let value_bindings = value_bindings.concat(attribute_bindings);
            let attribute_expr = ir::Expr::XmlAttribute(ir::XmlAttribute {
                name: attribute_name_atom,
                value: value_atom,
            });
            let (attribute_atom, attribute_bindings) = value_bindings
                .bind_expr_no_span(&mut self.variables, attribute_expr)
                .atom_bindings();
            let append_expr = ir::Expr::XmlAppend(ir::XmlAppend {
                parent: element_atom.clone(),
                child: attribute_atom,
            });
            let append_bindings =
                attribute_bindings.bind_expr_no_span(&mut self.variables, append_expr);
            bindings = bindings.concat(append_bindings);
        }
        let sequence_constructor_bindings = self.sequence_constructor_append(
            element_atom.clone(),
            &element_node.sequence_constructor,
        )?;
        let bindings = bindings.concat(sequence_constructor_bindings);
        Ok(bindings)
    }

    fn xs_type_from_eqname(
        &self,
        type_name: &ast::EqName,
        span: ast::Span,
    ) -> error::SpannedResult<Xs> {
        Xs::by_name(type_name.namespace(), type_name.local_name()).ok_or_else(|| {
            let span = xpath_ast::Span::new(span.start, span.end);
            error::Error::Unsupported("xsl:type only supports xs:* names".to_string())
                .with_ast_span(span)
        })
    }

    fn sequence_constructor_append(
        &mut self,
        element_atom: ir::AtomS,
        sequence_constructor: &ast::SequenceConstructor,
    ) -> error::SpannedResult<Bindings> {
        if !sequence_constructor.is_empty() {
            let (atom, bindings) = self
                .sequence_constructor(sequence_constructor)?
                .atom_bindings();
            let append = ir::Expr::XmlAppend(ir::XmlAppend {
                parent: element_atom,
                child: atom,
            });
            let bindings = bindings.bind_expr_no_span(&mut self.variables, append);
            Ok(bindings)
        } else {
            Ok(Bindings::empty())
        }
    }

    fn space_separator_atom(&self) -> ir::AtomS {
        Spanned::new(
            ir::Atom::Const(ir::Const::String(" ".to_string())),
            (0..0).into(),
        )
    }

    fn apply_templates(
        &mut self,
        apply_templates: &ast::ApplyTemplates,
    ) -> error::SpannedResult<Bindings> {
        let (select_atom, bindings) = self.expression(&apply_templates.select)?.atom_bindings();
        let mode = match &apply_templates.mode {
            ast::ApplyTemplatesModeValue::EqName(name) => {
                ir::ApplyTemplatesModeValue::Named(name.clone())
            }
            ast::ApplyTemplatesModeValue::Unnamed => ir::ApplyTemplatesModeValue::Unnamed,
            ast::ApplyTemplatesModeValue::Current => ir::ApplyTemplatesModeValue::Current,
        };

        let mut params = Vec::new();
        let mut bindings = bindings;
        for content in &apply_templates.content {
            let with_param = match content {
                ast::ApplyTemplatesContent::WithParam(with_param) => with_param,
                ast::ApplyTemplatesContent::Sort(_) => {
                    continue;
                }
            };
            if with_param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel params are not supported".to_string(),
                )
                .into());
            }
            let (value_atom, value_bindings) =
                self.with_param_value_atom(with_param)?.atom_bindings();
            bindings = bindings.concat(value_bindings);
            params.push(ir::WithParam {
                name: with_param.name.clone(),
                value: value_atom,
            });
        }

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::ApplyTemplates(ir::ApplyTemplates {
                mode,
                select: select_atom,
                params,
            }),
        ))
    }

    fn apply_imports(
        &mut self,
        apply_imports: &ast::ApplyImports,
    ) -> error::SpannedResult<Bindings> {
        let mut params = Vec::new();
        let mut bindings = Bindings::empty();
        for with_param in &apply_imports.with_params {
            if with_param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel params are not supported".to_string(),
                )
                .into());
            }
            let (value_atom, value_bindings) =
                self.with_param_value_atom(with_param)?.atom_bindings();
            bindings = bindings.concat(value_bindings);
            params.push(ir::WithParam {
                name: with_param.name.clone(),
                value: value_atom,
            });
        }

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::ApplyImports(ir::ApplyImports { params }),
        ))
    }

    fn next_match(&mut self, next_match: &ast::NextMatch) -> error::SpannedResult<Bindings> {
        let mut params = Vec::new();
        let mut bindings = Bindings::empty();
        for content in &next_match.content {
            let with_param = match content {
                ast::NextMatchContent::WithParam(with_param) => with_param,
                ast::NextMatchContent::Fallback(_) => {
                    return Err(error::Error::Unsupported(
                        "xsl:fallback not supported".to_string(),
                    )
                    .into());
                }
            };
            if with_param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel params are not supported".to_string(),
                )
                .into());
            }
            let (value_atom, value_bindings) =
                self.with_param_value_atom(with_param)?.atom_bindings();
            bindings = bindings.concat(value_bindings);
            params.push(ir::WithParam {
                name: with_param.name.clone(),
                value: value_atom,
            });
        }

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::NextMatch(ir::NextMatch { params }),
        ))
    }

    fn call_template(
        &mut self,
        call_template: &ast::CallTemplate,
    ) -> error::SpannedResult<Bindings> {
        let mut params = Vec::new();
        let mut bindings = Bindings::empty();
        for with_param in &call_template.with_params {
            if with_param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel params are not supported".to_string(),
                )
                .into());
            }
            let (value_atom, value_bindings) =
                self.with_param_value_atom(with_param)?.atom_bindings();
            bindings = bindings.concat(value_bindings);
            params.push(ir::WithParam {
                name: with_param.name.clone(),
                value: value_atom,
            });
        }

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::CallTemplate(ir::CallTemplate {
                name: call_template.name.clone(),
                params,
            }),
        ))
    }

    fn try_(&mut self, try_: &ast::Try) -> error::SpannedResult<Bindings> {
        let try_body = self.select_or_sequence_constructor_function(
            try_.select.as_ref(),
            &try_.sequence_constructor,
        )?;

        let mut catches = Vec::new();
        catches.push(self.catch_clause(&try_.catch)?);

        for entry in &try_.catches {
            match entry {
                ast::TryCatchOrFallback::Catch(catch) => {
                    catches.push(self.catch_clause(catch)?);
                }
                ast::TryCatchOrFallback::Fallback(_) => {
                    return Err(error::Error::Unsupported(
                        "xsl:fallback in xsl:try is not supported".to_string(),
                    )
                    .into());
                }
            }
        }

        let expr = ir::Expr::TryCatch(ir::TryCatch {
            try_body,
            catches,
            rollback_output: try_.rollback_output.unwrap_or(true),
        });

        Ok(Bindings::new(self.variables.new_binding_no_span(expr)))
    }

    fn catch_clause(&mut self, catch: &ast::Catch) -> error::SpannedResult<ir::CatchClause> {
        let errors = self.parse_catch_errors(catch.errors.as_ref())?;
        let body = self.select_or_sequence_constructor_function(
            catch.select.as_ref(),
            &catch.sequence_constructor,
        )?;
        Ok(ir::CatchClause { errors, body })
    }

    fn select_or_sequence_constructor_function(
        &mut self,
        select: Option<&ast::Expression>,
        sequence_constructor: &ast::SequenceConstructor,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        if let Some(select) = select {
            self.expression_function(select)
        } else {
            self.sequence_constructor_function(sequence_constructor, &[])
        }
    }

    fn expression_function(
        &mut self,
        expression: &ast::Expression,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        let bindings = self.expression(expression)?;
        self.variables.pop_context();
        let mut params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
            },
        ];
        params.extend(self.global_param_ir_params());
        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn parse_catch_errors(
        &self,
        errors: Option<&Vec<ast::Token>>,
    ) -> error::SpannedResult<Vec<CatchError>> {
        let errors = match errors {
            Some(errors) if !errors.is_empty() => errors,
            _ => return Ok(vec![CatchError::Any]),
        };
        let mut result = Vec::with_capacity(errors.len());
        for token in errors {
            result.push(self.parse_catch_error_token(token)?);
        }
        Ok(result)
    }

    fn parse_catch_error_token(&self, token: &str) -> error::SpannedResult<CatchError> {
        let token = token.trim();
        if token == "*" || token == "*:*" {
            return Ok(CatchError::Any);
        }
        if let Some(local) = token.strip_prefix("*:") {
            return Ok(CatchError::Local(local.to_string()));
        }
        if let Some(prefix) = token.strip_suffix(":*") {
            let namespace = self
                .static_context
                .namespaces()
                .by_prefix(prefix)
                .ok_or_else(|| {
                    error::Error::Unsupported(format!(
                        "Unknown namespace prefix in xsl:catch errors: {token}"
                    ))
                })?;
            return Ok(CatchError::Namespace(namespace.to_string()));
        }
        if let Some(qname) = token.strip_prefix("Q{") {
            if let Some(end) = qname.find('}') {
                let namespace = &qname[..end];
                let local = &qname[end + 1..];
                if local == "*" {
                    return Ok(CatchError::Namespace(namespace.to_string()));
                }
                if local.is_empty() {
                    return Err(error::Error::Unsupported(format!(
                        "Invalid xsl:catch errors token: {token}"
                    ))
                    .into());
                }
                return Ok(CatchError::Name(Name::new(
                    local.to_string(),
                    namespace.to_string(),
                    String::new(),
                )));
            }
        }
        if !token.contains(':') {
            return Ok(CatchError::Name(Name::new(
                token.to_string(),
                String::new(),
                String::new(),
            )));
        }

        match parse_name(token, self.static_context.namespaces()) {
            Ok(spanned) => Ok(CatchError::Name(spanned.value)),
            Err(_) => Err(error::Error::Unsupported(format!(
                "Invalid xsl:catch errors token: {token}"
            ))
            .into()),
        }
    }

    fn select_or_sequence_constructor(
        &mut self,
        instruction: &impl ast::SelectOrSequenceConstructor,
    ) -> error::SpannedResult<Bindings> {
        if let Some(select) = instruction.select() {
            self.expression(select)
        } else {
            self.sequence_constructor(instruction.sequence_constructor())
        }
    }

    fn sequence_constructor_document(
        &mut self,
        sequence_constructor: &ast::SequenceConstructor,
    ) -> error::SpannedResult<Bindings> {
        let doc_expr = ir::Expr::XmlDocument(ir::XmlRoot {});
        let (doc_atom, mut bindings) = Bindings::new(self.variables.new_binding_no_span(doc_expr))
            .atom_bindings();
        if !sequence_constructor.is_empty() {
            let (child_atom, child_bindings) =
                self.sequence_constructor(sequence_constructor)?.atom_bindings();
            bindings = bindings.concat(child_bindings);
            let append = ir::Expr::XmlAppend(ir::XmlAppend {
                parent: doc_atom,
                child: child_atom,
            });
            bindings = bindings.bind_expr_no_span(&mut self.variables, append);
        }
        Ok(bindings)
    }

    fn sequence_constructor_has_content(sequence_constructor: &ast::SequenceConstructor) -> bool {
        sequence_constructor.iter().any(|item| {
            matches!(item, ast::SequenceConstructorItem::Content(_))
        })
    }

    fn with_param_value_atom(
        &mut self,
        with_param: &ast::WithParam,
    ) -> error::SpannedResult<Bindings> {
        if let Some(as_) = &with_param.as_ {
            if let Some(occurrence) = Self::string_sequence_occurrence(as_) {
                match occurrence {
                    xpath_ast::Occurrence::One => {
                        return self.select_or_sequence_constructor_simple_content(with_param);
                    }
                    xpath_ast::Occurrence::Option => {
                        if with_param.select.is_some()
                            || !with_param.sequence_constructor.is_empty()
                        {
                            return self.select_or_sequence_constructor_simple_content(with_param);
                        }
                    }
                    _ => {}
                }
            }
        }
        if with_param.select.is_none()
            && with_param.sequence_constructor.is_empty()
            && with_param.as_.is_none()
        {
            return Ok(Bindings::new(self.variables.new_binding_no_span(ir::Expr::Atom(
                Spanned::new(
                    ir::Atom::Const(ir::Const::String(String::new())),
                    (0..0).into(),
                ),
            ))));
        }
        if with_param.select.is_none()
            && !with_param.sequence_constructor.is_empty()
            && Self::sequence_constructor_has_content(&with_param.sequence_constructor)
        {
            return self.sequence_constructor_document(&with_param.sequence_constructor);
        }
        self.select_or_sequence_constructor(with_param)
    }

    fn string_sequence_occurrence(
        sequence_type: &xpath_ast::SequenceType,
    ) -> Option<xpath_ast::Occurrence> {
        match sequence_type {
            xpath_ast::SequenceType::Item(item) => {
                matches!(
                    item.item_type,
                    xpath_ast::ItemType::AtomicOrUnionType(Xs::String)
                )
                .then_some(item.occurrence)
            }
            _ => None,
        }
    }

    fn select_or_sequence_constructor_simple_content(
        &mut self,
        instruction: &impl ast::SelectOrSequenceConstructor,
    ) -> error::SpannedResult<Bindings> {
        let (select_atom, bindings) = self
            .select_or_sequence_constructor(instruction)?
            .atom_bindings();

        let separator_atom = self.space_separator_atom();
        let expr = self.simple_content_expr(select_atom, separator_atom);
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn select_or_sequence_constructor_simple_content_with_separator(
        &mut self,
        instruction: &impl ast::SelectOrSequenceConstructor,
        separator: &Option<ast::ValueTemplate<String>>,
    ) -> error::SpannedResult<Bindings> {
        let (select_atom, select_bindings) = self
            .select_or_sequence_constructor(instruction)?
            .atom_bindings();

        let (separator_atom, separator_bindings) = if let Some(separator) = separator {
            self.attribute_value_template(separator)?
        } else {
            Bindings::new(
                self.variables
                    .new_binding_no_span(ir::Expr::Atom(self.space_separator_atom())),
            )
        }
        .atom_bindings();
        let bindings = select_bindings.concat(separator_bindings);
        let expr = self.simple_content_expr(select_atom, separator_atom);
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn value_of(&mut self, value_of: &ast::ValueOf) -> error::SpannedResult<Bindings> {
        let (text_atom, bindings) = self
            .select_or_sequence_constructor_simple_content_with_separator(
                value_of,
                &value_of.separator,
            )?
            .atom_bindings();

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlText(ir::XmlText { value: text_atom }),
        ))
    }

    fn assert_error_expr(&mut self, assert_: &ast::Assert) -> error::SpannedResult<ir::ExprS> {
        let (code_atom, code_bindings) = self.assert_error_code(assert_)?.atom_bindings();
        let (message_atom, message_bindings) = self.assert_message(assert_)?.atom_bindings();
        let bindings = code_bindings.concat(message_bindings);
        let error_atom = self.static_function_atom("error", FN_NAMESPACE, 2);
        let expr = ir::Expr::FunctionCall(ir::FunctionCall {
            atom: Spanned::new(error_atom, (0..0).into()),
            args: vec![code_atom, message_atom],
        });
        Ok(bindings
            .bind_expr_no_span(&mut self.variables, expr)
            .expr())
    }

    fn assert_error_code(&mut self, assert_: &ast::Assert) -> error::SpannedResult<Bindings> {
        let (namespace, local) = if let Some(error_code) = &assert_.error_code {
            let literal = self
                .value_template_literal(error_code)
                .ok_or_else(|| {
                    error::Error::Unsupported(
                        "xsl:assert error-code must be a literal in this implementation"
                            .to_string(),
                    )
                })?;
            self.parse_error_code_literal(&literal)?
        } else {
            (
                "http://www.w3.org/2005/xqt-errors".to_string(),
                "XTMM9001".to_string(),
            )
        };

        Ok(self.qname_expr(&namespace, &local))
    }

    fn assert_message(&mut self, assert_: &ast::Assert) -> error::SpannedResult<Bindings> {
        let (select_atom, bindings) = if let Some(select) = &assert_.select {
            self.expression(select)?.atom_bindings()
        } else {
            self.sequence_constructor(&assert_.sequence_constructor)?
                .atom_bindings()
        };

        let expr = self.simple_content_expr(select_atom, self.space_separator_atom());
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn qname_expr(&mut self, namespace: &str, qname: &str) -> Bindings {
        let namespace_atom = Spanned::new(
            ir::Atom::Const(ir::Const::String(namespace.to_string())),
            (0..0).into(),
        );
        let qname_atom = Spanned::new(
            ir::Atom::Const(ir::Const::String(qname.to_string())),
            (0..0).into(),
        );
        let qname_fn = self.static_function_atom("QName", FN_NAMESPACE, 2);
        let expr = ir::Expr::FunctionCall(ir::FunctionCall {
            atom: Spanned::new(qname_fn, (0..0).into()),
            args: vec![namespace_atom, qname_atom],
        });
        Bindings::new(self.variables.new_binding_no_span(expr))
    }

    fn value_template_literal<T>(&self, template: &ast::ValueTemplate<T>) -> Option<String>
    where
        T: Clone + PartialEq + Eq,
    {
        let mut out = String::new();
        for item in &template.template {
            match item {
                ast::ValueTemplateItem::String { text, .. } => out.push_str(text),
                ast::ValueTemplateItem::Curly { c } => out.push(*c),
                ast::ValueTemplateItem::Value { .. } => return None,
            }
        }
        Some(out)
    }

    fn parse_error_code_literal(&self, value: &str) -> error::SpannedResult<(String, String)> {
        if let Some(rest) = value.strip_prefix("Q{") {
            if let Some(end) = rest.find('}') {
                let namespace = rest[..end].to_string();
                let local = rest[end + 1..].to_string();
                if local.is_empty() {
                    return Err(error::Error::Unsupported(format!(
                        "Invalid error-code EQName: {}",
                        value
                    ))
                    .into());
                }
                return Ok((namespace, local));
            }
        }

        let local = value
            .rsplit_once(':')
            .map(|(_, local)| local)
            .unwrap_or(value)
            .to_string();

        if local.is_empty() {
            return Err(error::Error::Unsupported(format!(
                "Invalid error-code EQName: {}",
                value
            ))
            .into());
        }

        Ok((String::new(), local))
    }

    fn attribute_value_template(
        &mut self,
        value_template: &ast::ValueTemplate<String>,
    ) -> error::SpannedResult<Bindings> {
        let mut all_bindings = Vec::new();
        for item in &value_template.template {
            let bindings = match item {
                ast::ValueTemplateItem::String { text, span: _span } => {
                    let text_atom = Spanned::new(
                        ir::Atom::Const(ir::Const::String(text.clone())),
                        (0..0).into(),
                    );
                    let bindings = Bindings::empty();
                    bindings.bind_expr_no_span(&mut self.variables, ir::Expr::Atom(text_atom))
                }
                ast::ValueTemplateItem::Curly { c } => {
                    let text_atom = Spanned::new(
                        ir::Atom::Const(ir::Const::String(c.to_string())),
                        (0..0).into(),
                    );
                    let bindings = Bindings::empty();
                    bindings.bind_expr_no_span(&mut self.variables, ir::Expr::Atom(text_atom))
                }
                ast::ValueTemplateItem::Value { xpath, span: _ } => {
                    let (atom, bindings) = self.xpath(&xpath.0)?.atom_bindings();
                    let expr = self.simple_content_expr(atom, self.space_separator_atom());
                    bindings.bind_expr_no_span(&mut self.variables, expr)
                }
            };
            all_bindings.push(bindings);
        }
        Ok(if all_bindings.is_empty() {
            // empty attribute value template is a string
            let bindings = Bindings::empty();
            let empty_string = ir::Expr::Atom(self.empty_string());
            bindings.bind_expr_no_span(&mut self.variables, empty_string)
        } else if all_bindings.len() == 1 {
            // a single binding is just that binding
            all_bindings.pop().unwrap()
        } else {
            // TODO: speculative code, needs tests
            // if we have multiple bindings, concatenate each result into
            // a single string
            let mut combined_bindings = Bindings::empty();
            let mut atoms = Vec::new();
            for binding in all_bindings {
                let (atom, binding) = binding.atom_bindings();
                combined_bindings = combined_bindings.concat(binding);
                atoms.push(atom);
            }
            // concatenate all the pieces of content into a single string
            // TODO: this may create more than we have arities for, so we may want to use more
            // generic concat function that takes a sequence at some point
            let concat_atom = self.concat_atom(atoms.len() as u8);
            let expr = ir::Expr::FunctionCall(ir::FunctionCall {
                atom: Spanned::new(concat_atom, (0..0).into()),
                args: atoms,
            });
            combined_bindings.bind_expr_no_span(&mut self.variables, expr)
        })
    }

    fn variable(
        &mut self,
        item: &ast::SequenceConstructorItem,
    ) -> error::SpannedResult<Option<(ir::Name, Bindings)>> {
        if let ast::SequenceConstructorItem::Instruction(
            ast::SequenceConstructorInstruction::Variable(variable),
        ) = item
        {
            let name = self.variables.new_var_name(&variable.name);
            let var_bindings = if let Some(select) = &variable.select {
                self.expression(select)?
            } else {
                self.sequence_constructor(&variable.sequence_constructor)?
            };
            Ok(Some((name, var_bindings)))
        } else {
            Ok(None)
        }
    }

    fn empty_sequence(&mut self) -> ir::ExprS {
        Spanned::new(
            ir::Expr::Atom(Spanned::new(
                ir::Atom::Const(ir::Const::EmptySequence),
                (0..0).into(),
            )),
            (0..0).into(),
        )
    }

    fn empty_string(&self) -> ir::AtomS {
        Spanned::new(
            ir::Atom::Const(ir::Const::String("".to_string())),
            (0..0).into(),
        )
    }

    fn if_(&mut self, if_: &ast::If) -> error::SpannedResult<Bindings> {
        let (condition, bindings) = self.expression(&if_.test)?.atom_bindings();
        let expr = ir::Expr::If(ir::If {
            condition,
            then: Box::new(self.sequence_constructor(&if_.sequence_constructor)?.expr()),
            else_: Box::new(self.empty_sequence()),
        });
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn choose(&mut self, choose: &ast::Choose) -> error::SpannedResult<Bindings> {
        self.choose_when_otherwise(&choose.when, choose.otherwise.as_ref())
    }

    fn assert_(&mut self, assert_: &ast::Assert) -> error::SpannedResult<Bindings> {
        if !self.static_context.assertions_enabled() {
            let empty = self.empty_sequence().value;
            return Ok(Bindings::empty().bind_expr_no_span(
                &mut self.variables,
                empty,
            ));
        }
        let (condition, bindings) = self.expression(&assert_.test)?.atom_bindings();
        let error_expr = self.assert_error_expr(assert_)?;

        let expr = ir::Expr::If(ir::If {
            condition,
            then: Box::new(self.empty_sequence()),
            else_: Box::new(error_expr),
        });

        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn choose_when_otherwise(
        &mut self,
        when: &[ast::When],
        otherwise: Option<&ast::Otherwise>,
    ) -> error::SpannedResult<Bindings> {
        let first = &when.first().unwrap();
        let rest = &when[1..];

        let (condition, bindings) = self.expression(&first.test)?.atom_bindings();
        let else_expr = if !rest.is_empty() {
            self.choose_when_otherwise(rest, otherwise)?.expr()
        } else if let Some(otherwise) = otherwise {
            self.sequence_constructor(&otherwise.sequence_constructor)?
                .expr()
        } else {
            self.empty_sequence()
        };

        let expr = ir::Expr::If(ir::If {
            condition,
            then: Box::new(
                self.sequence_constructor(&first.sequence_constructor)?
                    .expr(),
            ),
            else_: Box::new(else_expr),
        });
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn for_each(&mut self, for_each: &ast::ForEach) -> error::SpannedResult<Bindings> {
        let (var_atom, bindings) = self.expression(&for_each.select)?.atom_bindings();

        let context_names = self.variables.push_context();
        let return_bindings = self.sequence_constructor(&for_each.sequence_constructor)?;
        self.variables.pop_context();
        let expr = ir::Expr::Map(ir::Map {
            context_names,
            var_atom,
            return_expr: Box::new(return_bindings.expr()),
        });

        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn iterate(&mut self, iterate: &ast::Iterate) -> error::SpannedResult<Bindings> {
        let (var_atom, bindings) = self.expression(&iterate.select)?.atom_bindings();

        let params = iterate
            .params
            .iter()
            .map(|param| -> error::SpannedResult<ir::IterateParam> {
                let param_bindings = self.select_or_sequence_constructor(param)?;
                let name = self.variables.new_var_name(&param.name);
                Ok(ir::IterateParam {
                    name,
                    value: Box::new(param_bindings.expr()),
                    type_: param.as_.clone(),
                })
            })
            .collect::<error::SpannedResult<Vec<_>>>()?;

        let (context_names, loop_name) = self.variables.push_iterate_context();
        let return_bindings = self.sequence_constructor(&iterate.sequence_constructor)?;
        let on_complete_bindings = iterate
            .on_completion
            .as_ref()
            .map(|oc| self.select_or_sequence_constructor(oc))
            .transpose()?;
        self.variables.pop_context();

        let expr = ir::Expr::Iterate(ir::Iterate {
            context_names,
            loop_name,
            var_atom,
            params,
            expr: Box::new(return_bindings.expr()),
            on_complete: on_complete_bindings.map(|x| Box::new(x.expr())),
        });

        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn break_(&mut self, break_: &ast::Break) -> error::SpannedResult<Bindings> {
        let loop_name = self
            .variables
            .current_iterate_loop_name()
            .ok_or(error::SpannedError {
                error: error::Error::XTSE3120,
                span: None,
            })?;

        let bindings = self.select_or_sequence_constructor(break_)?;
        let expr = ir::Expr::IterateBreak(ir::IterateBreak {
            loop_name,
            return_expr: Box::new(bindings.expr()),
        });
        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn next_iteration(
        &mut self,
        next_iteration: &ast::NextIteration,
    ) -> error::SpannedResult<Bindings> {
        let params = next_iteration
            .with_params
            .iter()
            .map(|param| {
                let value_bind = self.select_or_sequence_constructor(param)?;
                Ok(ir::IterateParam {
                    name: self.variables.new_var_name(&param.name),
                    value: Box::new(value_bind.expr()),
                    type_: param.as_.clone(),
                })
            })
            .collect::<error::SpannedResult<Vec<_>>>()?;

        let empty_sequence = self.empty_sequence();
        let return_expr = Bindings::new(
            self.variables
                .new_binding(empty_sequence.value, empty_sequence.span),
        );
        let let_next = ir::Expr::IterateLetNext(ir::IterateLetNext {
            params,
            return_expr: Box::new(return_expr.expr()),
        });
        let result = return_expr.bind_expr_no_span(&mut self.variables, let_next);
        Ok(result)
    }

    fn copy(&mut self, copy: &ast::Copy) -> error::SpannedResult<Bindings> {
        let (context_atom, bindings) = if let Some(select) = &copy.select {
            self.expression(select)?.atom_bindings()
        } else {
            self.variables.context_item((0..0).into())?.atom_bindings()
        };
        // copy shallow this item
        let expr = ir::Expr::CopyShallow(ir::CopyShallow {
            select: context_atom,
        });
        let (mut copy_atom, mut bindings) = bindings
            .bind_expr_no_span(&mut self.variables, expr)
            .atom_bindings();
        if let Some(type_name) = &copy.type_ {
            let xs = self.xs_type_from_eqname(type_name, copy.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: copy_atom.clone(),
                xs,
            });
            let (typed_atom, set_type_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, set_type_expr)
                .atom_bindings();
            bindings = set_type_bindings;
            copy_atom = typed_atom;
        }

        // if it is an element or document,
        // execute sequence constructor
        // TODO: work on document check
        // let _is_document_expr = self.is_document_expr(context_atom.clone());
        let is_element_expr = self.is_element_expr(copy_atom.clone());
        let (is_element_atom, bindings) = bindings
            .bind_expr_no_span(&mut self.variables, is_element_expr)
            .atom_bindings();

        let copy_expr = ir::Expr::Atom(copy_atom.clone());

        let (sequence_constructor_atom, sequence_constructor_bindings) = self
            .sequence_constructor(&copy.sequence_constructor)?
            .atom_bindings();

        let bindings = bindings.concat(sequence_constructor_bindings);

        let append = ir::Expr::XmlAppend(ir::XmlAppend {
            parent: copy_atom,
            child: sequence_constructor_atom,
        });

        let if_expr = ir::Expr::If(ir::If {
            condition: is_element_atom,
            then: Box::new(Spanned::new(append, (0..0).into())),
            else_: Box::new(Spanned::new(copy_expr, (0..0).into())),
        });

        Ok(bindings.bind_expr_no_span(&mut self.variables, if_expr))
    }

    // fn is_document_expr(&self, atom: ir::AtomS) -> ir::Expr {
    //     ir::Expr::InstanceOf(ir::InstanceOf {
    //         atom,
    //         sequence_type: xpath_ast::SequenceType::Item(xpath_ast::Item {
    //             item_type: xpath_ast::ItemType::KindTest(xpath_ast::KindTest::Document(None)),
    //             occurrence: xpath_ast::Occurrence::One,
    //         }),
    //     })
    // }

    fn is_element_expr(&self, atom: ir::AtomS) -> ir::Expr {
        ir::Expr::InstanceOf(ir::InstanceOf {
            atom,
            sequence_type: xpath_ast::SequenceType::Item(xpath_ast::Item {
                item_type: xpath_ast::ItemType::KindTest(xpath_ast::KindTest::Element(None)),
                occurrence: xpath_ast::Occurrence::One,
            }),
        })
    }

    fn copy_of(&mut self, copy_of: &ast::CopyOf) -> error::SpannedResult<Bindings> {
        let (atom, bindings) = self.expression(&copy_of.select)?.atom_bindings();
        let copy_deep_expr = ir::Expr::CopyDeep(ir::CopyDeep { select: atom });
        let (copy_atom, mut bindings) = bindings
            .bind_expr_no_span(&mut self.variables, copy_deep_expr)
            .atom_bindings();
        if let Some(type_name) = &copy_of.type_ {
            let xs = self.xs_type_from_eqname(type_name, copy_of.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: copy_atom.clone(),
                xs,
            });
            let (_typed_atom, set_type_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, set_type_expr)
                .atom_bindings();
            bindings = set_type_bindings;
        }
        Ok(bindings)
    }

    fn document(&mut self, document: &ast::Document) -> error::SpannedResult<Bindings> {
        let doc_expr = ir::Expr::XmlDocument(ir::XmlRoot {});
        let (mut doc_atom, mut bindings) =
            Bindings::new(self.variables.new_binding_no_span(doc_expr)).atom_bindings();
        if let Some(type_name) = &document.type_ {
            let xs = self.xs_type_from_eqname(type_name, document.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: doc_atom.clone(),
                xs,
            });
            let (typed_atom, set_type_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, set_type_expr)
                .atom_bindings();
            bindings = set_type_bindings;
            doc_atom = typed_atom;
        }
        if !document.sequence_constructor.is_empty() {
            let (child_atom, child_bindings) = self
                .sequence_constructor(&document.sequence_constructor)?
                .atom_bindings();
            bindings = bindings.concat(child_bindings);
            let append = ir::Expr::XmlAppend(ir::XmlAppend {
                parent: doc_atom,
                child: child_atom,
            });
            bindings = bindings.bind_expr_no_span(&mut self.variables, append);
        }
        Ok(bindings)
    }

    fn sequence(&mut self, sequence: &ast::Sequence) -> error::SpannedResult<Bindings> {
        self.select_or_sequence_constructor(sequence)
    }

    fn xml_name(&mut self, name: &ast::Name) -> error::SpannedResult<Bindings> {
        let local_name = Spanned::new(
            ir::Atom::Const(ir::Const::String(name.local_name().to_string())),
            (0..0).into(),
        );
        let namespace = self.empty_string();

        let binding = self
            .variables
            .new_binding_no_span(ir::Expr::XmlName(ir::XmlName {
                local_name,
                namespace,
            }));
        Ok(Bindings::new(binding))
    }

    fn xml_name_dynamic(
        &mut self,
        name: &ast::ValueTemplate<String>,
        namespace: &Option<ast::ValueTemplate<String>>,
    ) -> error::SpannedResult<Bindings> {
        let (localname_atom, bindings) = self.attribute_value_template(name)?.atom_bindings();
        let (namespace_atom, namespace_bindings) = if let Some(namespace) = namespace {
            self.attribute_value_template(namespace)?.atom_bindings()
        } else {
            let namespace_atom = self.empty_string();
            (namespace_atom, Bindings::empty())
        };
        let bindings = bindings.concat(namespace_bindings);
        let name = ir::Expr::XmlName(ir::XmlName {
            local_name: localname_atom,
            namespace: namespace_atom,
        });
        Ok(bindings.bind_expr_no_span(&mut self.variables, name))
    }

    fn ncname_dynamic(
        &mut self,
        name: &ast::ValueTemplate<String>,
    ) -> error::SpannedResult<Bindings> {
        self.attribute_value_template(name)
    }

    fn element(&mut self, element: &ast::Element) -> error::SpannedResult<Bindings> {
        let (name_atom, bindings) = self
            .xml_name_dynamic(&element.name, &element.namespace)?
            .atom_bindings();

        let expr = ir::Expr::XmlElement(ir::XmlElement { name: name_atom });
        let (mut element_atom, mut bindings) = bindings
            .bind_expr_no_span(&mut self.variables, expr)
            .atom_bindings();
        if let Some(type_name) = &element.type_ {
            let xs = self.xs_type_from_eqname(type_name, element.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: element_atom.clone(),
                xs,
            });
            let (typed_atom, set_type_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, set_type_expr)
                .atom_bindings();
            bindings = set_type_bindings;
            element_atom = typed_atom;
        }
        let sequence_constructor_bindings =
            self.sequence_constructor_append(element_atom, &element.sequence_constructor)?;
        Ok(bindings.concat(sequence_constructor_bindings))
    }

    fn text(&mut self, text: &ast::Text) -> error::SpannedResult<Bindings> {
        let (atom, bindings) = self
            .attribute_value_template(&text.content)?
            .atom_bindings();
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlText(ir::XmlText { value: atom }),
        ))
    }

    fn attribute(&mut self, attribute: &ast::Attribute) -> error::SpannedResult<Bindings> {
        let (name_atom, name_bindings) = self
            .xml_name_dynamic(&attribute.name, &attribute.namespace)?
            .atom_bindings();
        let (text_atom, text_bindings) = self
            .select_or_sequence_constructor_simple_content_with_separator(
                attribute,
                &attribute.separator,
            )?
            .atom_bindings();
        let bindings = name_bindings.concat(text_bindings);
        let attribute_expr = ir::Expr::XmlAttribute(ir::XmlAttribute {
            name: name_atom,
            value: text_atom,
        });
        let (attribute_atom, mut bindings) = bindings
            .bind_expr_no_span(&mut self.variables, attribute_expr)
            .atom_bindings();
        if let Some(type_name) = &attribute.type_ {
            let xs = self.xs_type_from_eqname(type_name, attribute.span)?;
            let set_type_expr = ir::Expr::XmlSetType(ir::XmlSetType {
                node: attribute_atom.clone(),
                xs,
            });
            let (_typed_atom, set_type_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, set_type_expr)
                .atom_bindings();
            bindings = set_type_bindings;
        }
        Ok(bindings)
    }

    fn namespace(&mut self, namespace: &ast::Namespace) -> error::SpannedResult<Bindings> {
        let (ncname_atom, ncname_bindings) = self.ncname_dynamic(&namespace.name)?.atom_bindings();
        let (text_atom, text_bindings) = self
            .select_or_sequence_constructor_simple_content(namespace)?
            .atom_bindings();
        let bindings = ncname_bindings.concat(text_bindings);
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlNamespace(ir::XmlNamespace {
                prefix: ncname_atom,
                namespace: text_atom,
            }),
        ))
    }

    fn comment(&mut self, comment: &ast::Comment) -> error::SpannedResult<Bindings> {
        let (atom, bindings) = self
            .select_or_sequence_constructor_simple_content(comment)?
            .atom_bindings();
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlComment(ir::XmlComment { value: atom }),
        ))
    }

    fn processing_instruction(
        &mut self,
        pi: &ast::ProcessingInstruction,
    ) -> error::SpannedResult<Bindings> {
        let (ncname_atom, ncname_bindings) = self.ncname_dynamic(&pi.name)?.atom_bindings();
        let (content_atom, content_bindings) = self
            .select_or_sequence_constructor_simple_content(pi)?
            .atom_bindings();
        let bindings = ncname_bindings.concat(content_bindings);
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlProcessingInstruction(ir::XmlProcessingInstruction {
                target: ncname_atom,
                content: content_atom,
            }),
        ))
    }

    // fn throw_error(&mut self) -> error::SpannedResult<Bindings> {
    //     let error_atom = self.error_atom();
    //     let expr = ir::Expr::FunctionCall(ir::FunctionCall {
    //         atom: Spanned::new(error_atom, (0..0).into()),
    //         args: vec![],
    //     });
    //     Ok(Bindings::new(self.variables.new_binding_no_span(expr)))
    // }

    fn expression(&mut self, expression: &ast::Expression) -> error::SpannedResult<Bindings> {
        self.xpath(&expression.xpath.0)
    }

    fn xpath(&mut self, xpath: &xee_xpath_ast::ast::ExprS) -> error::SpannedResult<Bindings> {
        let mut ir_converter = if let Some(user_functions) = &self.user_functions {
            xee_xpath_compiler::IrConverter::new_with_user_functions(
                &mut self.variables,
                self.static_context,
                user_functions.clone(),
            )
        } else {
            xee_xpath_compiler::IrConverter::new(&mut self.variables, self.static_context)
        };
        ir_converter.expr(xpath)
    }

    fn pattern_predicate(
        &mut self,
        expr: &xpath_ast::ExprS,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        let bindings = self.xpath(expr)?;
        self.variables.pop_context();
        // a predicate is a function that takes a sequence as an argument and returns
        // a boolean that is true if the sequence matches the predicate
        let name = self.variables.new_name();
        let var_atom = Spanned::new(ir::Atom::Variable(name.clone()), (0..0).into());
        let filter = ir::Expr::PatternPredicate(ir::PatternPredicate {
            context_names: context_names.clone(),
            var_atom,
            expr: Box::new(bindings.expr()),
        });
        let bindings = bindings.bind_expr(&mut self.variables, Spanned::new(filter, (0..0).into()));

        let mut params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
            },
        ];
        params.extend(self.global_param_ir_params());

        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn template_params(
        &mut self,
        params: &[ast::Param],
    ) -> error::SpannedResult<(Vec<ir::TemplateParam>, Vec<ir::Param>)> {
        let mut template_params = Vec::new();
        let mut template_ir_params = Vec::new();
        let mut seen = HashSet::new();

        for param in params {
            if param.static_ {
                return Err(error::Error::Unsupported(
                    "Static template params are not supported".to_string(),
                )
                .into());
            }
            if param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel params are not supported".to_string(),
                )
                .into());
            }
            if !seen.insert(param.name.clone()) {
                return Err(error::Error::Unsupported(
                    "Duplicate template param names are not supported".to_string(),
                )
                .into());
            }
            if param.required
                && (param.select.is_some() || !param.sequence_constructor.is_empty())
            {
                return Err(error::SpannedError {
                    error: error::Error::XTSE0010,
                    span: Some((param.span.start..param.span.end).into()),
                });
            }

            let default_expr = if let Some(select) = &param.select {
                Some(self.expression(select)?.expr())
            } else if !param.sequence_constructor.is_empty() {
                if Self::sequence_constructor_has_content(&param.sequence_constructor) {
                    Some(
                        self.sequence_constructor_document(&param.sequence_constructor)?
                            .expr(),
                    )
                } else {
                    Some(self.sequence_constructor(&param.sequence_constructor)?.expr())
                }
            } else {
                None
            };
            let var_name = self.variables.new_var_name(&param.name);
            template_params.push(ir::TemplateParam {
                name: param.name.clone(),
                var_name: var_name.clone(),
                required: param.required,
                default_expr,
                type_: param.as_.clone(),
            });
            template_ir_params.push(ir::Param {
                name: var_name,
                type_: param.as_.clone(),
            });
        }

        Ok((template_params, template_ir_params))
    }

    fn function_params(&mut self, params: &[ast::Param]) -> error::SpannedResult<Vec<ir::Param>> {
        let mut function_params = Vec::new();
        let mut seen = HashSet::new();

        for param in params {
            if param.static_ {
                return Err(error::Error::Unsupported(
                    "Static function params are not supported".to_string(),
                )
                .into());
            }
            if param.tunnel {
                return Err(error::Error::Unsupported(
                    "Tunnel function params are not supported".to_string(),
                )
                .into());
            }
            if param.select.is_some() || !param.sequence_constructor.is_empty() {
                return Err(error::Error::Unsupported(
                    "Function param defaults are not supported".to_string(),
                )
                .into());
            }
            if !seen.insert(param.name.clone()) {
                return Err(error::Error::Unsupported(
                    "Duplicate function param names are not supported".to_string(),
                )
                .into());
            }

            let var_name = self.variables.new_var_name(&param.name);
            function_params.push(ir::Param {
                name: var_name,
                type_: param.as_.clone(),
            });
        }

        Ok(function_params)
    }

    fn collect_global_params(
        &mut self,
        declarations: &[DeclarationWithImport],
    ) -> error::SpannedResult<()> {
        for declaration in declarations {
            match &declaration.declaration {
                ast::Declaration::Param(param) => {
                    if param.static_ {
                        continue;
                    }
                    if self.global_param_lookup.contains_key(&param.name) {
                        continue;
                    }
                    let var_name = self.variables.new_var_name(&param.name);
                    let index = self.global_params.len();
                    self.global_param_lookup.insert(param.name.clone(), index);
                    self.global_params.push(ir::GlobalParam {
                        name: param.name.clone(),
                        var_name,
                        required: param.required,
                        overrideable: true,
                        default_expr: None,
                    });
                }
                ast::Declaration::Variable(variable) => {
                    if variable.static_ {
                        continue;
                    }
                    if self.global_param_lookup.contains_key(&variable.name) {
                        continue;
                    }
                    let var_name = self.variables.new_var_name(&variable.name);
                    let index = self.global_params.len();
                    self.global_param_lookup.insert(variable.name.clone(), index);
                    self.global_params.push(ir::GlobalParam {
                        name: variable.name.clone(),
                        var_name,
                        required: false,
                        overrideable: false,
                        default_expr: None,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn collect_functions(
        &mut self,
        declarations: &[DeclarationWithImport],
    ) -> error::SpannedResult<()> {
        for declaration in declarations {
            if let ast::Declaration::Function(function) = &declaration.declaration {
                let arity = function.params.len();
                if arity > u8::MAX as usize {
                    return Err(error::Error::Unsupported(
                        "Function arity too large".to_string(),
                    )
                    .into());
                }
                let key = (function.name.clone(), arity as u8);
                if self.function_lookup.contains_key(&key) {
                    return Err(error::Error::Unsupported(
                        "Duplicate function declaration".to_string(),
                    )
                    .into());
                }
                let index = self.function_lookup.len();
                self.function_lookup.insert(key, index);
            }
        }
        if !self.function_lookup.is_empty() {
            self.user_functions = Some(UserFunctions::new(
                self.function_lookup.clone(),
                self.global_param_names(),
            ));
        }
        Ok(())
    }

    fn param_declaration(&mut self, param: &ast::Param) -> error::SpannedResult<()> {
        if param.static_ {
            return Ok(());
        }
        if param.required && (param.select.is_some() || !param.sequence_constructor.is_empty()) {
            return Err(error::SpannedError {
                error: error::Error::XTSE0010,
                span: Some((param.span.start..param.span.end).into()),
            });
        }
        let default_expr = if let Some(select) = &param.select {
            Some(self.expression(select)?.expr())
        } else if !param.sequence_constructor.is_empty() {
            if Self::sequence_constructor_has_content(&param.sequence_constructor) {
                Some(
                    self.sequence_constructor_document(&param.sequence_constructor)?
                        .expr(),
                )
            } else {
                Some(self.sequence_constructor(&param.sequence_constructor)?.expr())
            }
        } else {
            None
        };
        if let Some(index) = self.global_param_lookup.get(&param.name).copied() {
            if let Some(entry) = self.global_params.get_mut(index) {
                entry.required = param.required;
                entry.default_expr = default_expr;
            }
        }
        Ok(())
    }

    fn variable_declaration(&mut self, variable: &ast::Variable) -> error::SpannedResult<()> {
        if variable.static_ {
            return Ok(());
        }
        let default_expr = if let Some(select) = &variable.select {
            Some(self.expression(select)?.expr())
        } else if !variable.sequence_constructor.is_empty() {
            if Self::sequence_constructor_has_content(&variable.sequence_constructor) {
                Some(
                    self.sequence_constructor_document(&variable.sequence_constructor)?
                        .expr(),
                )
            } else {
                Some(self.sequence_constructor(&variable.sequence_constructor)?.expr())
            }
        } else {
            None
        };
        if let Some(index) = self.global_param_lookup.get(&variable.name).copied() {
            if let Some(entry) = self.global_params.get_mut(index) {
                entry.default_expr = default_expr;
            }
        }
        Ok(())
    }

    fn global_param_ir_params(&self) -> Vec<ir::Param> {
        self.global_params
            .iter()
            .map(|param| ir::Param {
                name: param.var_name.clone(),
                type_: None,
            })
            .collect()
    }

    fn global_param_names(&self) -> Vec<ir::Name> {
        self.global_params
            .iter()
            .map(|param| param.var_name.clone())
            .collect()
    }
}
