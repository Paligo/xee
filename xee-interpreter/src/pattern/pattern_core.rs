use xee_xpath_type::ast::KindTest;
use xot::Xot;

use xee_xpath_ast::pattern;

use crate::function::InlineFunctionId;
use crate::sequence::Item;
use crate::xml;

pub(crate) enum NodeMatch {
    Match(Option<xot::Node>),
    NotMatch,
}

pub(crate) trait PredicateMatcher {
    fn match_predicate(&mut self, inline_function_id: InlineFunctionId, item: &Item) -> bool;
    fn xot(&self) -> &Xot;

    fn matches(&mut self, pattern: &pattern::Pattern<InlineFunctionId>, item: &Item) -> bool {
        match pattern {
            pattern::Pattern::Expr(expr_pattern) => self.matches_expr_pattern(expr_pattern, item),
            pattern::Pattern::Predicate(predicate_pattern) => {
                self.matches_predicate_pattern(predicate_pattern, item)
            }
        }
    }

    fn matches_expr_pattern(
        &mut self,
        expr_pattern: &pattern::ExprPattern<InlineFunctionId>,
        item: &Item,
    ) -> bool {
        if let Item::Node(node) = item {
            match expr_pattern {
                pattern::ExprPattern::Path(path_expr) => self.matches_path_expr(path_expr, *node),
                pattern::ExprPattern::BinaryExpr(binary_expr) => {
                    self.matches_binary_expr(binary_expr, *node)
                }
            }
        } else {
            false
        }
    }

    fn matches_predicate_pattern(
        &mut self,
        predicate_pattern: &pattern::PredicatePattern<InlineFunctionId>,
        item: &Item,
    ) -> bool {
        for predicate in &predicate_pattern.predicates {
            if !self.match_predicate(*predicate, item) {
                return false;
            }
        }
        true
    }

    fn matches_binary_expr(
        &mut self,
        binary_expr: &pattern::BinaryExpr<InlineFunctionId>,
        node: xot::Node,
    ) -> bool {
        match binary_expr.operator {
            pattern::Operator::Union => {
                self.matches_expr_pattern(&binary_expr.left, &Item::from(node))
                    || self.matches_expr_pattern(&binary_expr.right, &Item::from(node))
            }
            pattern::Operator::Intersect => {
                self.matches_expr_pattern(&binary_expr.left, &Item::from(node))
                    && self.matches_expr_pattern(&binary_expr.right, &Item::from(node))
            }
            pattern::Operator::Except => {
                self.matches_expr_pattern(&binary_expr.left, &Item::from(node))
                    && !self.matches_expr_pattern(&binary_expr.right, &Item::from(node))
            }
        }
    }

    fn matches_path_expr(
        &mut self,
        path_expr: &pattern::PathExpr<InlineFunctionId>,
        node: xot::Node,
    ) -> bool {
        match &path_expr.root {
            // this one awaits support for variables in patterns. also there are various
            // possible function calls
            pattern::PathRoot::Rooted {
                root: _,
                predicates: _,
            } => todo!(),
            pattern::PathRoot::AbsoluteSlash => self.matches_absolute_steps(&path_expr.steps, node),
            pattern::PathRoot::AbsoluteDoubleSlash => {
                self.matches_absolute_double_slash_steps(&path_expr.steps, node)
            }
            pattern::PathRoot::Relative => {
                match self.matches_relative_steps(&path_expr.steps, node) {
                    NodeMatch::Match(_) => true,
                    NodeMatch::NotMatch => false,
                }
            }
        }
    }

    fn matches_absolute_steps(
        &mut self,
        steps: &[pattern::StepExpr<InlineFunctionId>],
        node: xot::Node,
    ) -> bool {
        let node_match = self.matches_relative_steps(steps, node);
        if let NodeMatch::Match(Some(node)) = node_match {
            self.xot().is_document(node)
        } else {
            false
        }
    }

    fn matches_absolute_double_slash_steps(
        &mut self,
        steps: &[pattern::StepExpr<InlineFunctionId>],
        node: xot::Node,
    ) -> bool {
        let node_match = self.matches_relative_steps(steps, node);
        if let NodeMatch::Match(Some(node)) = node_match {
            // we need to be under root
            let mut current_node = node;
            loop {
                if self.xot().is_document(current_node) {
                    return true;
                }
                if let Some(parent) = self.xot().parent(current_node) {
                    current_node = parent;
                } else {
                    return false;
                }
            }
        } else {
            false
        }
    }

    fn matches_relative_steps(
        &mut self,
        steps: &[pattern::StepExpr<InlineFunctionId>],
        node: xot::Node,
    ) -> NodeMatch {
        let mut node = Some(node);
        let mut axis = pattern::ForwardAxis::Child;
        for step in steps.iter().rev() {
            loop {
                if let Some(n) = node {
                    let (matches, new_axis) = self.matches_step_expr(step, n);
                    match axis {
                        pattern::ForwardAxis::Descendant => {
                            if !matches {
                                node = self.xot().parent(n);
                                continue;
                            }
                            axis = new_axis;
                            break;
                        }
                        // TODO: handle other kinds of forward axes, right now make
                        // them NotMatch
                        pattern::ForwardAxis::Self_ => return NodeMatch::NotMatch,
                        pattern::ForwardAxis::DescendantOrSelf => return NodeMatch::NotMatch,
                        pattern::ForwardAxis::Namespace => return NodeMatch::NotMatch,
                        _ => {
                            if !matches {
                                return NodeMatch::NotMatch;
                            }
                            axis = new_axis;
                            break;
                        }
                    }
                } else {
                    return NodeMatch::NotMatch;
                }
            }
            if let Some(n) = node {
                node = self.xot().parent(n);
            } else {
                return NodeMatch::NotMatch;
            }
        }
        NodeMatch::Match(node)
    }

    fn matches_step_expr(
        &mut self,
        step: &pattern::StepExpr<InlineFunctionId>,
        node: xot::Node,
    ) -> (bool, pattern::ForwardAxis) {
        match step {
            pattern::StepExpr::AxisStep(axis_step) => self.matches_axis_step(axis_step, node),
            // does the child forward axis make sense here?
            pattern::StepExpr::PostfixExpr(postfix_expr) => (
                self.matches_postfix_expr(postfix_expr, node),
                pattern::ForwardAxis::Child,
            ),
        }
    }

    fn matches_axis_step(
        &mut self,
        step: &pattern::AxisStep<InlineFunctionId>,
        node: xot::Node,
    ) -> (bool, pattern::ForwardAxis) {
        // if the forward axis is attribute based, we won't match with an element,
        // and vice versa
        if self.xot().is_attribute_node(node) {
            if step.forward != pattern::ForwardAxis::Attribute {
                return (false, step.forward);
            }
        } else if step.forward == pattern::ForwardAxis::Attribute {
            return (false, step.forward);
        }
        if !Self::matches_node_test(&step.node_test, node, self.xot()) {
            return (false, step.forward);
        }
        // if we have a match, check whether the predicates apply
        let item = Item::Node(node);
        for predicate in &step.predicates {
            if !self.match_predicate(*predicate, &item) {
                return (false, step.forward);
            }
        }
        (true, step.forward)
    }

    fn matches_postfix_expr(
        &mut self,
        postfix_expr: &pattern::PostfixExpr<InlineFunctionId>,
        node: xot::Node,
    ) -> bool {
        if !self.matches_expr_pattern(&postfix_expr.expr, &Item::from(node)) {
            return false;
        }
        let item = Item::Node(node);
        for predicate in &postfix_expr.predicates {
            if !self.match_predicate(*predicate, &item) {
                return false;
            }
        }
        true
    }

    fn matches_node_test(node_test: &pattern::NodeTest, node: xot::Node, xot: &Xot) -> bool {
        match node_test {
            pattern::NodeTest::NameTest(name_test) => Self::matches_name_test(name_test, node, xot),
            pattern::NodeTest::KindTest(kind_test) => Self::matches_kind_test(kind_test, node, xot),
        }
    }

    fn matches_name_test(name_test: &pattern::NameTest, node: xot::Node, xot: &Xot) -> bool {
        match name_test {
            pattern::NameTest::Name(expected_name) => {
                // TODO: unwrap - what if prefix couldn't be identified?
                if let Some(node_name) = xot.node_name_ref(node).unwrap() {
                    expected_name.value.maybe_to_ref(xot) == Some(node_name)
                } else {
                    false
                }
            }
            pattern::NameTest::Star => true,
            pattern::NameTest::LocalName(expected_local_name) => {
                if let Some(name) = xot.node_name(node) {
                    xot.local_name_str(name) == expected_local_name
                } else {
                    false
                }
            }
            pattern::NameTest::Namespace(ns) => {
                if let Some(name) = xot.node_name(node) {
                    let namespace_uri = xot.uri_str(name);
                    namespace_uri == ns
                } else {
                    false
                }
            }
        }
    }

    fn matches_kind_test(kind_test: &KindTest, node: xot::Node, xot: &Xot) -> bool {
        xml::kind_test(kind_test, xot, node)
    }
}

#[cfg(test)]
mod tests {
    use xee_xpath_ast::pattern::transform_pattern;

    use crate::atomic::Atomic;

    use super::*;

    fn parse_pattern(pattern: &str) -> pattern::Pattern<InlineFunctionId> {
        let namespaces = xee_name::Namespaces::default();
        parse_pattern_namespaces(pattern, &namespaces)
    }

    fn parse_pattern_namespaces(
        pattern: &str,
        namespaces: &xee_name::Namespaces,
    ) -> pattern::Pattern<InlineFunctionId> {
        let variable_names = xee_name::VariableNames::default();
        let pattern = pattern::Pattern::parse(pattern, namespaces, &variable_names).unwrap();

        transform_pattern(&pattern, |_expr| dummy_inline_function_id()).unwrap()
    }

    fn dummy_inline_function_id() -> Result<InlineFunctionId, ()> {
        Ok(InlineFunctionId::new(0))
    }

    struct BasicPredicateMatcher<'a> {
        xot: &'a Xot,
        predicate_matches: bool,
    }

    impl<'a> BasicPredicateMatcher<'a> {
        fn new(xot: &'a Xot) -> Self {
            Self {
                xot,
                predicate_matches: false,
            }
        }

        fn matching(xot: &'a Xot) -> Self {
            Self {
                xot,
                predicate_matches: true,
            }
        }
    }

    impl PredicateMatcher for BasicPredicateMatcher<'_> {
        fn match_predicate(&mut self, _inline_function_id: InlineFunctionId, _item: &Item) -> bool {
            self.predicate_matches
        }

        fn xot(&self) -> &Xot {
            self.xot
        }
    }

    #[test]
    fn test_match_name() {
        let mut xot = Xot::new();

        let root = xot.parse(r#"<root><foo/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();

        let item: Item = node.into();

        let pattern = parse_pattern("foo");

        let mut pm = BasicPredicateMatcher::new(&xot);
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_star() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><foo/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let pattern = parse_pattern("*");

        let mut pm = BasicPredicateMatcher::new(&xot);
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_local_name() {
        let mut xot = Xot::new();
        let root = xot
            .parse(r#"<root><foo xmlns="different"/></root>"#)
            .unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("*:foo");
        assert!(pm.matches(&pattern, &item));
        let pattern = parse_pattern("foo");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_namespace_name() {
        let mut xot = Xot::new();
        let root = xot
            .parse(r#"<root><foo xmlns="different"/></root>"#)
            .unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let namespaces = xee_name::Namespaces::new(
            vec![
                ("d".to_string(), "different".to_string()),
                ("o".to_string(), "other".to_string()),
            ]
            .into_iter()
            .collect(),
            "".to_string(),
            "".to_string(),
        );

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern_namespaces("d:*", &namespaces);
        assert!(pm.matches(&pattern, &item));
        let pattern = parse_pattern_namespaces("d:foo", &namespaces);
        assert!(pm.matches(&pattern, &item));
        let pattern = parse_pattern_namespaces("o:*", &namespaces);
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><foo/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("notfound");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_name_nested() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><bar><foo/></bar></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.first_child(node).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("bar/foo");
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_nested() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><bar><foo/></bar></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("notfound/foo");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_name_nested_with_explicit_descendant_axis() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><bar><foo/></bar></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.first_child(node).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("bar/descendant::foo");
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_nested_with_explicit_descendant_axis() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><bar><foo/></bar></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.first_child(node).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("notfound/descendant::foo");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_name_nested_actually_with_explicit_descendant_axis() {
        let mut xot = Xot::new();
        let root = xot
            .parse(r#"<root><qux><bar><foo/></bar></qux></root>"#)
            .unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.first_child(node).unwrap();
        let node = xot.first_child(node).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("qux/descendant::foo");
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_nested_actually_with_explicit_descendant_axis() {
        let mut xot = Xot::new();
        let root = xot
            .parse(r#"<root><qux><bar><foo/></bar></qux></root>"#)
            .unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.first_child(node).unwrap();
        let node = xot.first_child(node).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("notfound/descendant::foo");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_name_attribute() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let root = xot.parse(r#"<root><foo bar="BAR"/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.attributes(node).get_node(bar_name).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("@bar");
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_attribute() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let root = xot.parse(r#"<root><foo bar="BAR"/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.attributes(node).get_node(bar_name).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("@qux");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_element_because_its_attribute() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let root = xot.parse(r#"<root><foo bar="BAR"/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.attributes(node).get_node(bar_name).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("bar");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_element_descendant_because_its_attribute() {
        let mut xot = Xot::new();
        let bar_name = xot.add_name("bar");
        let root = xot.parse(r#"<root><foo bar="BAR"/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let node = xot.attributes(node).get_node(bar_name).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);

        let pattern = parse_pattern("root//bar");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_not_match_name_attribute_because_its_element() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><bar /></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("@bar");
        assert!(!pm.matches(&pattern, &item));
    }

    #[test]
    fn test_matches_kind_test_any() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><foo bar="BAR" /></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let element_node = node;
        let element_item: Item = element_node.into();
        let attribute_name = xot.add_name("bar");
        let attribute_node = xot.attributes(node).get_node(attribute_name).unwrap();
        let attribute_item: Item = attribute_node.into();

        // the axis determines whether we match. This is a bit
        // counter intuitive, but the spec affirms it.

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("node()");
        assert!(pm.matches(&pattern, &element_item));
        assert!(!pm.matches(&pattern, &attribute_item));
        let pattern = parse_pattern("attribute::node()");
        assert!(!pm.matches(&pattern, &element_item));
        assert!(pm.matches(&pattern, &attribute_item));
    }

    #[test]
    fn test_matches_kind_test_element() {
        let mut xot = Xot::new();
        let root = xot
            .parse(r#"<root><foo bar="BAR">text</foo></root>"#)
            .unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let element_node = node;
        let element_item: Item = element_node.into();
        let attribute_name = xot.add_name("bar");
        let attribute_node = xot.attributes(node).get_node(attribute_name).unwrap();
        let attribute_item: Item = attribute_node.into();
        let text_node = xot.first_child(node).unwrap();
        let text_item: Item = text_node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("element()");
        assert!(pm.matches(&pattern, &element_item));
        assert!(!pm.matches(&pattern, &text_item));
        assert!(!pm.matches(&pattern, &attribute_item));
    }

    #[test]
    fn test_matches_absolute_slash() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root/>"#).unwrap();
        let item: Item = root.into();
        let document_element = xot.document_element(root).unwrap();
        let document_element_item: Item = document_element.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("/");
        assert!(pm.matches(&pattern, &item));
        assert!(!pm.matches(&pattern, &document_element_item));
    }

    #[test]
    fn test_matches_absolute_slash_with_element() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root/>"#).unwrap();
        let item: Item = root.into();

        let document_element = xot.document_element(root).unwrap();
        let document_element_item: Item = document_element.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("/root");
        assert!(!pm.matches(&pattern, &item));
        assert!(pm.matches(&pattern, &document_element_item));
    }

    #[test]
    fn test_matches_absolute_double_slash() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root/>"#).unwrap();
        let item: Item = root.into();

        let document_element = xot.document_element(root).unwrap();
        let document_element_item: Item = document_element.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("//root");
        assert!(!pm.matches(&pattern, &item));
        assert!(pm.matches(&pattern, &document_element_item));
    }

    #[test]
    fn test_matches_absolute_double_slash_nesting() {
        let mut xot = Xot::new();
        let root = xot.parse(r#"<root><foo/></root>"#).unwrap();
        let item: Item = root.into();
        let document_element = xot.document_element(root).unwrap();
        let document_element_item: Item = document_element.into();
        let foo_node = xot.first_child(document_element).unwrap();
        let foo_item: Item = foo_node.into();

        let mut pm = BasicPredicateMatcher::new(&xot);
        let pattern = parse_pattern("//root/foo");
        assert!(!pm.matches(&pattern, &item));
        assert!(!pm.matches(&pattern, &document_element_item));
        assert!(pm.matches(&pattern, &foo_item));
    }

    #[test]
    fn test_predicate_pattern_matches() {
        let xot = Xot::new();
        let mut pm = BasicPredicateMatcher::matching(&xot);
        let atom: Atomic = 1.into();
        let item: Item = atom.into();
        let pattern = parse_pattern(".[. instance of xs:integer]");
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_match_name_with_predicate() {
        let mut xot = Xot::new();

        let root = xot.parse(r#"<root><foo/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let node = xot.first_child(document_element).unwrap();
        let item: Item = node.into();

        let pattern = parse_pattern("foo[1]");

        let mut pm = BasicPredicateMatcher::matching(&xot);
        assert!(pm.matches(&pattern, &item));
    }

    #[test]
    fn test_binary_expr_union() {
        let mut xot = Xot::new();

        let root = xot.parse(r#"<root><foo/><bar/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let foo_node = xot.first_child(document_element).unwrap();
        let bar_node = xot.next_sibling(foo_node).unwrap();
        let foo_item: Item = foo_node.into();
        let bar_item: Item = bar_node.into();

        let pattern = parse_pattern("foo | bar");

        let mut pm = BasicPredicateMatcher::new(&xot);
        assert!(pm.matches(&pattern, &foo_item));
        assert!(pm.matches(&pattern, &bar_item));
    }

    #[test]
    fn test_binary_expr_intersection() {
        let mut xot = Xot::new();

        let root = xot.parse(r#"<root><foo/><bar/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let foo_node = xot.first_child(document_element).unwrap();
        let bar_node = xot.next_sibling(foo_node).unwrap();
        let foo_item: Item = foo_node.into();
        let bar_item: Item = bar_node.into();

        let pattern = parse_pattern("(foo | bar) intersect foo");

        let mut pm = BasicPredicateMatcher::new(&xot);
        assert!(pm.matches(&pattern, &foo_item));
        assert!(!pm.matches(&pattern, &bar_item));
    }

    #[test]
    fn test_binary_expr_except() {
        let mut xot = Xot::new();

        let root = xot.parse(r#"<root><foo/><bar/></root>"#).unwrap();
        let document_element = xot.document_element(root).unwrap();
        let foo_node = xot.first_child(document_element).unwrap();
        let bar_node = xot.next_sibling(foo_node).unwrap();
        let foo_item: Item = foo_node.into();
        let bar_item: Item = bar_node.into();

        let pattern = parse_pattern("(foo | bar) except foo");

        let mut pm = BasicPredicateMatcher::new(&xot);
        assert!(!pm.matches(&pattern, &foo_item));
        assert!(pm.matches(&pattern, &bar_item));
    }
}
