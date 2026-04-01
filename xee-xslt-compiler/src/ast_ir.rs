use ahash::{HashMap, HashMapExt, HashSetExt};
use xee_name::{Name, Namespaces, FN_NAMESPACE};

use std::collections::HashSet;
use std::path::PathBuf;
use xee_interpreter::{
    context::StaticContext,
    error,
    interpreter::{self, instruction::RaisedError},
    sequence::QNameOrString,
};
use xee_ir::{compile_xslt, ir, Bindings, Variables};
use xee_xpath_ast::{ast as xpath_ast, pattern::transform_pattern, span::Spanned};
use xee_xslt_ast::{
    ast,
    error::{AttributeError, ElementError},
    parse_transform,
};
use xot::xmlname::{NameStrInfo, OwnedName};

use crate::priority::default_priority;

struct IrConverter<'a> {
    variables: Variables,
    static_context: &'a StaticContext,
    xslt_functions: HashMap<(OwnedName, u8), OwnedName>,
}

pub fn compile(
    transform: ast::Transform,
    static_context: StaticContext,
) -> error::SpannedResult<interpreter::Program> {
    let mut ir_converter = IrConverter::new(&static_context);
    let declarations = ir_converter.transform(&transform)?;
    compile_xslt(declarations, static_context)
}

pub fn parse(
    static_context: StaticContext,
    xslt: &str,
) -> error::SpannedResult<interpreter::Program> {
    parse_with_base_dir(static_context, xslt, std::env::current_dir().ok())
}

pub fn parse_with_base_dir(
    static_context: StaticContext,
    xslt: &str,
    base_dir: Option<std::path::PathBuf>,
) -> error::SpannedResult<interpreter::Program> {
    let transform = parse_transform(xslt);
    // TODO: better error handling
    let mut transform = match transform {
        Ok(transform) => transform,
        Err(e) => {
            return Err(map_parse_error(xslt, e));
        }
    };

    // Process xsl:import and xsl:include directives
    let mut visited = HashSet::new();
    transform.declarations =
        process_imports_and_includes(transform.declarations, base_dir, &mut visited)?;

    compile(transform, static_context)
}

fn map_parse_error(xslt: &str, error: ElementError) -> error::SpannedError {
    match error {
        ElementError::Attribute(attribute_error) => match attribute_error {
            AttributeError::NotFound { span, .. } => error::SpannedError {
                error: error::Error::XTSE0010,
                span: Some((span.start..span.end).into()),
            },
            AttributeError::Unexpected { span, .. } => error::SpannedError {
                error: error::Error::XTSE0090,
                span: Some((span.start..span.end).into()),
            },
            AttributeError::Invalid { span, .. } | AttributeError::InvalidEqName { span, .. } => {
                error::SpannedError {
                    error: error::Error::XTSE0020,
                    span: Some((span.start..span.end).into()),
                }
            }
            other => error::Error::Unsupported(format!("Failed parsing XSLT: {:?}", other)).into(),
        },
        ElementError::Unexpected { span } => {
            let text = xslt.get(span.start..span.end).unwrap_or_default();
            error::Error::Unsupported(format!(
                "Failed parsing XSLT, Unexpected {} {:?}",
                text, span
            ))
            .into()
        }
        other => error::Error::Unsupported(format!("Failed parsing XSLT: {:?}", other)).into(),
    }
}

fn process_imports_and_includes(
    declarations: ast::Declarations,
    base_dir: Option<std::path::PathBuf>,
    visited: &mut HashSet<PathBuf>,
) -> error::SpannedResult<ast::Declarations> {
    let mut local_declarations = Vec::new();
    let mut imports = Vec::new(); // Collect imports in order

    for decl in declarations {
        match &decl {
            ast::Declaration::Import(import) => {
                // Load and parse the imported stylesheet
                let (imported_decls, resolved_path) =
                    load_stylesheet(&import.href.to_string(), base_dir.as_ref())?;
                if visited.contains(&resolved_path) {
                    return Err(error::Error::Unsupported(format!(
                        "Circular import detected: '{}'",
                        resolved_path.display()
                    ))
                    .into());
                }
                visited.insert(resolved_path);
                // Recursively process imports in the imported stylesheet
                let processed =
                    process_imports_and_includes(imported_decls, base_dir.clone(), visited)?;
                imports.push(processed);
            }
            ast::Declaration::Include(include) => {
                // Load and parse the included stylesheet
                let (included_decls, resolved_path) =
                    load_stylesheet(&include.href.to_string(), base_dir.as_ref())?;
                if visited.contains(&resolved_path) {
                    return Err(error::Error::Unsupported(format!(
                        "Circular include detected: '{}'",
                        resolved_path.display()
                    ))
                    .into());
                }
                visited.insert(resolved_path);
                // Recursively process imports in the included stylesheet
                let processed =
                    process_imports_and_includes(included_decls, base_dir.clone(), visited)?;
                local_declarations.extend(processed);
            }
            _ => {
                local_declarations.push(decl);
            }
        }
    }

    // Build result: imports first (lower precedence), then local declarations (higher precedence)
    // This ensures later imports override earlier imports, and local declarations override all imports
    let mut result = Vec::new();
    for import_decls in imports {
        result.extend(import_decls);
    }
    result.extend(local_declarations);
    Ok(result)
}

fn load_stylesheet(
    href: &str,
    base_dir: Option<&std::path::PathBuf>,
) -> error::SpannedResult<(ast::Declarations, PathBuf)> {
    // Resolve the file path
    let path = if let Some(base_dir) = base_dir {
        base_dir.join(href)
    } else {
        std::path::PathBuf::from(href)
    };

    let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

    // Try to read the file
    let content = std::fs::read_to_string(&path).map_err(|e| {
        error::Error::Unsupported(format!(
            "Failed to load stylesheet '{}': {}",
            path.display(),
            e
        ))
    })?;

    // Parse the stylesheet
    let transform = parse_transform(&content).map_err(|e| {
        error::Error::Unsupported(format!(
            "Failed to parse imported stylesheet '{}': {:?}",
            path.display(),
            e
        ))
    })?;

    Ok((transform.declarations, canonical))
}

impl<'a> IrConverter<'a> {
    fn new(static_context: &'a StaticContext) -> Self {
        IrConverter {
            variables: Variables::new(),
            static_context,
            xslt_functions: HashMap::new(),
        }
    }

    fn main_sequence_constructor(&mut self) -> ast::SequenceConstructor {
        vec![ast::SequenceConstructorItem::Instruction(
            ast::SequenceConstructorInstruction::ApplyTemplates(Box::new(ast::ApplyTemplates {
                // TODO: mode should be configurable from the outside somehow,
                // the XSTL test suite I think requires this.
                mode: ast::ApplyTemplatesModeValue::Unnamed,
                builtin_template_params_passthrough: true,
                select: ast::Expression {
                    xpath: xee_xpath_ast::ast::XPath::parse(
                        "/",
                        &Namespaces::default(),
                        &xee_name::VariableNames::new(),
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

    fn static_function_call_expr(
        &mut self,
        name: &str,
        namespace: &str,
        arity: u8,
        args: Vec<ir::AtomS>,
    ) -> ir::Expr {
        ir::Expr::FunctionCall(ir::FunctionCall {
            atom: Spanned::new(
                self.static_function_atom(name, namespace, arity),
                (0..0).into(),
            ),
            args,
        })
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

    fn transform(&mut self, transform: &ast::Transform) -> error::SpannedResult<ir::Declarations> {
        self.register_xslt_function_names(&transform.declarations)?;
        // Register global variable/param names early so $var references resolve.
        let global_vars = self.collect_global_variables(&transform.declarations)?;

        let main_sequence_constructor = self.main_sequence_constructor();
        let main = self.sequence_constructor_function(&main_sequence_constructor)?;
        let mut declarations = ir::Declarations::new(main);
        declarations.global_variables = global_vars;

        for declaration in &transform.declarations {
            self.declaration(&mut declarations, declaration)?;
        }

        Ok(declarations)
    }

    fn register_xslt_function_names(
        &mut self,
        declarations: &[ast::Declaration],
    ) -> error::SpannedResult<()> {
        for declaration in declarations {
            let ast::Declaration::Function(function) = declaration else {
                continue;
            };

            let arity = u8::try_from(function.params.len()).map_err(|_| {
                error::Error::Unsupported("Too many XSLT function parameters".to_string())
            })?;

            let hidden_name = OwnedName::new(
                format!("function-{}", self.xslt_functions.len()),
                "urn:xee:internal:function".to_string(),
                "xee-internal".to_string(),
            );
            self.xslt_functions
                .insert((function.name.clone(), arity), hidden_name.clone());
            self.variables.new_var_name(&hidden_name);
        }

        Ok(())
    }

    fn declaration(
        &mut self,
        declarations: &mut ir::Declarations,
        declaration: &ast::Declaration,
    ) -> error::SpannedResult<()> {
        use ast::Declaration::*;
        match declaration {
            Template(template) => self.template(declarations, template),
            Mode(mode) => self.mode(declarations, mode),
            Output(output) => self.output(declarations, output),
            // Import/Include already handled during pre-processing in parse_with_base_dir
            Import(_) | Include(_) => Ok(()),
            // These declarations are parsed but not yet compiled - skip gracefully
            // to allow stylesheets containing them to still process templates
            Function(_) | Variable(_) | Param(_) | Key(_) | StripSpace(_) | PreserveSpace(_)
            | DecimalFormat(_) | CharacterMap(_) | NamespaceAlias(_) | ImportSchema(_)
            | UsePackage(_) | GlobalContextItem(_) | Accumulator(_) => Ok(()),
        }
    }

    fn collect_global_variables(
        &mut self,
        declarations: &[ast::Declaration],
    ) -> error::SpannedResult<Vec<ir::GlobalVariable>> {
        for decl in declarations {
            match decl {
                ast::Declaration::Variable(var) => {
                    self.variables.new_var_name(&var.name);
                }
                ast::Declaration::Param(param) => {
                    self.variables.new_var_name(&param.name);
                }
                ast::Declaration::Function(function) => {
                    let arity = u8::try_from(function.params.len()).map_err(|_| {
                        error::Error::Unsupported("Too many XSLT function parameters".to_string())
                    })?;
                    let hidden_name = self
                        .xslt_functions
                        .get(&(function.name.clone(), arity))
                        .ok_or_else(|| {
                            error::Error::Unsupported("Unregistered XSLT function name".to_string())
                        })?;
                    self.variables.new_var_name(hidden_name);
                }
                _ => {}
            }
        }

        let mut globals = Vec::new();
        for decl in declarations {
            match decl {
                ast::Declaration::Variable(var) => {
                    let name = self.variables.lookup_var_name(&var.name).unwrap();
                    let expr = self.with_hidden_global_name(&var.name, |this| {
                        let context_names = this.variables.push_context();
                        let params = Self::context_params(&context_names);
                        let expr = this.global_variable_expr(
                            var.select.as_ref(),
                            &var.sequence_constructor,
                            var.as_.as_ref(),
                        )?;
                        this.variables.pop_context();
                        Ok((params, expr))
                    })?;
                    globals.push(ir::GlobalVariable {
                        name,
                        original_name: None,
                        required: false,
                        params: expr.0,
                        expr: expr.1,
                    });
                }
                ast::Declaration::Param(param) => {
                    self.validate_param(param)?;
                    let name = self.variables.lookup_var_name(&param.name).unwrap();
                    let expr = self.with_hidden_global_name(&param.name, |this| {
                        let context_names = this.variables.push_context();
                        let params = Self::context_params(&context_names);
                        let expr = this.global_param_expr(
                            param.select.as_ref(),
                            &param.sequence_constructor,
                        )?;
                        this.variables.pop_context();
                        Ok((params, expr))
                    })?;
                    globals.push(ir::GlobalVariable {
                        name,
                        original_name: Some(param.name.clone()),
                        required: param.required,
                        params: expr.0,
                        expr: expr.1,
                    });
                }
                ast::Declaration::Function(function) => {
                    let arity = u8::try_from(function.params.len()).map_err(|_| {
                        error::Error::Unsupported("Too many XSLT function parameters".to_string())
                    })?;
                    let hidden_name = self
                        .xslt_functions
                        .get(&(function.name.clone(), arity))
                        .ok_or_else(|| {
                            error::Error::Unsupported("Unregistered XSLT function name".to_string())
                        })?;
                    let name = self.variables.lookup_var_name(hidden_name).unwrap();
                    let context_names = self.variables.push_context();
                    let params = Self::context_params(&context_names);
                    let function_definition = self.xslt_function_definition(function)?;
                    self.variables.pop_context();
                    let expr = Spanned::new(
                        ir::Expr::FunctionDefinition(function_definition),
                        (function.span.start..function.span.end).into(),
                    );
                    globals.push(ir::GlobalVariable {
                        name,
                        original_name: None,
                        required: false,
                        params,
                        expr,
                    });
                }
                _ => {}
            }
        }
        Ok(globals)
    }

    fn with_hidden_global_name<T, F>(&mut self, name: &ast::Name, f: F) -> error::SpannedResult<T>
    where
        F: FnOnce(&mut Self) -> error::SpannedResult<T>,
    {
        let hidden_name = self.variables.remove_var_name_in_current_scope(name);
        let result = f(self);
        if let Some(hidden_name) = hidden_name {
            self.variables
                .insert_var_name_in_current_scope(name.clone(), hidden_name);
        }
        result
    }

    fn validate_param(&self, param: &ast::Param) -> error::SpannedResult<()> {
        if param.required && (param.select.is_some() || !param.sequence_constructor.is_empty()) {
            return Err(error::Error::XTSE0010.into());
        }
        Ok(())
    }

    fn global_variable_expr(
        &mut self,
        select: Option<&ast::Expression>,
        sequence_constructor: &ast::SequenceConstructor,
        sequence_type: Option<&xpath_ast::SequenceType>,
    ) -> error::SpannedResult<ir::ExprS> {
        let expr = if let Some(select) = select {
            self.expression(select)?.expr()
        } else if sequence_type.is_some() {
            self.sequence_constructor(sequence_constructor)?.expr()
        } else if !sequence_constructor.is_empty() {
            self.temporary_tree(sequence_constructor)?.expr()
        } else {
            Spanned::new(ir::Expr::Atom(self.empty_string()), (0..0).into())
        };
        self.convert_expr(expr, sequence_type, RaisedError::XTTE0570)
    }

    fn global_param_expr(
        &mut self,
        select: Option<&ast::Expression>,
        sequence_constructor: &ast::SequenceConstructor,
    ) -> error::SpannedResult<ir::ExprS> {
        let expr = if let Some(select) = select {
            self.expression(select)?.expr()
        } else if !sequence_constructor.is_empty() {
            self.sequence_constructor(sequence_constructor)?.expr()
        } else {
            self.empty_sequence()
        };
        Ok(expr)
    }

    fn convert_expr(
        &mut self,
        expr: ir::ExprS,
        sequence_type: Option<&xpath_ast::SequenceType>,
        error: RaisedError,
    ) -> error::SpannedResult<ir::ExprS> {
        let Some(sequence_type) = sequence_type else {
            return Ok(expr);
        };

        let binding = self.variables.new_binding(expr.value, expr.span);
        let (atom, bindings) = Bindings::new(binding).atom_bindings();
        Ok(bindings
            .bind_expr_no_span(
                &mut self.variables,
                ir::Expr::ConvertSequence(ir::ConvertSequence {
                    atom,
                    sequence_type: sequence_type.clone(),
                    error,
                }),
            )
            .expr())
    }

    fn convert_bindings(
        &mut self,
        bindings: Bindings,
        sequence_type: Option<&xpath_ast::SequenceType>,
        error: RaisedError,
    ) -> error::SpannedResult<Bindings> {
        let Some(sequence_type) = sequence_type else {
            return Ok(bindings);
        };

        let (atom, bindings) = bindings.atom_bindings();
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::ConvertSequence(ir::ConvertSequence {
                atom,
                sequence_type: sequence_type.clone(),
                error,
            }),
        ))
    }

    fn context_params(context_names: &ir::ContextNames) -> Vec<ir::Param> {
        vec![
            ir::Param {
                name: context_names.item.clone(),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.position.clone(),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.last.clone(),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
        ]
    }

    fn template(
        &mut self,
        declarations: &mut ir::Declarations,
        template: &ast::Template,
    ) -> error::SpannedResult<()> {
        for param in &template.params {
            self.validate_param(param)?;
        }
        // Determine type of template first before creating function definition
        if let Some(pattern) = &template.match_ {
            let function_definition = self.matched_template_function(template)?;
            let modes = template
                .mode
                .iter()
                .map(Self::ast_mode_value_to_ir_mode_value)
                .collect::<Vec<_>>();

            if let Some(priority) = &template.priority {
                declarations.rules.push(ir::Rule {
                    priority: *priority,
                    modes,
                    pattern: transform_pattern(&pattern.pattern, |expr| {
                        self.pattern_predicate(expr)
                    })?,
                    function_definition,
                });
                return Ok(());
            }

            let default_priorities = default_priority(&pattern.pattern).collect::<Vec<_>>();
            for (split_pattern, priority) in default_priorities {
                declarations.rules.push(ir::Rule {
                    priority,
                    modes: modes.clone(),
                    pattern: transform_pattern(&split_pattern, |expr| {
                        self.pattern_predicate(expr)
                    })?,
                    function_definition: function_definition.clone(),
                });
            }
            Ok(())
        } else if let Some(name) = &template.name {
            // Named template - compile with parameters in function signature
            let function_definition = self.template_with_params_function(template)?;
            declarations.functions.push(ir::FunctionBinding {
                name: ir::Name::new(name.local_name().to_string()),
                main: function_definition,
            });
            Ok(())
        } else {
            Err(error::Error::Unsupported(
                "Template must have either match or name attribute".to_string(),
            )
            .into())
        }
    }

    fn template_with_params_function(
        &mut self,
        template: &ast::Template,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        self.variables.push_scope();
        let param_names = self.register_template_param_names(template)?;

        let bindings = self.sequence_constructor(&template.sequence_constructor)?;
        let mut params = Self::context_params(&context_names);
        params.extend(self.template_params(template, param_names)?);
        self.variables.pop_scope();
        self.variables.pop_context();

        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn matched_template_function(
        &mut self,
        template: &ast::Template,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        self.variables.push_scope();
        let param_names = self.register_template_param_names(template)?;
        let bindings = self.sequence_constructor(&template.sequence_constructor)?;

        let mut params = Self::context_params(&context_names);
        params.extend(self.template_params(template, param_names)?);
        self.variables.pop_scope();
        self.variables.pop_context();

        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }

    fn xslt_function_definition(
        &mut self,
        function: &ast::Function,
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        self.variables.push_absent_context();
        self.variables.push_scope();

        let mut params = Vec::new();
        let mut seen_names = HashSet::new();
        for param in &function.params {
            let param_key = (
                param.name.namespace().to_string(),
                param.name.local_name().to_string(),
            );
            if !seen_names.insert(param_key) {
                return Err(error::Error::Unsupported(
                    "Duplicate XSLT function parameters are not supported".to_string(),
                )
                .into());
            }

            let name = self.variables.declare_var_name(&param.name);
            params.push(ir::Param {
                name,
                type_: param.as_.clone(),
                default: None,
                required: true,
                original_name: None,
                tunnel: false,
            });
        }

        let bindings = self.sequence_constructor(&function.sequence_constructor)?;

        self.variables.pop_scope();
        self.variables.pop_context();

        Ok(ir::FunctionDefinition {
            params,
            return_type: function.as_.clone(),
            body: Box::new(bindings.expr()),
        })
    }

    fn register_template_param_names(
        &mut self,
        template: &ast::Template,
    ) -> error::SpannedResult<Vec<(String, ir::Name)>> {
        let mut param_names = Vec::new();
        let mut seen_names = HashSet::new();
        for param in &template.params {
            let param_key = (
                param.name.namespace().to_string(),
                param.name.local_name().to_string(),
            );
            if !seen_names.insert(param_key) {
                return Err(error::SpannedError {
                    error: error::Error::XTSE0580,
                    span: Some((param.span.start..param.span.end).into()),
                });
            }
            let var_name = self.variables.declare_var_name(&param.name);
            param_names.push((param.name.local_name().to_string(), var_name));
        }
        Ok(param_names)
    }

    fn template_params(
        &mut self,
        template: &ast::Template,
        param_names: Vec<(String, ir::Name)>,
    ) -> error::SpannedResult<Vec<ir::Param>> {
        let mut params = Vec::new();
        for (original_name, runtime_name) in param_names {
            let ast_param = template
                .params
                .iter()
                .find(|param| param.name.local_name() == original_name);
            let required = ast_param.map(|param| param.required).unwrap_or(false);

            let default = if let Some(ast_param) = ast_param {
                if !ast_param.sequence_constructor.is_empty() {
                    let expr_s = self
                        .sequence_constructor(&ast_param.sequence_constructor)?
                        .expr();
                    Some(Box::new(expr_s.value))
                } else if let Some(select_expr) = &ast_param.select {
                    let expr_s = self.expression(select_expr)?.expr();
                    Some(Box::new(expr_s.value))
                } else {
                    None
                }
            } else {
                None
            };

            params.push(ir::Param {
                name: runtime_name,
                type_: template
                    .params
                    .iter()
                    .find(|param| param.name.local_name() == original_name)
                    .and_then(|param| param.as_.clone()),
                default,
                required,
                original_name: Some(original_name),
                tunnel: ast_param.map(|param| param.tunnel).unwrap_or(false),
            });
        }
        Ok(params)
    }

    fn mode(
        &mut self,
        declarations: &mut ir::Declarations,
        mode: &ast::Mode,
    ) -> error::SpannedResult<()> {
        declarations.modes.insert(mode.name.clone(), ir::Mode {});
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
    ) -> error::SpannedResult<ir::FunctionDefinition> {
        let context_names = self.variables.push_context();
        let bindings = self.sequence_constructor(sequence_constructor)?;
        self.variables.pop_context();
        let params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
        ];
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
        self.variables.push_scope();
        let result = self.sequence_constructor_in_scope(sequence_constructor);
        self.variables.pop_scope();
        result
    }

    fn sequence_constructor_in_scope(
        &mut self,
        sequence_constructor: &[ast::SequenceConstructorItem],
    ) -> error::SpannedResult<Bindings> {
        let mut items = sequence_constructor.iter();
        let left = items.next();
        if let Some(left) = left {
            if let Some((name, var_bindings)) = self.variable(left)? {
                let expr = ir::Expr::Let(ir::Let {
                    name,
                    var_expr: Box::new(var_bindings.expr()),
                    return_expr: Box::new(
                        self.sequence_constructor_in_scope(items.as_slice())?.expr(),
                    ),
                });
                return Ok(Bindings::new(
                    self.variables.new_binding(expr, (0..0).into()),
                ));
            }

            let mut left_bindings = self.sequence_constructor_item(left)?;
            if items.as_slice().is_empty() {
                return Ok(left_bindings);
            }
            let mut right_bindings = self.sequence_constructor_in_scope(items.as_slice())?;
            let expr = ir::Expr::Binary(ir::Binary {
                left: left_bindings.atom(),
                op: ir::BinaryOperator::Comma,
                right: right_bindings.atom(),
            });
            let binding = self.variables.new_binding_no_span(expr);
            Ok(left_bindings.concat(right_bindings).bind(binding))
        } else {
            let empty_sequence = self.empty_sequence();
            Ok(Bindings::new(
                self.variables
                    .new_binding(empty_sequence.value, empty_sequence.span),
            ))
        }
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
            CallTemplate(call_template) => self.call_template(call_template),
            ValueOf(value_of) => self.value_of(value_of),
            If(if_) => self.if_(if_),
            Choose(choose) => self.choose(choose),
            ForEach(for_each) => self.for_each(for_each),
            ForEachGroup(for_each_group) => self.for_each_group(for_each_group),
            Iterate(iterate) => self.iterate(iterate),
            NextIteration(next_iteration) => self.next_iteration(next_iteration),
            NextMatch(next_match) => self.next_match(next_match),
            Break(break_) => self.break_(break_),
            Copy(copy) => self.copy(copy),
            CopyOf(copy_of) => self.copy_of(copy_of),
            Message(message) => self.message(message),
            Sequence(sequence) => self.sequence(sequence),
            Element(element) => self.element(element),
            Text(text) => self.text(text),
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

    fn message(&mut self, message: &ast::Message) -> error::SpannedResult<Bindings> {
        let empty_sequence = self.empty_sequence();
        let message_bindings = if let Some(select) = &message.select {
            self.expression(select)?
        } else if !message.sequence_constructor.is_empty() {
            self.sequence_constructor(&message.sequence_constructor)?
        } else {
            Bindings::new(
                self.variables
                    .new_binding_no_span(empty_sequence.value.clone()),
            )
        };

        Ok(message_bindings.bind_expr(&mut self.variables, empty_sequence))
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
        for namespace in &element_node.namespaces {
            let prefix_atom = Spanned::new(
                ir::Atom::Const(ir::Const::String(namespace.prefix.clone())),
                (0..0).into(),
            );
            let namespace_atom = Spanned::new(
                ir::Atom::Const(ir::Const::String(namespace.uri.clone())),
                (0..0).into(),
            );
            let namespace_expr = ir::Expr::XmlNamespace(ir::XmlNamespace {
                prefix: prefix_atom,
                namespace: namespace_atom,
            });
            let (namespace_atom, namespace_bindings) = Bindings::empty()
                .bind_expr_no_span(&mut self.variables, namespace_expr)
                .atom_bindings();
            let append_expr = ir::Expr::XmlAppend(ir::XmlAppend {
                parent: element_atom.clone(),
                child: namespace_atom,
            });
            let append_bindings =
                namespace_bindings.bind_expr_no_span(&mut self.variables, append_expr);
            bindings = bindings.concat(append_bindings);
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
        let mut sorts = Vec::new();
        let mut params = Vec::new();
        let mut param_bindings = Bindings::empty();

        for content in &apply_templates.content {
            match content {
                ast::ApplyTemplatesContent::Sort(sort) => sorts.push(sort),
                ast::ApplyTemplatesContent::WithParam(with_param) => {
                    let (select_atom, select_bindings) = if let Some(select) = &with_param.select {
                        let (atom, bindings) = self.expression(select)?.atom_bindings();
                        (Some(atom), bindings)
                    } else {
                        let sc_bindings =
                            self.sequence_constructor(&with_param.sequence_constructor)?;
                        let (atom, bindings) = sc_bindings.atom_bindings();
                        (Some(atom), bindings)
                    };

                    param_bindings = param_bindings.concat(select_bindings);

                    params.push(ir::WithParam {
                        name: ir::Name::new(with_param.name.local_name().to_string()),
                        select: select_atom,
                        sequence_constructor: None,
                        tunnel: with_param.tunnel,
                    });
                }
            }
        }

        let (select_atom, sort_bindings) = self.apply_template_sorts(select_atom, &sorts)?;

        let mode = match &apply_templates.mode {
            ast::ApplyTemplatesModeValue::EqName(name) => {
                ir::ApplyTemplatesModeValue::Named(name.clone())
            }
            ast::ApplyTemplatesModeValue::Unnamed => ir::ApplyTemplatesModeValue::Unnamed,
            ast::ApplyTemplatesModeValue::Current => ir::ApplyTemplatesModeValue::Current,
        };

        let bindings = bindings.concat(sort_bindings).concat(param_bindings);

        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::ApplyTemplates(ir::ApplyTemplates {
                mode,
                select: select_atom,
                builtin_template_params_passthrough: apply_templates
                    .builtin_template_params_passthrough,
                params,
            }),
        ))
    }

    fn apply_imports(
        &mut self,
        apply_imports: &ast::ApplyImports,
    ) -> error::SpannedResult<Bindings> {
        self.continue_template(apply_imports.with_params.iter())
    }

    fn next_match(&mut self, next_match: &ast::NextMatch) -> error::SpannedResult<Bindings> {
        self.continue_template(
            next_match
                .content
                .iter()
                .filter_map(|content| match content {
                    ast::NextMatchContent::WithParam(with_param) => Some(with_param),
                    ast::NextMatchContent::Fallback(_) => None,
                }),
        )
    }

    fn continue_template<'b>(
        &mut self,
        with_params: impl Iterator<Item = &'b ast::WithParam>,
    ) -> error::SpannedResult<Bindings> {
        let mut params = Vec::new();
        let mut param_bindings = Bindings::empty();

        for with_param in with_params {
            let (select_atom, select_bindings) = if let Some(select) = &with_param.select {
                let (atom, bindings) = self.expression(select)?.atom_bindings();
                (Some(atom), bindings)
            } else {
                let sc_bindings = self.sequence_constructor(&with_param.sequence_constructor)?;
                let (atom, bindings) = sc_bindings.atom_bindings();
                (Some(atom), bindings)
            };

            param_bindings = param_bindings.concat(select_bindings);

            params.push(ir::WithParam {
                name: ir::Name::new(with_param.name.local_name().to_string()),
                select: select_atom,
                sequence_constructor: None,
                tunnel: with_param.tunnel,
            });
        }

        Ok(param_bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::ContinueTemplate(ir::ContinueTemplate { params }),
        ))
    }

    fn apply_template_sorts(
        &mut self,
        select_atom: ir::AtomS,
        sorts: &[&ast::Sort],
    ) -> error::SpannedResult<(ir::AtomS, Bindings)> {
        let mut current_atom = select_atom;
        let mut bindings = Bindings::empty();

        for sort in sorts.iter().rev() {
            self.ensure_supported_sort(sort)?;

            let (key_atom, key_bindings) = self.sort_key_function(sort)?;
            let (collation_atom, collation_bindings) = self.sort_collation_atom(sort)?;

            bindings = bindings.concat(key_bindings).concat(collation_bindings);
            let sort_expr = self.static_function_call_expr(
                "sort",
                FN_NAMESPACE,
                3,
                vec![current_atom.clone(), collation_atom, key_atom],
            );
            let (sorted_atom, sorted_bindings) = bindings
                .bind_expr_no_span(&mut self.variables, sort_expr)
                .atom_bindings();
            bindings = sorted_bindings;
            current_atom = sorted_atom;

            if self.sort_is_descending(sort)? {
                let reverse_expr = self.static_function_call_expr(
                    "reverse",
                    FN_NAMESPACE,
                    1,
                    vec![current_atom.clone()],
                );
                let (reversed_atom, reversed_bindings) = bindings
                    .bind_expr_no_span(&mut self.variables, reverse_expr)
                    .atom_bindings();
                bindings = reversed_bindings;
                current_atom = reversed_atom;
            }
        }

        Ok((current_atom, bindings))
    }

    fn sort_key_function(
        &mut self,
        sort: &ast::Sort,
    ) -> error::SpannedResult<(ir::AtomS, Bindings)> {
        let param_name = self.variables.new_name();
        let context_names = self.variables.push_context();
        let return_bindings = self.sort_key_return_bindings(sort)?;
        self.variables.pop_context();

        let body = ir::Expr::Map(ir::Map {
            context_names,
            var_atom: Spanned::new(ir::Atom::Variable(param_name.clone()), (0..0).into()),
            return_expr: Box::new(return_bindings.expr()),
        });
        let function = ir::Expr::FunctionDefinition(ir::FunctionDefinition {
            params: vec![ir::Param {
                name: param_name,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            }],
            return_type: None,
            body: Box::new(Spanned::new(body, (0..0).into())),
        });

        let bindings = Bindings::empty().bind_expr_no_span(&mut self.variables, function);
        Ok(bindings.atom_bindings())
    }

    fn sort_key_return_bindings(&mut self, sort: &ast::Sort) -> error::SpannedResult<Bindings> {
        let key_bindings = if let Some(select) = &sort.select {
            self.expression(select)?
        } else if !sort.sequence_constructor.is_empty() {
            self.sequence_constructor(&sort.sequence_constructor)?
        } else {
            self.variables.context_item((0..0).into())?
        };
        self.atomized_key_bindings(key_bindings, self.sort_data_type(sort)?)
    }

    fn atomized_key_bindings(
        &mut self,
        key_bindings: Bindings,
        data_type: SortDataType,
    ) -> error::SpannedResult<Bindings> {
        let (key_atom, bindings) = key_bindings.atom_bindings();
        let atomized_expr = self.static_function_call_expr("data", FN_NAMESPACE, 1, vec![key_atom]);
        let bindings = bindings.bind_expr_no_span(&mut self.variables, atomized_expr);

        match data_type {
            SortDataType::Text => Ok(bindings),
            SortDataType::Number => {
                let (atomized_atom, bindings) = bindings.atom_bindings();
                let number_expr =
                    self.static_function_call_expr("number", FN_NAMESPACE, 1, vec![atomized_atom]);
                Ok(bindings.bind_expr_no_span(&mut self.variables, number_expr))
            }
        }
    }

    fn sort_collation_atom(
        &mut self,
        sort: &ast::Sort,
    ) -> error::SpannedResult<(ir::AtomS, Bindings)> {
        if let Some(collation) = &sort.collation {
            Ok(self.attribute_value_template(collation)?.atom_bindings())
        } else {
            Ok((
                Spanned::new(ir::Atom::Const(ir::Const::EmptySequence), (0..0).into()),
                Bindings::empty(),
            ))
        }
    }

    fn ensure_supported_sort(&self, sort: &ast::Sort) -> error::SpannedResult<()> {
        if sort.lang.is_some() {
            return Err(error::Error::Unsupported(String::from(
                "xsl:sort lang is not supported yet",
            ))
            .into());
        }
        if sort.case_order.is_some() {
            return Err(error::Error::Unsupported(String::from(
                "xsl:sort case-order is not supported yet",
            ))
            .into());
        }
        Ok(())
    }

    fn sort_is_descending(&self, sort: &ast::Sort) -> error::SpannedResult<bool> {
        let Some(order) = &sort.order else {
            return Ok(false);
        };
        match self
            .literal_value_template(order, "xsl:sort order")?
            .as_deref()
        {
            Some("ascending") => Ok(false),
            Some("descending") => Ok(true),
            Some(value) => Err(error::Error::Unsupported(format!(
                "xsl:sort order value {:?} is not supported yet",
                value
            ))
            .into()),
            None => Ok(false),
        }
    }

    fn sort_data_type(&self, sort: &ast::Sort) -> error::SpannedResult<SortDataType> {
        let Some(data_type) = &sort.data_type else {
            return Ok(SortDataType::Text);
        };
        match self
            .literal_value_template(data_type, "xsl:sort data-type")?
            .as_deref()
        {
            Some("text") => Ok(SortDataType::Text),
            Some("number") => Ok(SortDataType::Number),
            Some(value) => Err(error::Error::Unsupported(format!(
                "xsl:sort data-type value {:?} is not supported yet",
                value
            ))
            .into()),
            None => Ok(SortDataType::Text),
        }
    }

    fn literal_value_template<V>(
        &self,
        value_template: &ast::ValueTemplate<V>,
        attribute: &str,
    ) -> error::SpannedResult<Option<String>>
    where
        V: Clone + PartialEq + Eq,
    {
        let mut value = String::new();
        for item in &value_template.template {
            match item {
                ast::ValueTemplateItem::String { text, .. } => value.push_str(text),
                ast::ValueTemplateItem::Curly { c } => value.push(*c),
                ast::ValueTemplateItem::Value { .. } => {
                    return Err(error::Error::Unsupported(format!(
                        "{} AVTs are not supported yet",
                        attribute
                    ))
                    .into())
                }
            }
        }
        Ok(Some(value))
    }

    fn call_template(
        &mut self,
        call_template: &ast::CallTemplate,
    ) -> error::SpannedResult<Bindings> {
        // Compile the with-params for the template invocation
        let mut params = Vec::new();
        let mut param_bindings = Bindings::empty();

        for with_param in &call_template.with_params {
            let (select_atom, select_bindings) = if let Some(select) = &with_param.select {
                let (atom, bindings) = self.expression(select)?.atom_bindings();
                (Some(atom), bindings)
            } else {
                let sc_bindings = self.sequence_constructor(&with_param.sequence_constructor)?;
                let (atom, bindings) = sc_bindings.atom_bindings();
                (Some(atom), bindings)
            };

            param_bindings = param_bindings.concat(select_bindings);

            params.push(ir::WithParam {
                name: ir::Name::new(with_param.name.local_name().to_string()),
                select: select_atom,
                sequence_constructor: None, // Already flattened into select_atom above
                tunnel: with_param.tunnel,
            });
        }

        let call_template_expr = ir::Expr::CallTemplate(ir::CallTemplate {
            name: ir::Name::new(call_template.name.local_name().to_string()),
            context: self.variables.current_context_names(),
            backwards_compatible: call_template.backwards_compatible,
            params,
        });

        Ok(param_bindings.bind_expr_no_span(&mut self.variables, call_template_expr))
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
            let var_bindings = if let Some(select) = &variable.select {
                self.expression(select)?
            } else if variable.as_.is_some() {
                self.sequence_constructor(&variable.sequence_constructor)?
            } else if !variable.sequence_constructor.is_empty() {
                self.temporary_tree(&variable.sequence_constructor)?
            } else {
                let empty_string = ir::Expr::Atom(self.empty_string());
                Bindings::empty().bind_expr_no_span(&mut self.variables, empty_string)
            };
            let var_bindings =
                self.convert_bindings(var_bindings, variable.as_.as_ref(), RaisedError::XTTE0570)?;
            let name = self.variables.declare_var_name(&variable.name);
            Ok(Some((name, var_bindings)))
        } else {
            Ok(None)
        }
    }

    fn temporary_tree(
        &mut self,
        sequence_constructor: &ast::SequenceConstructor,
    ) -> error::SpannedResult<Bindings> {
        let (document_atom, document_bindings) = Bindings::empty()
            .bind_expr_no_span(&mut self.variables, ir::Expr::XmlDocument(ir::XmlRoot {}))
            .atom_bindings();

        if sequence_constructor.is_empty() {
            return Ok(document_bindings);
        }

        let (child_atom, child_bindings) = self
            .sequence_constructor(sequence_constructor)?
            .atom_bindings();
        let append_expr = ir::Expr::XmlAppend(ir::XmlAppend {
            parent: document_atom,
            child: child_atom,
        });
        Ok(document_bindings
            .concat(child_bindings)
            .bind_expr_no_span(&mut self.variables, append_expr))
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
        let (select_atom, bindings) = self.expression(&for_each.select)?.atom_bindings();
        let sort_refs = for_each.sort.iter().collect::<Vec<_>>();
        let (var_atom, sort_bindings) = self.apply_template_sorts(select_atom, &sort_refs)?;
        let bindings = bindings.concat(sort_bindings);

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

    fn for_each_group(
        &mut self,
        for_each_group: &ast::ForEachGroup,
    ) -> error::SpannedResult<Bindings> {
        if for_each_group.group_adjacent.is_some()
            || for_each_group.group_starting_with.is_some()
            || for_each_group.group_ending_with.is_some()
            || for_each_group.composite
            || for_each_group.collation.is_some()
            || !for_each_group.sort.is_empty()
        {
            return Err(error::Error::Unsupported(format!(
                "Instruction not supported: {:?}",
                for_each_group
            ))
            .into());
        }

        let group_by = for_each_group.group_by.as_ref().ok_or_else(|| {
            error::Error::Unsupported(format!("Instruction not supported: {:?}", for_each_group))
        })?;

        let (select_atom, bindings) = self.expression(&for_each_group.select)?.atom_bindings();
        let (key_function_atom, key_function_bindings) = self.group_key_function(group_by)?;
        let grouped_expr = self.static_function_call_expr(
            "group-by-first",
            FN_NAMESPACE,
            2,
            vec![select_atom, key_function_atom],
        );
        let (grouped_atom, group_bindings) = key_function_bindings
            .bind_expr_no_span(&mut self.variables, grouped_expr)
            .atom_bindings();
        let bindings = bindings.concat(group_bindings);

        let context_names = self.variables.push_context();
        let return_bindings = self.sequence_constructor(&for_each_group.sequence_constructor)?;
        self.variables.pop_context();
        let expr = ir::Expr::Map(ir::Map {
            context_names,
            var_atom: grouped_atom,
            return_expr: Box::new(return_bindings.expr()),
        });

        Ok(bindings.bind_expr_no_span(&mut self.variables, expr))
    }

    fn group_key_function(
        &mut self,
        group_by: &ast::Expression,
    ) -> error::SpannedResult<(ir::AtomS, Bindings)> {
        let param_name = self.variables.new_name();
        let context_names = self.variables.push_context();
        let bindings = self.expression(group_by)?;
        let bindings = self.atomized_key_bindings(bindings, SortDataType::Text)?;
        self.variables.pop_context();

        let body = ir::Expr::Map(ir::Map {
            context_names,
            var_atom: Spanned::new(ir::Atom::Variable(param_name.clone()), (0..0).into()),
            return_expr: Box::new(bindings.expr()),
        });

        let function_definition = ir::FunctionDefinition {
            params: vec![ir::Param {
                name: param_name,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            }],
            return_type: None,
            body: Box::new(Spanned::new(body, (0..0).into())),
        };

        let function_expr = Bindings::empty().bind_expr_no_span(
            &mut self.variables,
            ir::Expr::FunctionDefinition(function_definition),
        );
        Ok(function_expr.atom_bindings())
    }

    fn iterate(&mut self, iterate: &ast::Iterate) -> error::SpannedResult<Bindings> {
        let (var_atom, bindings) = self.expression(&iterate.select)?.atom_bindings();

        let params = iterate
            .params
            .iter()
            .map(|param| -> error::SpannedResult<ir::IterateParam> {
                let param_bindings = self.select_or_sequence_constructor(param)?;
                let name = self.variables.declare_var_name(&param.name);
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
        let (copy_atom, bindings) = bindings
            .bind_expr_no_span(&mut self.variables, expr)
            .atom_bindings();

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
        Ok(bindings.bind_expr_no_span(&mut self.variables, copy_deep_expr))
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
        let (element_atom, bindings) = bindings
            .bind_expr_no_span(&mut self.variables, expr)
            .atom_bindings();
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
        Ok(bindings.bind_expr_no_span(
            &mut self.variables,
            ir::Expr::XmlAttribute(ir::XmlAttribute {
                name: name_atom,
                value: text_atom,
            }),
        ))
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
        let mut rewritten_xpath = xpath.clone();
        self.rewrite_user_function_references_expr(&mut rewritten_xpath);
        let mut ir_converter =
            xee_xpath_compiler::IrConverter::new(&mut self.variables, self.static_context);
        ir_converter.expr(&rewritten_xpath)
    }

    fn lookup_xslt_function_var_name(&self, name: &OwnedName, arity: u8) -> Option<OwnedName> {
        self.xslt_functions.get(&(name.clone(), arity)).cloned()
    }

    fn rewrite_user_function_references_expr(&self, expr: &mut xpath_ast::ExprS) {
        for expr_single in &mut expr.value.0 {
            self.rewrite_user_function_references_expr_single(expr_single);
        }
    }

    fn rewrite_user_function_references_expr_or_empty(&self, expr: &mut xpath_ast::ExprOrEmptyS) {
        if let Some(expr) = &mut expr.value {
            for expr_single in &mut expr.0 {
                self.rewrite_user_function_references_expr_single(expr_single);
            }
        }
    }

    fn rewrite_user_function_references_expr_single(&self, expr: &mut xpath_ast::ExprSingleS) {
        match &mut expr.value {
            xpath_ast::ExprSingle::Path(path_expr) => {
                self.rewrite_user_function_references_path_expr(path_expr);
            }
            xpath_ast::ExprSingle::Apply(apply_expr) => {
                self.rewrite_user_function_references_path_expr(&mut apply_expr.path_expr);
                if let xpath_ast::ApplyOperator::SimpleMap(path_exprs) = &mut apply_expr.operator {
                    for path_expr in path_exprs {
                        self.rewrite_user_function_references_path_expr(path_expr);
                    }
                }
            }
            xpath_ast::ExprSingle::Let(let_expr) => {
                self.rewrite_user_function_references_expr_single(&mut let_expr.var_expr);
                self.rewrite_user_function_references_expr_single(&mut let_expr.return_expr);
            }
            xpath_ast::ExprSingle::If(if_expr) => {
                self.rewrite_user_function_references_expr(&mut if_expr.condition);
                self.rewrite_user_function_references_expr_single(&mut if_expr.then);
                self.rewrite_user_function_references_expr_single(&mut if_expr.else_);
            }
            xpath_ast::ExprSingle::Binary(binary_expr) => {
                self.rewrite_user_function_references_path_expr(&mut binary_expr.left);
                self.rewrite_user_function_references_path_expr(&mut binary_expr.right);
            }
            xpath_ast::ExprSingle::For(for_expr) => {
                self.rewrite_user_function_references_expr_single(&mut for_expr.var_expr);
                self.rewrite_user_function_references_expr_single(&mut for_expr.return_expr);
            }
            xpath_ast::ExprSingle::Quantified(quantified_expr) => {
                self.rewrite_user_function_references_expr_single(&mut quantified_expr.var_expr);
                self.rewrite_user_function_references_expr_single(
                    &mut quantified_expr.satisfies_expr,
                );
            }
        }
    }

    fn rewrite_user_function_references_path_expr(&self, path_expr: &mut xpath_ast::PathExpr) {
        for step in &mut path_expr.steps {
            self.rewrite_user_function_references_step_expr(step);
        }
    }

    fn rewrite_user_function_references_step_expr(&self, step: &mut xpath_ast::StepExprS) {
        match &mut step.value {
            xpath_ast::StepExpr::PrimaryExpr(primary) => {
                let extra_postfixes = self.rewrite_user_function_references_primary_expr(primary);
                if !extra_postfixes.is_empty() {
                    step.value = xpath_ast::StepExpr::PostfixExpr {
                        primary: primary.clone(),
                        postfixes: extra_postfixes,
                    };
                }
            }
            xpath_ast::StepExpr::PostfixExpr { primary, postfixes } => {
                let extra_postfixes = self.rewrite_user_function_references_primary_expr(primary);
                for postfix in postfixes.iter_mut() {
                    self.rewrite_user_function_references_postfix(postfix);
                }
                if !extra_postfixes.is_empty() {
                    let mut new_postfixes = extra_postfixes;
                    new_postfixes.append(postfixes);
                    *postfixes = new_postfixes;
                }
            }
            xpath_ast::StepExpr::AxisStep(axis_step) => {
                for predicate in &mut axis_step.predicates {
                    self.rewrite_user_function_references_expr(predicate);
                }
            }
        }
    }

    fn rewrite_user_function_references_primary_expr(
        &self,
        primary: &mut xpath_ast::PrimaryExprS,
    ) -> Vec<xpath_ast::Postfix> {
        match &mut primary.value {
            xpath_ast::PrimaryExpr::FunctionCall(function_call) => {
                for argument in &mut function_call.arguments {
                    self.rewrite_user_function_references_expr_single(argument);
                }

                let arity = match u8::try_from(function_call.arguments.len()) {
                    Ok(arity) => arity,
                    Err(_) => return Vec::new(),
                };

                if let Some(hidden_name) =
                    self.lookup_xslt_function_var_name(&function_call.name.value, arity)
                {
                    let arguments = function_call.arguments.clone();
                    primary.value = xpath_ast::PrimaryExpr::VarRef(hidden_name);
                    vec![xpath_ast::Postfix::ArgumentList(arguments)]
                } else {
                    Vec::new()
                }
            }
            xpath_ast::PrimaryExpr::NamedFunctionRef(named_function_ref) => {
                if let Some(hidden_name) = self.lookup_xslt_function_var_name(
                    &named_function_ref.name.value,
                    named_function_ref.arity,
                ) {
                    primary.value = xpath_ast::PrimaryExpr::VarRef(hidden_name);
                }
                Vec::new()
            }
            xpath_ast::PrimaryExpr::Expr(expr) => {
                self.rewrite_user_function_references_expr_or_empty(expr);
                Vec::new()
            }
            xpath_ast::PrimaryExpr::InlineFunction(inline_function) => {
                self.rewrite_user_function_references_expr_or_empty(&mut inline_function.body);
                Vec::new()
            }
            xpath_ast::PrimaryExpr::MapConstructor(map_constructor) => {
                for entry in &mut map_constructor.entries {
                    self.rewrite_user_function_references_expr_single(&mut entry.key);
                    self.rewrite_user_function_references_expr_single(&mut entry.value);
                }
                Vec::new()
            }
            xpath_ast::PrimaryExpr::ArrayConstructor(array_constructor) => {
                match array_constructor {
                    xpath_ast::ArrayConstructor::Square(expr) => {
                        self.rewrite_user_function_references_expr(expr);
                    }
                    xpath_ast::ArrayConstructor::Curly(expr) => {
                        self.rewrite_user_function_references_expr_or_empty(expr);
                    }
                }
                Vec::new()
            }
            xpath_ast::PrimaryExpr::UnaryLookup(key_specifier) => {
                self.rewrite_user_function_references_key_specifier(key_specifier);
                Vec::new()
            }
            xpath_ast::PrimaryExpr::Literal(_)
            | xpath_ast::PrimaryExpr::VarRef(_)
            | xpath_ast::PrimaryExpr::ContextItem => Vec::new(),
        }
    }

    fn rewrite_user_function_references_postfix(&self, postfix: &mut xpath_ast::Postfix) {
        match postfix {
            xpath_ast::Postfix::Predicate(expr) => {
                self.rewrite_user_function_references_expr(expr);
            }
            xpath_ast::Postfix::ArgumentList(arguments) => {
                for argument in arguments {
                    self.rewrite_user_function_references_expr_single(argument);
                }
            }
            xpath_ast::Postfix::Lookup(key_specifier) => {
                self.rewrite_user_function_references_key_specifier(key_specifier);
            }
        }
    }

    fn rewrite_user_function_references_key_specifier(
        &self,
        key_specifier: &mut xpath_ast::KeySpecifier,
    ) {
        if let xpath_ast::KeySpecifier::Expr(expr) = key_specifier {
            self.rewrite_user_function_references_expr_or_empty(expr);
        }
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

        let params = vec![
            ir::Param {
                name: context_names.item,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.position,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: context_names.last,
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
        ];

        Ok(ir::FunctionDefinition {
            params,
            return_type: None,
            body: Box::new(bindings.expr()),
        })
    }
}

enum SortDataType {
    Text,
    Number,
}
