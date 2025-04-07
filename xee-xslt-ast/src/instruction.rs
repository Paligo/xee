use std::sync::OnceLock;

use xot::Node;

use crate::ast_core::{self as ast};
use crate::attributes::Attributes;
use crate::combinator::{one, NodeParser};
use crate::content::Content;
use crate::element::{by_element, children, instruction, sequence_constructor, ContentParseLock};
use crate::error::ElementError as Error;
use crate::state::State;

type Result<V> = std::result::Result<V, Error>;

pub(crate) trait InstructionParser: Sized {
    fn validate(&self, _node: Node, _state: &State) -> Result<()> {
        Ok(())
    }

    fn should_be_empty() -> bool {
        false
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self>;

    fn parse_and_validate(attributes: &Attributes) -> Result<Self> {
        let content = &attributes.content;
        if Self::should_be_empty() {
            if let Some(child) = content.state.xot.first_child(content.node) {
                return Err(Error::Unexpected {
                    span: content.state.span(child).ok_or(Error::Internal)?,
                });
            }
        }

        let node = content.node;
        let state = content.state;
        let item = Self::parse(&attributes.content, attributes)?;
        item.validate(node, state)?;
        attributes.validate_unseen()?;
        Ok(item)
    }
}

pub(crate) trait SequenceConstructorParser:
    InstructionParser + Into<ast::SequenceConstructorItem>
{
    fn parse_sequence_constructor_item(
        attributes: &Attributes,
    ) -> Result<ast::SequenceConstructorItem> {
        let item = Self::parse_and_validate(attributes)?;
        Ok(item.into())
    }
}

impl<T> SequenceConstructorParser for T where
    T: InstructionParser + Into<ast::SequenceConstructorItem>
{
}

pub(crate) trait DeclarationParser: InstructionParser + Into<ast::Declaration> {
    fn parse_declaration(attributes: &Attributes) -> Result<ast::Declaration> {
        let item = Self::parse_and_validate(attributes)?;
        Ok(item.into())
    }
}

impl<T> DeclarationParser for T where T: InstructionParser + Into<ast::Declaration> {}

pub(crate) trait OverrideContentParser:
    InstructionParser + Into<ast::OverrideContent>
{
    fn parse_override_content(attributes: &Attributes) -> Result<ast::OverrideContent> {
        let item = Self::parse_and_validate(attributes)?;
        Ok(item.into())
    }
}

impl<T> OverrideContentParser for T where T: InstructionParser + Into<ast::OverrideContent> {}

impl InstructionParser for ast::SequenceConstructorItem {
    fn parse(content: &Content, attributes: &Attributes) -> Result<ast::SequenceConstructorItem> {
        let state = content.state;
        let name = state
            .names
            .sequence_constructor_name(attributes.element.name());

        if let Some(name) = name {
            name.parse(attributes)
        } else {
            let ns = state.xot.namespace_for_name(attributes.element.name());
            if ns == state.names.xsl_ns {
                // we have an unknown xsl instruction, fail with error
                Err(Error::Unexpected {
                    span: attributes.content.span()?,
                })
            } else {
                // we parse the literal element
                ast::ElementNode::parse_sequence_constructor_item(attributes)
            }
        }
    }
}

impl InstructionParser for ast::Declaration {
    fn parse(content: &Content, attributes: &Attributes) -> Result<ast::Declaration> {
        let name = content
            .state
            .names
            .declaration_name(attributes.element.name());

        if let Some(name) = name {
            name.parse(attributes)
        } else {
            Err(Error::Unexpected {
                span: content.span()?,
            })
        }
    }
}

impl InstructionParser for ast::ElementNode {
    fn parse(content: &Content, attributes: &Attributes) -> Result<ast::ElementNode> {
        let mut element_attributes = Vec::new();
        for key in content.state.xot.attributes(content.node).keys() {
            let name = content.state.xot.name_ref(key, content.node)?;
            // if any name is in the xsl namespace, we skip it
            if name.namespace_id() == content.state.names.xsl_ns {
                continue;
            }
            let value = attributes.required(key, attributes.value_template(attributes.string()))?;
            element_attributes.push((name.to_owned(), value));
        }

        let name = content
            .state
            .xot
            .name_ref(attributes.element.name(), content.node)?;
        Ok(ast::ElementNode {
            name: name.to_owned(),
            attributes: element_attributes,
            span: content.span()?,
            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Accept {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Accept {
            component: attributes.required(names.component, attributes.component())?,
            names: attributes.required(names.names, attributes.tokens())?,
            visibility: attributes
                .required(names.visibility, attributes.visibility_with_hidden())?,

            span: content.span()?,
        })
    }
}

static ACCUMULATOR_CONTENT: ContentParseLock<Vec<ast::AccumulatorRule>> = OnceLock::new();

impl InstructionParser for ast::Accumulator {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = ACCUMULATOR_CONTENT
            .get_or_init(|| children(instruction(names.xsl_accumulator_rule).many()));

        Ok(ast::Accumulator {
            name: attributes.required(names.name, attributes.eqname())?,
            initial_value: attributes.required(names.initial_value, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            streamable: attributes.boolean_with_default(names.streamable, false)?,

            span: content.span()?,

            rules: parse(content)?,
        })
    }
}

impl InstructionParser for ast::AccumulatorRule {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::AccumulatorRule {
            match_: attributes.required(names.match_, attributes.pattern())?,
            phase: attributes.optional(names.phase, attributes.phase())?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

type AnalyzeStringContent = (
    (
        Option<ast::MatchingSubstring>,
        Option<ast::NonMatchingSubstring>,
    ),
    Vec<ast::Fallback>,
);

static ANALYZE_STRING: ContentParseLock<AnalyzeStringContent> = OnceLock::new();

impl InstructionParser for ast::AnalyzeString {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let span = content.span()?;

        let select = attributes.required(names.select, attributes.xpath())?;
        let regex =
            attributes.required(names.regex, attributes.value_template(attributes.string()))?;
        let flags =
            attributes.optional(names.flags, attributes.value_template(attributes.string()))?;

        let parse = ANALYZE_STRING.get_or_init(|| {
            children(
                instruction(names.xsl_matching_substring)
                    .option()
                    .then(instruction(names.xsl_non_matching_substring).option())
                    .then(instruction(names.xsl_fallback).many()),
            )
        });

        let ((matching_substring, non_matching_substring), fallbacks) = parse(content)?;

        Ok(ast::AnalyzeString {
            select,
            regex,
            flags,

            span,

            matching_substring,
            non_matching_substring,
            fallbacks,
        })
    }
}

static APPLY_IMPORTS_CONTENT: ContentParseLock<Vec<ast::WithParam>> = OnceLock::new();

impl InstructionParser for ast::ApplyImports {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let parse = APPLY_IMPORTS_CONTENT
            .get_or_init(|| children(instruction(content.state.names.xsl_with_param).many()));
        Ok(ast::ApplyImports {
            span: content.span()?,

            with_params: parse(content)?,
        })
    }
}

static APPLY_TEMPLATES_CONTENT: ContentParseLock<Vec<ast::ApplyTemplatesContent>> = OnceLock::new();

impl InstructionParser for ast::ApplyTemplates {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = APPLY_TEMPLATES_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_with_param)
                    .map(ast::ApplyTemplatesContent::WithParam)
                    .or(instruction(names.xsl_sort).map(ast::ApplyTemplatesContent::Sort))
                    .many(),
            )
        });

        let mode = attributes.optional(names.mode, attributes.apply_templates_mode())?;

        let mode = if let Some(mode) = mode {
            mode
        } else {
            match &content.context.default_mode {
                ast::DefaultMode::Unnamed => ast::ApplyTemplatesModeValue::Unnamed,
                ast::DefaultMode::EqName(name) => {
                    ast::ApplyTemplatesModeValue::EqName(name.clone())
                }
            }
        };

        Ok(ast::ApplyTemplates {
            select: attributes.optional(names.select, attributes.xpath())?,
            mode,
            span: content.span()?,
            content: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Assert {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Assert {
            test: attributes.required(names.test, attributes.xpath())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            error_code: attributes.optional(
                names.error_code,
                attributes.value_template(attributes.eqname()),
            )?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Attribute {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Attribute {
            name: attributes.required(names.name, attributes.value_template(attributes.qname()))?,
            namespace: attributes
                .optional(names.namespace, attributes.value_template(attributes.uri()))?,
            select: attributes.optional(names.select, attributes.xpath())?,
            separator: attributes.optional(
                names.separator,
                attributes.value_template(attributes.string()),
            )?,
            type_: attributes.optional(names.type_, attributes.eqname())?,
            validation: attributes.optional(names.validation, attributes.validation())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static ATTRIBUTE_SET_CONTENT: ContentParseLock<Vec<ast::Attribute>> = OnceLock::new();

impl InstructionParser for ast::AttributeSet {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse =
            ATTRIBUTE_SET_CONTENT.get_or_init(|| children(instruction(names.xsl_attribute).many()));

        Ok(ast::AttributeSet {
            name: attributes.required(names.name, attributes.eqname())?,
            use_attribute_sets: attributes
                .optional(names.use_attribute_sets, attributes.eqnames())?,
            visibility: attributes
                .optional(names.visibility, attributes.visibility_with_abstract())?,
            streamable: attributes.boolean_with_default(names.streamable, false)?,

            span: content.span()?,

            attributes: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Break {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(ast::Break {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static CALL_TEMPLATE_CONTENT: ContentParseLock<Vec<ast::WithParam>> = OnceLock::new();

impl InstructionParser for ast::CallTemplate {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let parse = CALL_TEMPLATE_CONTENT
            .get_or_init(|| children(instruction(names.xsl_with_param).many()));

        Ok(ast::CallTemplate {
            name: attributes.required(names.name, attributes.eqname())?,

            span: content.span()?,

            with_params: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Catch {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Catch {
            errors: attributes.optional(names.errors, attributes.tokens())?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static CHARACTER_MAP_CONTENT: ContentParseLock<Vec<ast::OutputCharacter>> = OnceLock::new();

impl InstructionParser for ast::CharacterMap {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = CHARACTER_MAP_CONTENT
            .get_or_init(|| children(instruction(names.xsl_output_character).many()));

        Ok(ast::CharacterMap {
            name: attributes.required(names.name, attributes.eqname())?,
            use_character_maps: attributes
                .optional(names.use_character_maps, attributes.eqnames())?,

            span: content.span()?,

            output_characters: parse(content)?,
        })
    }
}

static CHOOSE_CONTENT: ContentParseLock<(Vec<ast::When>, Option<ast::Otherwise>)> = OnceLock::new();

impl InstructionParser for ast::Choose {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let span = content.span()?;
        let names = &content.state.names;

        let parse = CHOOSE_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_when)
                    .one_or_more()
                    .then(instruction(names.xsl_otherwise).option()),
            )
        });

        let (when, otherwise) = parse(content)?;
        Ok(ast::Choose {
            span,

            when,
            otherwise,
        })
    }
}

impl InstructionParser for ast::Comment {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(ast::Comment {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::ContextItem {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::ContextItem {
            as_: attributes.optional(names.as_, attributes.item_type())?,
            use_: attributes.optional(names.use_, attributes.use_())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::Copy {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Copy {
            select: attributes.optional(names.select, attributes.xpath())?,
            copy_namespaces: attributes.boolean_with_default(names.copy_namespaces, true)?,
            inherit_namespaces: attributes.boolean_with_default(names.inherit_namespaces, true)?,
            use_attribute_sets: attributes
                .optional(names.use_attribute_sets, attributes.eqnames())?,
            type_: attributes.optional(names.type_, attributes.eqname())?,
            validation: attributes
                .optional(names.validation, attributes.validation())?
                // TODO: should depend on global validation attribute
                .unwrap_or(ast::Validation::Strip),

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::CopyOf {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::CopyOf {
            select: attributes.required(names.select, attributes.xpath())?,
            copy_accumulators: attributes.boolean_with_default(names.copy_accumulators, false)?,
            copy_namespaces: attributes.boolean_with_default(names.copy_namespaces, true)?,
            type_: attributes.optional(names.type_, attributes.eqname())?,
            validation: attributes.optional(names.validation, attributes.validation())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::DecimalFormat {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::DecimalFormat {
            name: attributes.optional(names.name, attributes.eqname())?,
            decimal_separator: attributes.optional(names.decimal_separator, attributes.char())?,
            grouping_separator: attributes.optional(names.grouping_separator, attributes.char())?,
            infinity: attributes.optional(names.infinity, attributes.string())?,
            minus_sign: attributes.optional(names.minus_sign, attributes.char())?,
            exponent_separator: attributes.optional(names.exponent_separator, attributes.char())?,
            nan: attributes.optional(names.nan, attributes.string())?,
            percent: attributes.optional(names.percent, attributes.char())?,
            per_mille: attributes.optional(names.per_mille, attributes.char())?,
            zero_digit: attributes.optional(names.zero_digit, attributes.char())?,
            digit: attributes.optional(names.digit, attributes.char())?,
            pattern_separator: attributes.optional(names.pattern_separator, attributes.char())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::Document {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Document {
            validation: attributes.optional(names.validation, attributes.validation())?,
            type_: attributes.optional(names.type_, attributes.eqname())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Element {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Element {
            name: attributes.required(names.name, attributes.value_template(attributes.qname()))?,
            namespace: attributes
                .optional(names.namespace, attributes.value_template(attributes.uri()))?,
            inherit_namespaces: attributes.boolean_with_default(names.inherit_namespaces, false)?,
            use_attribute_sets: attributes
                .optional(names.use_attribute_sets, attributes.eqnames())?,
            type_: attributes.optional(names.type_, attributes.eqname())?,
            validation: attributes.optional(names.validation, attributes.validation())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static EVALUATE_CONTENT: ContentParseLock<Vec<ast::EvaluateContent>> = OnceLock::new();

impl InstructionParser for ast::Evaluate {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let parse = EVALUATE_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_with_param)
                    .map(ast::EvaluateContent::WithParam)
                    .or(instruction(names.xsl_fallback).map(ast::EvaluateContent::Fallback))
                    .many(),
            )
        });

        Ok(ast::Evaluate {
            xpath: attributes.required(names.xpath, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            base_uri: attributes
                .optional(names.base_uri, attributes.value_template(attributes.uri()))?,
            with_params: attributes.optional(names.with_params, attributes.xpath())?,
            context_item: attributes.optional(names.context_item, attributes.xpath())?,
            namespace_context: attributes.optional(names.namespace_context, attributes.xpath())?,
            schema_aware: attributes.optional(
                names.schema_aware,
                attributes.value_template(attributes.boolean()),
            )?,

            span: content.span()?,

            content: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Expose {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Expose {
            component: attributes.required(names.component, attributes.component())?,
            names: attributes.required(names.names, attributes.tokens())?,
            visibility: attributes
                .required(names.visibility, attributes.visibility_with_abstract())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::Fallback {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::Fallback {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static FOR_EACH_CONTENT: ContentParseLock<(Vec<ast::Sort>, ast::SequenceConstructor)> =
    OnceLock::new();

impl InstructionParser for ast::ForEach {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let select = attributes.required(names.select, attributes.xpath())?;
        let span = content.span()?;

        let parse = FOR_EACH_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_sort)
                    .many()
                    .then(sequence_constructor()),
            )
        });

        let (sort, sequence_constructor) = parse(content)?;

        Ok(ast::ForEach {
            select,

            span,

            sort,
            sequence_constructor,
        })
    }
}

static FOR_EACH_GROUP_CONTENT: ContentParseLock<(Vec<ast::Sort>, ast::SequenceConstructor)> =
    OnceLock::new();

impl InstructionParser for ast::ForEachGroup {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let select = attributes.required(names.select, attributes.xpath())?;
        let group_by = attributes.optional(names.group_by, attributes.xpath())?;
        let group_adjacent = attributes.optional(names.group_adjacent, attributes.xpath())?;
        let group_starting_with =
            attributes.optional(names.group_starting_with, attributes.pattern())?;
        let group_ending_with =
            attributes.optional(names.group_ending_with, attributes.pattern())?;
        let composite = attributes.boolean_with_default(names.composite, false)?;
        let collation =
            attributes.optional(names.collation, attributes.value_template(attributes.uri()))?;
        let span = content.span()?;

        let parse = FOR_EACH_GROUP_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_sort)
                    .many()
                    .then(sequence_constructor()),
            )
        });

        let (sort, sequence_constructor) = parse(content)?;

        Ok(ast::ForEachGroup {
            select,
            group_by,
            group_adjacent,
            group_starting_with,
            group_ending_with,
            composite,
            collation,

            span,

            sort,
            sequence_constructor,
        })
    }
}

static FORK_CONTENT: ContentParseLock<(Vec<ast::Fallback>, ast::ForkContent)> = OnceLock::new();

impl InstructionParser for ast::Fork {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let span = content.span()?;

        let parse = FORK_CONTENT.get_or_init(|| {
            let sequence_fallbacks = (instruction(names.xsl_sequence)
                .then(instruction(names.xsl_fallback).many()))
            .many();
            let for_each_group_fallbacks =
                instruction(names.xsl_for_each_group).then(instruction(names.xsl_fallback).many());

            // look for for-each-group first, and only if that fails,
            // look for sequence fallbacks (which can be the empty list and thus
            // would greedily conclude the parse if it was done first)
            children(
                instruction(names.xsl_fallback).many().then(
                    for_each_group_fallbacks
                        .map(ast::ForkContent::ForEachGroup)
                        .or(sequence_fallbacks.map(ast::ForkContent::SequenceFallbacks)),
                ),
            )
        });

        let (fallbacks, content) = parse(content)?;

        Ok(ast::Fork {
            span,

            fallbacks,
            content,
        })
    }
}

static FUNCTION_CONTENT: ContentParseLock<(Vec<ast::Param>, ast::SequenceConstructor)> =
    OnceLock::new();

impl InstructionParser for ast::Function {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let name = attributes.required(names.name, attributes.eqname())?;
        let as_ = attributes.optional(names.as_, attributes.sequence_type())?;
        let visibility =
            attributes.optional(names.visibility, attributes.visibility_with_abstract())?;
        let streamability = attributes.optional(names.streamability, attributes.streamability())?;
        let override_extension_function =
            attributes.boolean_with_default(names.override_extension_function, false)?;
        // deprecated
        let override_ = attributes.boolean_with_default(names.override_, false)?;
        let new_each_time = attributes.optional(names.new_each_time, attributes.new_each_time())?;
        let cache = attributes.boolean_with_default(names.cache, false)?;

        let span = content.span()?;

        let parse = FUNCTION_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_param)
                    .many()
                    .then(sequence_constructor()),
            )
        });

        let (params, sequence_constructor) = parse(content)?;

        Ok(ast::Function {
            name,
            as_,
            visibility,
            streamability,
            override_extension_function,
            override_,
            new_each_time,
            cache,

            span,

            params,
            sequence_constructor,
        })
    }
}

impl InstructionParser for ast::GlobalContextItem {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::GlobalContextItem {
            as_: attributes.optional(names.as_, attributes.item_type())?,
            use_: attributes.optional(names.use_, attributes.use_())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::If {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::If {
            test: attributes.required(names.test, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Import {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Import {
            href: attributes.required(names.href, attributes.uri())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::ImportSchema {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::ImportSchema {
            namespace: attributes.optional(names.namespace, attributes.uri())?,
            schema_location: attributes.optional(names.schema_location, attributes.uri())?,

            span: content.span()?,

            // TODO
            schema: None,
        })
    }
}

impl InstructionParser for ast::Include {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Include {
            href: attributes.required(names.href, attributes.uri())?,

            span: content.span()?,
        })
    }
}

type IterateContent = (
    (Vec<ast::Param>, Option<ast::OnCompletion>),
    ast::SequenceConstructor,
);

static ITERATE_CONTENT: ContentParseLock<IterateContent> = OnceLock::new();

impl InstructionParser for ast::Iterate {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        let select = attributes.required(names.select, attributes.xpath())?;

        let span = content.span()?;

        let parse = ITERATE_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_param)
                    .many()
                    .then(instruction(names.xsl_on_completion).option())
                    .then(sequence_constructor()),
            )
        });

        let ((params, on_completion), sequence_constructor) = parse(content)?;

        Ok(ast::Iterate {
            select,

            span,

            params,
            on_completion,
            sequence_constructor,
        })
    }
}

impl InstructionParser for ast::Key {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::Key {
            name: attributes.required(names.name, attributes.eqname())?,
            match_: attributes.required(names.match_, attributes.pattern())?,
            use_: attributes.optional(names.use_, attributes.xpath())?,
            composite: attributes.boolean_with_default(names.composite, false)?,
            collation: attributes.optional(names.collation, attributes.uri())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Map {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::Map {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MapEntry {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::MapEntry {
            key: attributes.required(names.key, attributes.xpath())?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MatchingSubstring {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::MatchingSubstring {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

type MergeContent = (
    Vec<ast::MergeSource>,
    (ast::MergeAction, Vec<ast::Fallback>),
);

static MERGE_CONTENT: ContentParseLock<MergeContent> = OnceLock::new();

impl InstructionParser for ast::Merge {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = MERGE_CONTENT.get_or_init(|| {
            children(instruction(names.xsl_merge_source).one_or_more().then(
                instruction(names.xsl_merge_action).then(instruction(names.xsl_fallback).many()),
            ))
        });

        let (merge_sources, (merge_action, fallbacks)) = parse(content)?;

        Ok(ast::Merge {
            span: content.span()?,

            merge_sources,
            merge_action,
            fallbacks,
        })
    }
}

impl InstructionParser for ast::MergeAction {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::MergeAction {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MergeKey {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::MergeKey {
            select: attributes.optional(names.select, attributes.xpath())?,
            lang: attributes
                .optional(names.lang, attributes.value_template(attributes.language()))?,
            order: attributes
                .optional(names.order, attributes.value_template(attributes.order()))?,
            collation: attributes
                .optional(names.collation, attributes.value_template(attributes.uri()))?,
            case_order: attributes.optional(
                names.case_order,
                attributes.value_template(attributes.case_order()),
            )?,
            data_type: attributes.optional(
                names.data_type,
                attributes.value_template(attributes.data_type()),
            )?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

static MERGE_SOURCE_CONTENT: ContentParseLock<Vec<ast::MergeKey>> = OnceLock::new();

impl InstructionParser for ast::MergeSource {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = MERGE_SOURCE_CONTENT
            .get_or_init(|| children(instruction(names.xsl_merge_key).one_or_more()));

        Ok(ast::MergeSource {
            name: attributes.optional(names.name, attributes.ncname())?,
            for_each_item: attributes.optional(names.for_each_item, attributes.xpath())?,
            for_each_source: attributes.optional(names.for_each_source, attributes.xpath())?,
            select: attributes.required(names.select, attributes.xpath())?,
            streamable: attributes.boolean_with_default(names.streamable, false)?,
            use_accumulators: attributes.optional(names.use_accumulators, attributes.tokens())?,
            sort_before_merge: attributes.boolean_with_default(names.sort_before_merge, false)?,
            validation: attributes.optional(names.validation, attributes.validation())?,
            type_: attributes.optional(names.type_, attributes.eqname())?,

            span: content.span()?,

            merge_keys: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Message {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Message {
            select: attributes.optional(names.select, attributes.xpath())?,
            terminate: attributes.optional(
                names.terminate,
                attributes.value_template(attributes.boolean()),
            )?,
            error_code: attributes.optional(
                names.error_code,
                attributes.value_template(attributes.eqname()),
            )?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Mode {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Mode {
            name: attributes.optional(names.name, attributes.eqname())?,
            streamable: attributes.boolean_with_default(names.streamable, false)?,
            use_accumulators: attributes.optional(names.use_accumulators, attributes.tokens())?,
            on_no_match: attributes.optional(names.on_no_match, attributes.on_no_match())?,
            on_multiple_match: attributes
                .optional(names.on_multiple_match, attributes.on_multiple_match())?,
            warning_on_no_match: attributes
                .boolean_with_default(names.warning_on_no_match, false)?,
            warning_on_multiple_match: attributes
                .boolean_with_default(names.warning_on_multiple_match, false)?,
            typed: attributes.optional(names.typed, attributes.typed())?,
            visibility: attributes.optional(names.visibility, attributes.visibility())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::Namespace {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Namespace {
            name: attributes
                .required(names.name, attributes.value_template(attributes.ncname()))?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::NamespaceAlias {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::NamespaceAlias {
            stylesheet_prefix: attributes
                .required(names.stylesheet_prefix, attributes.prefix_or_default())?,
            result_prefix: attributes
                .required(names.result_prefix, attributes.prefix_or_default())?,

            span: content.span()?,
        })
    }
}

static NEXT_ITERATION_CONTENT: ContentParseLock<Vec<ast::WithParam>> = OnceLock::new();

impl InstructionParser for ast::NextIteration {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let parse = NEXT_ITERATION_CONTENT
            .get_or_init(|| children(instruction(content.state.names.xsl_with_param).many()));

        Ok(ast::NextIteration {
            span: content.span()?,

            with_params: parse(content)?,
        })
    }
}

static NEXT_MATCH_CONTENT: ContentParseLock<Vec<ast::NextMatchContent>> = OnceLock::new();

impl InstructionParser for ast::NextMatch {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let parse = NEXT_MATCH_CONTENT.get_or_init(|| {
            children(
                instruction(names.xsl_with_param)
                    .map(ast::NextMatchContent::WithParam)
                    .or(instruction(names.xsl_fallback).map(ast::NextMatchContent::Fallback))
                    .many(),
            )
        });

        Ok(ast::NextMatch {
            span: content.span()?,

            content: parse(content)?,
        })
    }
}

impl InstructionParser for ast::NonMatchingSubstring {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::NonMatchingSubstring {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Number {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::Number {
            value: attributes.optional(names.value, attributes.xpath())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            level: attributes.optional(names.level, attributes.level())?,
            count: attributes.optional(names.count, attributes.pattern())?,
            from: attributes.optional(names.from, attributes.pattern())?,
            format: attributes
                .optional(names.format, attributes.value_template(attributes.string()))?,
            lang: attributes
                .optional(names.lang, attributes.value_template(attributes.language()))?,
            letter_value: attributes.optional(
                names.letter_value,
                attributes.value_template(attributes.letter_value()),
            )?,
            ordinal: attributes.optional(
                names.ordinal,
                attributes.value_template(attributes.string()),
            )?,
            start_at: attributes.optional(
                names.start_at,
                attributes.value_template(attributes.string()),
            )?,
            grouping_separator: attributes.optional(
                names.grouping_separator,
                attributes.value_template(attributes.char()),
            )?,
            grouping_size: attributes.optional(
                names.grouping_size,
                attributes.value_template(attributes.integer()),
            )?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::OnCompletion {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(ast::OnCompletion {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::OnEmpty {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(ast::OnEmpty {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::OnNonEmpty {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(ast::OnNonEmpty {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Otherwise {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::Otherwise {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Output {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Output {
            name: attributes.optional(names.name, attributes.eqname())?,
            method: attributes.optional(names.method, attributes.method())?,
            allow_duplicate_names: attributes
                .boolean_with_default(names.allow_duplicate_names, false)?,
            build_tree: attributes.boolean_with_default(names.build_tree, false)?,
            byte_order_mark: attributes.boolean_with_default(names.byte_order_mark, false)?,
            cdata_section_elements: attributes
                .optional(names.cdata_section_elements, attributes.eqnames())?,
            doctype_public: attributes.optional(names.doctype_public, attributes.string())?,
            doctype_system: attributes.optional(names.doctype_system, attributes.string())?,
            encoding: attributes.optional(names.encoding, attributes.string())?,
            escape_uri_attributes: attributes
                .boolean_with_default(names.escape_uri_attributes, true)?,
            html_version: attributes.optional(names.html_version, attributes.decimal())?,
            include_content_type: attributes
                .boolean_with_default(names.include_content_type, true)?,
            // TODO default value is informed by the method
            indent: attributes.boolean_with_default(names.indent, false)?,
            item_separator: attributes.optional(names.item_separator, attributes.string())?,
            json_node_output_method: attributes.optional(
                names.json_node_output_method,
                attributes.json_node_output_method(),
            )?,
            media_type: attributes.optional(names.media_type, attributes.string())?,
            normalization_form: attributes
                .optional(names.normalization_form, attributes.normalization_form())?,
            omit_xml_declaration: attributes
                .boolean_with_default(names.omit_xml_declaration, false)?,
            parameter_document: attributes.optional(names.parameter_document, attributes.uri())?,
            standalone: attributes.optional(names.standalone, attributes.standalone())?,
            suppress_indentation: attributes
                .optional(names.suppress_indentation, attributes.eqnames())?,
            undeclare_prefixes: attributes.boolean_with_default(names.undeclare_prefixes, false)?,
            use_character_maps: attributes
                .optional(names.use_character_maps, attributes.eqnames())?,
            version: attributes.optional(names.version, attributes.nmtoken())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::OutputCharacter {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::OutputCharacter {
            character: attributes.required(names.character, attributes.char())?,
            string: attributes.required(names.string, attributes.string())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::OverrideContent {
    fn parse(content: &Content, attributes: &Attributes) -> Result<ast::OverrideContent> {
        let name = content
            .state
            .names
            .override_content_name(attributes.element.name());

        if let Some(name) = name {
            name.parse(attributes)
        } else {
            Err(Error::Unexpected {
                span: content.span()?,
            })
        }
    }
}

static OVERRIDE_CONTENT: ContentParseLock<Vec<ast::OverrideContent>> = OnceLock::new();

impl InstructionParser for ast::Override {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        let parse = OVERRIDE_CONTENT.get_or_init(|| {
            children(one(by_element(ast::OverrideContent::parse_override_content)).many())
        });

        Ok(ast::Override {
            span: content.span()?,

            content: parse(content)?,
        })
    }
}

// TODO: xsl:package

impl InstructionParser for ast::Param {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Param {
            name: attributes.required(names.name, attributes.eqname())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            required: attributes.boolean_with_default(names.required, false)?,
            tunnel: attributes.boolean_with_default(names.tunnel, false)?,
            static_: attributes.boolean_with_default(names.static_, false)?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

// TODO: xsl:perform-sort

// TODO: xsl:preserve-space

impl InstructionParser for ast::PreserveSpace {
    fn should_be_empty() -> bool {
        true
    }

    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::PreserveSpace {
            elements: attributes.required(names.elements, attributes.tokens())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::ProcessingInstruction {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::ProcessingInstruction {
            name: attributes
                .required(names.name, attributes.value_template(attributes.ncname()))?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

// TODO: xsl:result-document

impl InstructionParser for ast::Sequence {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Sequence {
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Sort {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Sort {
            select: attributes.optional(names.select, attributes.xpath())?,
            lang: attributes
                .optional(names.lang, attributes.value_template(attributes.language()))?,
            order: attributes
                .optional(names.order, attributes.value_template(attributes.order()))?,
            collation: attributes
                .optional(names.collation, attributes.value_template(attributes.uri()))?,
            stable: attributes.optional(
                names.stable,
                attributes.value_template(attributes.boolean()),
            )?,
            case_order: attributes.optional(
                names.case_order,
                attributes.value_template(attributes.case_order()),
            )?,
            data_type: attributes.optional(
                names.data_type,
                attributes.value_template(attributes.data_type()),
            )?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::SourceDocument {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(ast::SourceDocument {
            href: attributes.required(names.href, attributes.value_template(attributes.uri()))?,
            streamable: attributes.boolean_with_default(names.streamable, false)?,
            use_accumulators: attributes.optional(names.use_accumulators, attributes.tokens())?,
            validation: attributes.optional(names.validation, attributes.validation())?,
            type_: attributes.optional(names.type_, attributes.eqname())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::StripSpace {
    fn should_be_empty() -> bool {
        true
    }
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::StripSpace {
            elements: attributes.required(names.elements, attributes.tokens())?,

            span: content.span()?,
        })
    }
}

type TemplateContent = (
    (Option<ast::ContextItem>, Vec<ast::Param>),
    ast::SequenceConstructor,
);

static TEMPLATE_CONTENT: ContentParseLock<TemplateContent> = OnceLock::new();

impl InstructionParser for ast::Template {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let match_ = attributes.optional(names.match_, attributes.pattern())?;
        let name = attributes.optional(names.name, attributes.eqname())?;
        let priority = attributes.optional(names.priority, attributes.decimal())?;
        let mode = attributes.optional(names.mode, attributes.modes())?;
        let as_ = attributes.optional(names.as_, attributes.sequence_type())?;
        let visibility =
            attributes.optional(names.visibility, attributes.visibility_with_abstract())?;

        let parse = TEMPLATE_CONTENT.get_or_init(|| {
            children(
                instruction(names.context_item)
                    .option()
                    .then(instruction(names.xsl_param).many())
                    .then(sequence_constructor()),
            )
        });

        let mode = mode.unwrap_or_else(|| {
            vec![match &content.context.default_mode {
                ast::DefaultMode::Unnamed => ast::ModeValue::Unnamed,
                ast::DefaultMode::EqName(name) => ast::ModeValue::EqName(name.clone()),
            }]
        });

        let span = content.span()?;
        let ((context_item, params), sequence_constructor) = parse(content)?;

        Ok(ast::Template {
            match_,
            name,
            priority,
            mode,
            as_,
            visibility,

            span,

            context_item,
            params,
            sequence_constructor,
        })
    }
}

impl InstructionParser for ast::Text {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        let children = content.state.xot.children(content.node).collect::<Vec<_>>();
        if children.len() > 1 {
            return Err(Error::Unexpected {
                span: content.span()?,
            });
        }
        let text = if !children.is_empty() {
            let text = content.state.xot.text_content_str(content.node);
            if let Some(text) = text {
                text
            } else {
                // this wasn't text content, and it wasn't because it was
                // empty either
                return Err(Error::Unexpected {
                    span: content.span()?,
                });
            }
        } else {
            ""
        };
        let text_content = attributes.value_template(attributes.string())(text, content.span()?)?;

        Ok(ast::Text {
            disable_output_escaping: attributes
                .boolean_with_default(names.disable_output_escaping, false)?,

            span: content.span()?,

            content: text_content,
        })
    }
}

impl InstructionParser for ast::Transform {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::Transform {
            id: attributes.optional(names.id, attributes.id())?,
            input_type_annotations: attributes.optional(
                names.input_type_annotations,
                attributes.input_type_annotations(),
            )?,
            extension_element_prefixes: attributes
                .optional(names.extension_element_prefixes, attributes.prefixes())?,

            span: content.span()?,

            declarations: content.declarations()?,
        })
    }
}

// TODO: xsl:try

// TODO: xsl:use-package

impl InstructionParser for ast::ValueOf {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::ValueOf {
            select: attributes.optional(names.select, attributes.xpath())?,
            separator: attributes.optional(
                names.separator,
                attributes.value_template(attributes.string()),
            )?,
            disable_output_escaping: attributes
                .boolean_with_default(names.disable_output_escaping, false)?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Variable {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        // This is a rule somewhere, but not sure whether it's correct;
        // can visibility be absent or is there a default visibility?
        // let visibility = visibility.unwrap_or(if static_ {
        //     ast::VisibilityWithAbstract::Private
        // } else {
        //     ast::VisibilityWithAbstract::Public
        // });

        Ok(ast::Variable {
            name: attributes.required(names.name, attributes.eqname())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            static_: attributes.boolean_with_default(names.static_, false)?,
            visibility: attributes
                .optional(names.visibility, attributes.visibility_with_abstract())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }

    fn validate(&self, node: Node, state: &State) -> Result<()> {
        if self.visibility == Some(ast::VisibilityWithAbstract::Abstract) && self.select.is_some() {
            return Err(state
                .attribute_unexpected(
                    node,
                    state.names.select,
                    "select attribute is not allowed when visibility is abstract",
                )
                .into());
        }
        Ok(())
    }
}

impl InstructionParser for ast::When {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::When {
            test: attributes.required(names.test, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::WherePopulated {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(ast::WherePopulated {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::WithParam {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(ast::WithParam {
            name: attributes.required(names.name, attributes.eqname())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            tunnel: attributes.boolean_with_default(names.tunnel, false)?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}
