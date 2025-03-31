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
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
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
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
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
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
            name: attributes.required(names.name, attributes.eqname())?,

            span: content.span()?,

            with_params: parse(content)?,
        })
    }
}

impl InstructionParser for ast::Catch {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
            span,

            when,
            otherwise,
        })
    }
}

impl InstructionParser for ast::Comment {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(Self {
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
        Ok(Self {
            as_: attributes.optional(names.as_, attributes.item_type())?,
            use_: attributes.optional(names.use_, attributes.use_())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::Copy {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
            as_: attributes.optional(names.as_, attributes.item_type())?,
            use_: attributes.optional(names.use_, attributes.use_())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::If {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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
        Ok(Self {
            href: attributes.required(names.href, attributes.uri())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::ImportSchema {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;

        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MapEntry {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
            key: attributes.required(names.key, attributes.xpath())?,
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MatchingSubstring {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(Self {
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

        Ok(Self {
            span: content.span()?,

            merge_sources,
            merge_action,
            fallbacks,
        })
    }
}

impl InstructionParser for ast::MergeAction {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(Self {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::MergeKey {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
            span: content.span()?,

            content: parse(content)?,
        })
    }
}

impl InstructionParser for ast::NonMatchingSubstring {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::OnEmpty {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(Self {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::OnNonEmpty {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        Ok(Self {
            select: attributes.optional(content.state.names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Otherwise {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
            character: attributes.required(names.character, attributes.char())?,
            string: attributes.required(names.string, attributes.string())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::OverrideContent {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
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

        Ok(Self {
            span: content.span()?,

            content: parse(content)?,
        })
    }
}

// TODO: xsl:package

impl InstructionParser for ast::Param {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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
        Ok(Self {
            elements: attributes.required(names.elements, attributes.tokens())?,

            span: content.span()?,
        })
    }
}

impl InstructionParser for ast::ProcessingInstruction {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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
        Ok(Self {
            select: attributes.optional(names.select, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::Sort {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
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
        Ok(Self {
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

        Ok(Self {
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
        Ok(Self {
            test: attributes.required(names.test, attributes.xpath())?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::WherePopulated {
    fn parse(content: &Content, _attributes: &Attributes) -> Result<Self> {
        Ok(Self {
            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

impl InstructionParser for ast::WithParam {
    fn parse(content: &Content, attributes: &Attributes) -> Result<Self> {
        let names = &content.state.names;
        Ok(Self {
            name: attributes.required(names.name, attributes.eqname())?,
            select: attributes.optional(names.select, attributes.xpath())?,
            as_: attributes.optional(names.as_, attributes.sequence_type())?,
            tunnel: attributes.boolean_with_default(names.tunnel, false)?,

            span: content.span()?,

            sequence_constructor: content.sequence_constructor()?,
        })
    }
}

#[cfg(test)]
mod tests {

    use crate::context::Context;
    use crate::{element::XsltParser, names::Names, state::State};

    use super::*;
    use insta::assert_ron_snapshot;
    use xot::Xot;

    fn parse_sequence_constructor_item(s: &str) -> Result<ast::SequenceConstructorItem> {
        let mut xot = Xot::new();
        let names = Names::new(&mut xot);
        let (node, span_info) = xot.parse_with_span_info(s).unwrap();
        let state = State::new(xot, span_info, names);
        let node = state.xot.document_element(node).unwrap();

        if let Some(element) = state.xot.element(node) {
            let context = Context::new(state.xot.prefixes(node));
            let content = Content::new(node, &state, context);
            content.parse_element(
                element,
                ast::SequenceConstructorItem::parse_sequence_constructor_item,
            )
        } else {
            Err(Error::Internal)
        }
    }

    fn parse_transform(s: &str) -> Result<ast::Transform> {
        let mut xot = Xot::new();
        let names = Names::new(&mut xot);

        let (node, span_info) = xot.parse_with_span_info(s).unwrap();
        let node = xot.document_element(node).unwrap();
        let context = State::new(xot, span_info, names);
        let parser = XsltParser::new(&context);
        parser.parse_transform(node)
    }

    #[test]
    fn test_if() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()">Hello</xsl:if>"#
        ));
    }

    #[test]
    fn test_variable() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" select="true()">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_missing_required() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_broken_xpath() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" select="let $x := 1">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_sequence_type() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xmlns:xs="http://www.w3.org/2001/XMLSchema" name="foo" as="xs:string" select="true()">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_boolean_default_no_with_explicit_yes() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" static="yes" select="true()">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_variable_visibility() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" visibility="public">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_variable_visibility_abstract_with_select_is_error() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:variable xmlns:xsl="http://www.w3.org/1999/XSL/Transform" name="foo" visibility="abstract" select="true()">Hello</xsl:variable>"#
        ));
    }

    #[test]
    fn test_copy() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" copy-namespaces="no" inherit-namespaces="no" validation="strict">Hello</xsl:copy>"#
        ));
    }

    #[test]
    fn test_eqnames() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" use-attribute-sets="foo bar baz">Hello</xsl:copy>"#
        ));
    }

    #[test]
    fn test_eqnames_error() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:copy xmlns:xsl="http://www.w3.org/1999/XSL/Transform" use-attribute-sets="foo br!ken bar">Hello</xsl:copy>"#
        ));
    }

    #[test]
    fn test_nested_if() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><xsl:if test="true()">Hello</xsl:if></xsl:if>"#
        ));
    }

    #[test]
    fn test_if_with_standard_attribute() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes">Hello</xsl:if>"#
        ));
    }

    #[test]
    fn test_literal_result_element() {
        assert_ron_snapshot!(parse_sequence_constructor_item(r#"<foo/>"#));
    }

    #[test]
    fn test_literal_result_element_with_standard_attribute() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<foo xmlns:xsl="http://www.w3.org/1999/XSL/Transform" xsl:expand-text="yes"/>"#
        ));
    }

    #[test]
    fn test_no_fn_namespace_by_default() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="fn:true()">Hello</xsl:if>"#
        ));
    }

    #[test]
    fn test_attribute_value_template_just_string() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:assert xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" error-code="foo">Hello</xsl:assert>"#
        ));
    }

    #[test]
    fn test_analyze_string() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_analyze_string_absent_matching_substring() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_analyze_string_absent_non_matching_substring() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:fallback>Fallback 1</xsl:fallback><xsl:fallback>Fallback 2</xsl:fallback></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_analyze_string_absent_fallbacks() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:matching-substring>Matching</xsl:matching-substring><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_analyze_string_absent_all() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_analyze_string_matching_non_matching_wrong_order() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:analyze-string xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()" regex="foo"><xsl:non-matching-substring>Nonmatching</xsl:non-matching-substring><xsl:matching-substring>Matching</xsl:matching-substring></xsl:analyze-string>"#
        ));
    }

    #[test]
    fn test_accumulator() {
        assert_ron_snapshot!(parse_transform(
            r#"<xsl:transform version="3.0" xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:accumulator name="foo" initial-value="1"><xsl:accumulator-rule match="foo"/></xsl:accumulator></xsl:transform>"#
        ));
    }

    #[test]
    fn test_should_be_empty_not_empty() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:copy-of xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()">Illegal content</xsl:copy-of>"#
        ))
    }

    #[test]
    fn test_apply_templates_with_mixed_content() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:apply-templates xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:sort>Sort</xsl:sort><xsl:with-param name="a">With param</xsl:with-param></xsl:apply-templates>"#
        ))
    }

    #[test]
    fn test_for_each() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:for-each xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="true()"><xsl:sort>Sort 1</xsl:sort><xsl:sort>Sort 2</xsl:sort>Sequence constructor</xsl:for-each>"#
        ))
    }

    #[test]
    fn test_fork1() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:fork xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:sequence>Sequence 1</xsl:sequence><xsl:sequence>Sequence 2</xsl:sequence></xsl:fork>"#
        ))
    }

    #[test]
    fn test_fork2() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:fork xmlns:xsl="http://www.w3.org/1999/XSL/Transform"><xsl:for-each-group select="true()">Content</xsl:for-each-group></xsl:fork>"#
        ))
    }

    #[test]
    fn test_unsupported_attribute() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" unsupported="Unsupported">Hello</xsl:if>"#
        ));
    }

    #[test]
    fn test_no_expand_text_should_not_expand_text() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()">Hello {world}!</xsl:if>"#
        ));
    }

    #[test]
    fn test_expand_text_should_expand_text() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes">Hello {world}!</xsl:if>"#
        ));
    }

    #[test]
    #[ignore]
    fn test_xsl_expand_text_should_expand_text() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p xsl:expand-text="yes">Hello {world}!</p></xsl:if>"#
        ));
    }

    #[test]
    fn test_expand_text_disabled_should_not_expand_text() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()" expand-text="yes"><xsl:if expand-text="no" test="true()">Hello {world}!</xsl:if></xsl:if>"#
        ));
    }

    #[test]
    fn test_nested_literal_elements() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p><another/></p></xsl:if>"#
        ));
    }

    #[test]
    fn test_sequence_constructor_nested_in_literal_element() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:if xmlns:xsl="http://www.w3.org/1999/XSL/Transform" test="true()"><p><xsl:if test="true()">foo</xsl:if></p></xsl:if>"#
        ));
    }

    #[test]
    fn test_attributes_on_literal_element() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<p xmlns:xsl="http://www.w3.org/1999/XSL/Transform" foo="FOO"/>"#
        ));
    }

    #[test]
    fn test_template_unnamed_mode() {
        assert_ron_snapshot!(parse_transform(
            r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*">a</xsl:template></xsl:transform>"#
        ));
    }

    #[test]
    fn test_template_named_mode() {
        assert_ron_snapshot!(parse_transform(
            r#"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*" mode="foo">a</xsl:template></xsl:transform>"#
        ));
    }

    #[test]
    fn test_template_default_mode_explicit_fallback_to_default() {
        // to include #" in the raw string, we need to mark it with double #
        // https://rahul-thakoor.github.io/rust-raw-string-literals/
        assert_ron_snapshot!(parse_transform(
            r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" default-mode="foo"><xsl:template match="*" mode="#default">a</xsl:template></xsl:transform>"##
        ));
    }

    #[test]
    fn test_template_default_mode_implicit_fallback_to_default() {
        // to include #" in the raw string, we need to mark it with double #
        // https://rahul-thakoor.github.io/rust-raw-string-literals/
        assert_ron_snapshot!(parse_transform(
            r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3" default-mode="foo"><xsl:template match="*">a</xsl:template></xsl:transform>"##
        ));
    }

    #[test]
    fn test_template_default_mode_explicit_fallback_to_unnamed() {
        // to include #" in the raw string, we need to mark it with double #
        // https://rahul-thakoor.github.io/rust-raw-string-literals/
        assert_ron_snapshot!(parse_transform(
            r##"<xsl:transform xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3"><xsl:template match="*" mode="#default">a</xsl:template></xsl:transform>"##
        ));
    }

    #[test]
    fn test_apply_templates_implicit_default_mode_is_unnamed() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:apply-templates xmlns:xsl="http://www.w3.org/1999/XSL/Transform" />"#
        ));
    }

    #[test]
    fn test_apply_templates_explicit_default_mode() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:apply-templates default-mode="foo" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="*"/>"#
        ));
    }

    #[test]
    fn test_apply_templates_explicit_mode() {
        assert_ron_snapshot!(parse_sequence_constructor_item(
            r#"<xsl:apply-templates mode="foo" xmlns:xsl="http://www.w3.org/1999/XSL/Transform" select="*"/>"#
        ));
    }
}
