use std::collections::BTreeMap;
use std::str::FromStr;

use ahash::HashSet;
use strum::VariantNames;
use xot::{NameId, NamespaceId, Xot};

use crate::ast_core::{self as ast, DeclarationName, OverrideContentName, SequenceConstructorName};
use crate::attributes::Attributes;
use crate::error::ElementError;
use crate::instruction::{DeclarationParser, OverrideContentParser, SequenceConstructorParser};

impl SequenceConstructorName {
    pub(crate) fn parse(
        &self,
        attributes: &Attributes,
    ) -> Result<ast::SequenceConstructorItem, ElementError> {
        match self {
            SequenceConstructorName::ApplyImports => {
                ast::ApplyImports::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::AnalyzeString => {
                ast::AnalyzeString::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::ApplyTemplates => {
                ast::ApplyTemplates::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Assert => {
                ast::Assert::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Attribute => {
                ast::Attribute::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Break => {
                ast::Break::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::CallTemplate => {
                ast::CallTemplate::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Choose => {
                ast::Choose::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Comment => {
                ast::Comment::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Copy => ast::Copy::parse_sequence_constructor_item(attributes),
            SequenceConstructorName::CopyOf => {
                ast::CopyOf::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Document => {
                ast::Document::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Element => {
                ast::Element::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Evaluate => {
                ast::Evaluate::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Fallback => {
                ast::Fallback::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::ForEach => {
                ast::ForEach::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::ForEachGroup => {
                ast::ForEachGroup::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Fork => ast::Fork::parse_sequence_constructor_item(attributes),
            SequenceConstructorName::If => ast::If::parse_sequence_constructor_item(attributes),
            SequenceConstructorName::Map => ast::Map::parse_sequence_constructor_item(attributes),
            SequenceConstructorName::MapEntry => {
                ast::MapEntry::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Merge => {
                ast::Merge::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Message => {
                ast::Message::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Namespace => {
                ast::Namespace::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::NextIteration => {
                ast::NextIteration::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::NextMatch => {
                ast::NextMatch::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Number => {
                ast::Number::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::OnEmpty => {
                ast::OnEmpty::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::OnNonEmpty => {
                ast::OnNonEmpty::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::ProcessingInstruction => {
                ast::ProcessingInstruction::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Sequence => {
                ast::Sequence::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::SourceDocument => {
                ast::SourceDocument::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Text => ast::Text::parse_sequence_constructor_item(attributes),
            SequenceConstructorName::ValueOf => {
                ast::ValueOf::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::Variable => {
                ast::Variable::parse_sequence_constructor_item(attributes)
            }
            SequenceConstructorName::WherePopulated => {
                ast::WherePopulated::parse_sequence_constructor_item(attributes)
            }

            _ => Err(ElementError::Unsupported),
        }
    }

    fn names(xot: &mut Xot, xsl_ns: xot::NamespaceId) -> BTreeMap<NameId, SequenceConstructorName> {
        let mut sequence_constructor_names = BTreeMap::new();
        for variant_name in Self::VARIANTS {
            let name = xot.add_name_ns(variant_name, xsl_ns);
            let constructor = SequenceConstructorName::from_str(variant_name).unwrap();
            sequence_constructor_names.insert(name, constructor);
        }
        sequence_constructor_names
    }
}

impl DeclarationName {
    pub(crate) fn parse(&self, attributes: &Attributes) -> Result<ast::Declaration, ElementError> {
        match self {
            DeclarationName::Accumulator => ast::Accumulator::parse_declaration(attributes),
            DeclarationName::Template => ast::Template::parse_declaration(attributes),
            _ => Err(ElementError::Unsupported),
        }
    }

    fn names(xot: &mut Xot, xsl_ns: xot::NamespaceId) -> BTreeMap<NameId, DeclarationName> {
        let mut declaration_names = BTreeMap::new();
        for variant_name in Self::VARIANTS {
            let name = xot.add_name_ns(variant_name, xsl_ns);
            let constructor = DeclarationName::from_str(variant_name).unwrap();
            declaration_names.insert(name, constructor);
        }
        declaration_names
    }
}

impl OverrideContentName {
    pub(crate) fn parse(
        &self,
        attributes: &Attributes,
    ) -> Result<ast::OverrideContent, ElementError> {
        use ast::OverrideContentName::*;

        match self {
            Template => ast::Template::parse_override_content(attributes),
            Function => ast::Function::parse_override_content(attributes),
            Variable => ast::Variable::parse_override_content(attributes),
            Param => ast::Param::parse_override_content(attributes),
            AttributeSet => ast::AttributeSet::parse_override_content(attributes),
        }
    }

    fn names(xot: &mut Xot, xsl_ns: xot::NamespaceId) -> BTreeMap<NameId, OverrideContentName> {
        let mut override_content_names = BTreeMap::new();
        for variant_name in Self::VARIANTS {
            let name = xot.add_name_ns(variant_name, xsl_ns);
            let constructor = OverrideContentName::from_str(variant_name).unwrap();
            override_content_names.insert(name, constructor);
        }
        override_content_names
    }
}

pub(crate) struct Names {
    pub(crate) xsl_ns: NamespaceId,

    pub(crate) sequence_constructor_names: BTreeMap<NameId, SequenceConstructorName>,
    pub(crate) declaration_names: BTreeMap<NameId, DeclarationName>,
    pub(crate) override_content_names: BTreeMap<NameId, OverrideContentName>,
    pub(crate) ignore_xml_space_parents: HashSet<NameId>,
    pub(crate) ignore_xml_space_next_siblings: HashSet<NameId>,
    pub(crate) ignore_xml_space_previous_siblings: HashSet<NameId>,

    // XSL elements
    pub(crate) xsl_accumulator_rule: xot::NameId,
    pub(crate) xsl_attribute: xot::NameId,
    pub(crate) xsl_fallback: xot::NameId,
    pub(crate) xsl_for_each: xot::NameId,
    pub(crate) xsl_for_each_group: xot::NameId,
    pub(crate) xsl_matching_substring: xot::NameId,
    pub(crate) xsl_merge_action: xot::NameId,
    pub(crate) xsl_merge_key: xot::NameId,
    pub(crate) xsl_merge_source: xot::NameId,
    pub(crate) xsl_non_matching_substring: xot::NameId,
    pub(crate) xsl_on_completion: xot::NameId,
    pub(crate) xsl_otherwise: xot::NameId,
    pub(crate) xsl_output_character: xot::NameId,
    pub(crate) xsl_param: xot::NameId,
    pub(crate) xsl_schema: xot::NameId,
    pub(crate) xsl_sequence: xot::NameId,
    pub(crate) xsl_sort: xot::NameId,
    pub(crate) xsl_text: xot::NameId,
    pub(crate) xsl_transform: xot::NameId,
    pub(crate) xsl_variable: xot::NameId,
    pub(crate) xsl_when: xot::NameId,
    pub(crate) xsl_with_param: xot::NameId,

    // attributes on XSLT elements
    pub(crate) allow_duplicate_names: xot::NameId,
    pub(crate) as_: xot::NameId,
    pub(crate) base_uri: xot::NameId,
    pub(crate) build_tree: xot::NameId,
    pub(crate) byte_order_mark: xot::NameId,
    pub(crate) cache: xot::NameId,
    pub(crate) case_order: xot::NameId,
    pub(crate) cdata_section_elements: xot::NameId,
    pub(crate) character: xot::NameId,
    pub(crate) component: xot::NameId,
    pub(crate) composite: xot::NameId,
    pub(crate) context_item: xot::NameId,
    pub(crate) collation: xot::NameId,
    pub(crate) copy_accumulators: xot::NameId,
    pub(crate) copy_namespaces: xot::NameId,
    pub(crate) count: xot::NameId,
    pub(crate) data_type: xot::NameId,
    pub(crate) decimal_separator: xot::NameId,
    pub(crate) digit: xot::NameId,
    pub(crate) disable_output_escaping: xot::NameId,
    pub(crate) doctype_public: xot::NameId,
    pub(crate) doctype_system: xot::NameId,
    pub(crate) elements: xot::NameId,
    pub(crate) encoding: xot::NameId,
    pub(crate) error_code: xot::NameId,
    pub(crate) errors: xot::NameId,
    pub(crate) escape_uri_attributes: xot::NameId,
    pub(crate) exponent_separator: xot::NameId,
    pub(crate) extension_element_prefixes: xot::NameId,
    pub(crate) flags: xot::NameId,
    pub(crate) for_each_item: xot::NameId,
    pub(crate) for_each_source: xot::NameId,
    pub(crate) format: xot::NameId,
    pub(crate) from: xot::NameId,
    pub(crate) group_adjacent: xot::NameId,
    pub(crate) group_by: xot::NameId,
    pub(crate) group_ending_with: xot::NameId,
    pub(crate) group_starting_with: xot::NameId,
    pub(crate) grouping_size: xot::NameId,
    pub(crate) grouping_separator: xot::NameId,
    pub(crate) href: xot::NameId,
    pub(crate) html_version: xot::NameId,
    pub(crate) id: xot::NameId,
    pub(crate) include_content_type: xot::NameId,
    pub(crate) indent: xot::NameId,
    pub(crate) infinity: xot::NameId,
    pub(crate) inherit_namespaces: xot::NameId,
    pub(crate) initial_value: xot::NameId,
    pub(crate) input_type_annotations: xot::NameId,
    pub(crate) item_separator: xot::NameId,
    pub(crate) json_node_output_method: xot::NameId,
    pub(crate) key: xot::NameId,
    pub(crate) lang: xot::NameId,
    pub(crate) letter_value: xot::NameId,
    pub(crate) level: xot::NameId,
    pub(crate) match_: xot::NameId,
    pub(crate) media_type: xot::NameId,
    pub(crate) method: xot::NameId,
    pub(crate) minus_sign: xot::NameId,
    pub(crate) mode: xot::NameId,
    pub(crate) name: xot::NameId,
    pub(crate) names: xot::NameId,
    pub(crate) namespace: xot::NameId,
    pub(crate) namespace_context: xot::NameId,
    pub(crate) normalization_form: xot::NameId,
    pub(crate) nan: xot::NameId,
    pub(crate) new_each_time: xot::NameId,
    pub(crate) omit_xml_declaration: xot::NameId,
    pub(crate) on_multiple_match: xot::NameId,
    pub(crate) on_no_match: xot::NameId,
    pub(crate) order: xot::NameId,
    pub(crate) ordinal: xot::NameId,
    pub(crate) override_: xot::NameId,
    pub(crate) override_extension_function: xot::NameId,
    pub(crate) parameter_document: xot::NameId,
    pub(crate) pattern_separator: xot::NameId,
    pub(crate) phase: xot::NameId,
    pub(crate) percent: xot::NameId,
    pub(crate) per_mille: xot::NameId,
    pub(crate) priority: xot::NameId,
    pub(crate) regex: xot::NameId,
    pub(crate) required: xot::NameId,
    pub(crate) result_prefix: xot::NameId,
    pub(crate) schema_aware: xot::NameId,
    pub(crate) schema_location: xot::NameId,
    pub(crate) select: xot::NameId,
    pub(crate) separator: xot::NameId,
    pub(crate) sort_before_merge: xot::NameId,
    pub(crate) stable: xot::NameId,
    pub(crate) standalone: xot::NameId,
    pub(crate) start_at: xot::NameId,
    pub(crate) static_: xot::NameId,
    pub(crate) streamable: xot::NameId,
    pub(crate) streamability: xot::NameId,
    pub(crate) string: xot::NameId,
    pub(crate) stylesheet_prefix: xot::NameId,
    pub(crate) suppress_indentation: xot::NameId,
    pub(crate) terminate: xot::NameId,
    pub(crate) test: xot::NameId,
    pub(crate) tunnel: xot::NameId,
    pub(crate) type_: xot::NameId,
    pub(crate) typed: xot::NameId,
    pub(crate) undeclare_prefixes: xot::NameId,
    pub(crate) use_: xot::NameId,
    pub(crate) use_accumulators: xot::NameId,
    pub(crate) use_attribute_sets: xot::NameId,
    pub(crate) use_character_maps: xot::NameId,
    pub(crate) validation: xot::NameId,
    pub(crate) value: xot::NameId,
    pub(crate) version: xot::NameId,
    pub(crate) visibility: xot::NameId,
    pub(crate) warning_on_multiple_match: xot::NameId,
    pub(crate) warning_on_no_match: xot::NameId,
    pub(crate) with_params: xot::NameId,
    pub(crate) xpath: xot::NameId,
    pub(crate) zero_digit: xot::NameId,

    // standard attributes on XSLT elements
    pub(crate) standard: StandardNames,
    // standard attributes on literal result elements
    pub(crate) xsl_standard: StandardNames,
}

pub(crate) struct StandardNames {
    pub(crate) default_collation: xot::NameId,
    pub(crate) default_mode: xot::NameId,
    pub(crate) default_validation: xot::NameId,
    pub(crate) exclude_result_prefixes: xot::NameId,
    pub(crate) expand_text: xot::NameId,
    pub(crate) extension_element_prefixes: xot::NameId,
    pub(crate) use_when: xot::NameId,
    pub(crate) version: xot::NameId,
    pub(crate) xpath_default_namespace: xot::NameId,
}

impl StandardNames {
    fn no_ns(xot: &mut Xot) -> Self {
        Self {
            default_collation: xot.add_name("default-collation"),
            default_mode: xot.add_name("default-mode"),
            default_validation: xot.add_name("default-validation"),
            exclude_result_prefixes: xot.add_name("exclude-result-prefixes"),
            expand_text: xot.add_name("expand-text"),
            extension_element_prefixes: xot.add_name("extension-element-prefixes"),
            use_when: xot.add_name("use-when"),
            version: xot.add_name("version"),
            xpath_default_namespace: xot.add_name("xpath-default-namespace"),
        }
    }

    fn xsl(xot: &mut Xot, xsl_ns: NamespaceId) -> Self {
        Self {
            default_collation: xot.add_name_ns("default-collation", xsl_ns),
            default_mode: xot.add_name_ns("default-mode", xsl_ns),
            default_validation: xot.add_name_ns("default-validation", xsl_ns),
            exclude_result_prefixes: xot.add_name_ns("exclude-result-prefixes", xsl_ns),
            expand_text: xot.add_name_ns("expand-text", xsl_ns),
            extension_element_prefixes: xot.add_name_ns("extension-element-prefixes", xsl_ns),
            use_when: xot.add_name_ns("use-when", xsl_ns),
            version: xot.add_name_ns("version", xsl_ns),
            xpath_default_namespace: xot.add_name_ns("xpath-default-namespace", xsl_ns),
        }
    }
}

impl Names {
    pub(crate) fn new(xot: &mut Xot) -> Self {
        let xsl_ns = xot.add_namespace("http://www.w3.org/1999/XSL/Transform");

        let ignore_xml_space_parents = [
            xot.add_name_ns("accumulator", xsl_ns),
            xot.add_name_ns("analyze-string", xsl_ns),
            xot.add_name_ns("apply-imports", xsl_ns),
            xot.add_name_ns("apply-templates", xsl_ns),
            xot.add_name_ns("attribute-set", xsl_ns),
            xot.add_name_ns("call-template", xsl_ns),
            xot.add_name_ns("character-map", xsl_ns),
            xot.add_name_ns("choose", xsl_ns),
            xot.add_name_ns("evaluate", xsl_ns),
            xot.add_name_ns("fork", xsl_ns),
            xot.add_name_ns("merge", xsl_ns),
            xot.add_name_ns("merge-source", xsl_ns),
            xot.add_name_ns("mode", xsl_ns),
            xot.add_name_ns("next-iteration", xsl_ns),
            xot.add_name_ns("next-match", xsl_ns),
            xot.add_name_ns("override", xsl_ns),
            xot.add_name_ns("package", xsl_ns),
            xot.add_name_ns("stylesheet", xsl_ns),
            xot.add_name_ns("transform", xsl_ns),
            xot.add_name_ns("use-package", xsl_ns),
        ]
        .iter()
        .copied()
        .collect();

        let ignore_xml_space_next_siblings = [
            xot.add_name_ns("param", xsl_ns),
            xot.add_name_ns("sort", xsl_ns),
            xot.add_name_ns("context-item", xsl_ns),
            xot.add_name_ns("on-completion", xsl_ns),
        ]
        .iter()
        .copied()
        .collect();

        let ignore_xml_space_previous_siblings =
            [xot.add_name_ns("catch", xsl_ns)].iter().copied().collect();

        Self {
            xsl_ns,

            sequence_constructor_names: SequenceConstructorName::names(xot, xsl_ns),
            declaration_names: DeclarationName::names(xot, xsl_ns),
            override_content_names: OverrideContentName::names(xot, xsl_ns),
            ignore_xml_space_parents,
            ignore_xml_space_next_siblings,
            ignore_xml_space_previous_siblings,

            xsl_accumulator_rule: xot.add_name_ns("accumulator-rule", xsl_ns),
            xsl_attribute: xot.add_name_ns("attribute", xsl_ns),
            xsl_fallback: xot.add_name_ns("fallback", xsl_ns),
            xsl_for_each: xot.add_name_ns("for-each", xsl_ns),
            xsl_for_each_group: xot.add_name_ns("for-each-group", xsl_ns),
            xsl_matching_substring: xot.add_name_ns("matching-substring", xsl_ns),
            xsl_merge_action: xot.add_name_ns("merge-action", xsl_ns),
            xsl_merge_key: xot.add_name_ns("merge-key", xsl_ns),
            xsl_merge_source: xot.add_name_ns("merge-source", xsl_ns),
            xsl_non_matching_substring: xot.add_name_ns("non-matching-substring", xsl_ns),
            xsl_on_completion: xot.add_name_ns("on-completion", xsl_ns),
            xsl_otherwise: xot.add_name_ns("otherwise", xsl_ns),
            xsl_output_character: xot.add_name_ns("output-character", xsl_ns),
            xsl_param: xot.add_name_ns("param", xsl_ns),
            xsl_schema: xot.add_name_ns("schema", xsl_ns),
            xsl_sequence: xot.add_name_ns("sequence", xsl_ns),
            xsl_sort: xot.add_name_ns("sort", xsl_ns),
            xsl_text: xot.add_name_ns("text", xsl_ns),
            xsl_transform: xot.add_name_ns("transform", xsl_ns),
            xsl_variable: xot.add_name_ns("variable", xsl_ns),
            xsl_when: xot.add_name_ns("when", xsl_ns),
            xsl_with_param: xot.add_name_ns("with-param", xsl_ns),

            as_: xot.add_name("as"),
            allow_duplicate_names: xot.add_name("allow-duplicate-names"),
            base_uri: xot.add_name("base-uri"),
            build_tree: xot.add_name("build-tree"),
            byte_order_mark: xot.add_name("byte-order-mark"),
            case_order: xot.add_name("case-order"),
            cache: xot.add_name("cache"),
            cdata_section_elements: xot.add_name("cdata-section-elements"),
            character: xot.add_name("character"),
            collation: xot.add_name("collation"),
            component: xot.add_name("component"),
            composite: xot.add_name("composite"),
            context_item: xot.add_name("context-item"),
            copy_accumulators: xot.add_name("copy-accumulators"),
            copy_namespaces: xot.add_name("copy-namespaces"),
            count: xot.add_name("count"),
            data_type: xot.add_name("data-type"),
            decimal_separator: xot.add_name("decimal-separator"),
            digit: xot.add_name("digit"),
            disable_output_escaping: xot.add_name("disable-output-escaping"),
            doctype_public: xot.add_name("doctype-public"),
            doctype_system: xot.add_name("doctype-system"),
            elements: xot.add_name("elements"),
            encoding: xot.add_name("encoding"),
            error_code: xot.add_name("error-code"),
            errors: xot.add_name("errors"),
            escape_uri_attributes: xot.add_name("escape-uri-attributes"),
            exponent_separator: xot.add_name("exponent-separator"),
            extension_element_prefixes: xot.add_name("extension-element-prefixes"),
            flags: xot.add_name("flags"),
            for_each_item: xot.add_name("for-each-item"),
            for_each_source: xot.add_name("for-each-source"),
            format: xot.add_name("format"),
            from: xot.add_name("from"),
            group_adjacent: xot.add_name("group-adjacent"),
            group_by: xot.add_name("group-by"),
            group_ending_with: xot.add_name("group-ending-with"),
            group_starting_with: xot.add_name("group-starting-with"),
            grouping_separator: xot.add_name("grouping-separator"),
            grouping_size: xot.add_name("grouping-size"),
            href: xot.add_name("href"),
            html_version: xot.add_name("html-version"),
            id: xot.add_name("id"),
            include_content_type: xot.add_name("include-content-type"),
            indent: xot.add_name("indent"),
            infinity: xot.add_name("infinity"),
            inherit_namespaces: xot.add_name("inherit-namespaces"),
            initial_value: xot.add_name("initial-value"),
            input_type_annotations: xot.add_name("input-type-annotations"),
            item_separator: xot.add_name("item-separator"),
            json_node_output_method: xot.add_name("json-node-output-method"),
            lang: xot.add_name("language"),
            letter_value: xot.add_name("letter-value"),
            level: xot.add_name("level"),
            key: xot.add_name("key"),
            match_: xot.add_name("match"),
            media_type: xot.add_name("media-type"),
            method: xot.add_name("method"),
            minus_sign: xot.add_name("minus-sign"),
            mode: xot.add_name("mode"),
            name: xot.add_name("name"),
            names: xot.add_name("names"),
            nan: xot.add_name("NaN"),
            namespace: xot.add_name("namespace"),
            namespace_context: xot.add_name("namespace-context"),
            new_each_time: xot.add_name("new-each-time"),
            normalization_form: xot.add_name("normalization-form"),
            omit_xml_declaration: xot.add_name("omit-xml-declaration"),
            on_multiple_match: xot.add_name("on-multiple-match"),
            on_no_match: xot.add_name("on-no-match"),
            order: xot.add_name("order"),
            ordinal: xot.add_name("ordinal"),
            override_: xot.add_name("override"),
            override_extension_function: xot.add_name("override-extension-function"),
            pattern_separator: xot.add_name("pattern-separator"),
            parameter_document: xot.add_name("parameter-document"),
            phase: xot.add_name("phase"),
            percent: xot.add_name("percent"),
            per_mille: xot.add_name("per-mille"),
            priority: xot.add_name("priority"),
            regex: xot.add_name("regex"),
            required: xot.add_name("required"),
            result_prefix: xot.add_name("result-prefix"),
            schema_aware: xot.add_name("schema-aware"),
            schema_location: xot.add_name("schema-location"),
            select: xot.add_name("select"),
            separator: xot.add_name("separator"),
            sort_before_merge: xot.add_name("sort-before-merge"),
            stable: xot.add_name("stable"),
            standalone: xot.add_name("standalone"),
            start_at: xot.add_name("start-at"),
            static_: xot.add_name("static"),
            streamability: xot.add_name("streamability"),
            streamable: xot.add_name("streamable"),
            string: xot.add_name("string"),
            stylesheet_prefix: xot.add_name("stylesheet-prefix"),
            suppress_indentation: xot.add_name("suppress-indentation"),
            terminate: xot.add_name("terminate"),
            test: xot.add_name("test"),
            tunnel: xot.add_name("tunnel"),
            type_: xot.add_name("type"),
            typed: xot.add_name("typed"),
            use_: xot.add_name("use"),
            use_accumulators: xot.add_name("use-accumulators"),
            use_attribute_sets: xot.add_name("use-attribute-sets"),
            use_character_maps: xot.add_name("use-character-maps"),
            validation: xot.add_name("validation"),
            visibility: xot.add_name("visibility"),
            undeclare_prefixes: xot.add_name("undeclare-prefixes"),
            value: xot.add_name("value"),
            version: xot.add_name("version"),
            warning_on_no_match: xot.add_name("warning-on-no-match"),
            warning_on_multiple_match: xot.add_name("warning-on-multiple-match"),
            with_params: xot.add_name("with-params"),
            xpath: xot.add_name("xpath"),
            zero_digit: xot.add_name("zero-digit"),

            // standard attributes
            standard: StandardNames::no_ns(xot),
            // standard attributes on literal result elements
            xsl_standard: StandardNames::xsl(xot, xsl_ns),
        }
    }

    pub(crate) fn sequence_constructor_name(
        &self,
        name: NameId,
    ) -> Option<SequenceConstructorName> {
        self.sequence_constructor_names.get(&name).copied()
    }

    pub(crate) fn declaration_name(&self, name: NameId) -> Option<DeclarationName> {
        self.declaration_names.get(&name).copied()
    }

    pub(crate) fn override_content_name(&self, name: NameId) -> Option<OverrideContentName> {
        self.override_content_names.get(&name).copied()
    }
}
