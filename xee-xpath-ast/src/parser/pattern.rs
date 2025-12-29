use xot::xmlname::NameStrInfo;

use crate::ast::Span;
use crate::{ast, FN_NAMESPACE};
use crate::{pattern, Namespaces, ParserError, VariableNames};

struct RootStep {
    root: pattern::RootExpr,
    predicates: Vec<ast::ExprS>,
}

fn unsupported_pattern(span: Span) -> ParserError {
    ParserError::ExpectedFound { span }
}

fn span_is_empty(span: Span) -> bool {
    span.start == span.end
}

fn span_len(span: Span) -> usize {
    span.end.saturating_sub(span.start)
}

fn expr_to_pattern(
    expr: &ast::Expr,
    span: Span,
) -> Result<pattern::Pattern<ast::ExprS>, ParserError> {
    if expr.0.len() != 1 {
        return Err(unsupported_pattern(span));
    }
    expr_single_to_pattern(&expr.0[0])
}

fn expr_to_expr_pattern(
    expr: &ast::Expr,
    span: Span,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    if expr.0.len() != 1 {
        return Err(unsupported_pattern(span));
    }
    expr_single_to_expr_pattern(&expr.0[0])
}

fn expr_single_to_pattern(
    expr_single: &ast::ExprSingleS,
) -> Result<pattern::Pattern<ast::ExprS>, ParserError> {
    match &expr_single.value {
        ast::ExprSingle::Path(path_expr) => {
            if let Some(predicates) = context_item_predicates(path_expr)? {
                return Ok(pattern::Pattern::Predicate(pattern::PredicatePattern {
                    predicates,
                }));
            }
            Ok(pattern::Pattern::Expr(pattern::ExprPattern::Path(
                path_expr_to_pattern(path_expr)?,
            )))
        }
        ast::ExprSingle::Binary(binary_expr) => Ok(pattern::Pattern::Expr(convert_binary_expr(
            binary_expr,
            expr_single.span,
        )?)),
        _ => Err(unsupported_pattern(expr_single.span)),
    }
}

fn expr_single_to_expr_pattern(
    expr_single: &ast::ExprSingleS,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    match &expr_single.value {
        ast::ExprSingle::Path(path_expr) => path_expr_to_expr_pattern(path_expr),
        ast::ExprSingle::Binary(binary_expr) => {
            convert_binary_expr(binary_expr, expr_single.span)
        }
        _ => Err(unsupported_pattern(expr_single.span)),
    }
}

fn convert_binary_expr(
    binary_expr: &ast::BinaryExpr,
    span: Span,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    let operator = match binary_expr.operator {
        ast::BinaryOperator::Union => pattern::Operator::Union,
        ast::BinaryOperator::Intersect => pattern::Operator::Intersect,
        ast::BinaryOperator::Except => pattern::Operator::Except,
        _ => return Err(unsupported_pattern(span)),
    };

    let left = path_expr_to_expr_pattern(&binary_expr.left)?;
    let right = path_expr_to_expr_pattern(&binary_expr.right)?;
    Ok(pattern::ExprPattern::BinaryExpr(pattern::BinaryExpr {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }))
}

fn path_expr_to_expr_pattern(
    path_expr: &ast::PathExpr,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    if context_item_predicates(path_expr)?.is_some() {
        let span = path_expr
            .steps
            .first()
            .map(|step| step.span)
            .unwrap_or_else(|| Span::new(0, 0));
        return Err(unsupported_pattern(span));
    }
    Ok(pattern::ExprPattern::Path(path_expr_to_pattern(
        path_expr,
    )?))
}

fn path_expr_to_pattern(
    path_expr: &ast::PathExpr,
) -> Result<pattern::PathExpr<ast::ExprS>, ParserError> {
    if context_item_predicates(path_expr)?.is_some() {
        let span = path_expr
            .steps
            .first()
            .map(|step| step.span)
            .unwrap_or_else(|| Span::new(0, 0));
        return Err(unsupported_pattern(span));
    }

    let steps = &path_expr.steps;
    if steps.is_empty() {
        return Err(unsupported_pattern(Span::new(0, 0)));
    }

    if let Some((root, start_index)) = implicit_root_info(steps) {
        if matches!(root, pattern::PathRoot::AbsoluteDoubleSlash) && steps.len() <= start_index {
            return Err(unsupported_pattern(steps[0].span));
        }
        let steps = convert_steps(&steps[start_index..])?;
        return Ok(pattern::PathExpr { root, steps });
    }

    let (root, start_index) = if let Some(root_step) = root_step_info(steps.get(0))? {
        (
            pattern::PathRoot::Rooted {
                root: root_step.root,
                predicates: root_step.predicates,
            },
            1,
        )
    } else {
        let steps = convert_steps(steps)?;
        return Ok(finalize_relative_path(steps));
    };

    let steps = convert_steps(&steps[start_index..])?;
    Ok(pattern::PathExpr { root, steps })
}

fn finalize_relative_path(
    steps: Vec<pattern::StepExpr<ast::ExprS>>,
) -> pattern::PathExpr<ast::ExprS> {
    if steps.len() == 1 {
        if let pattern::StepExpr::PostfixExpr(postfix_expr) = &steps[0] {
            if postfix_expr.predicates.is_empty() {
                if let pattern::ExprPattern::Path(path_expr) = &postfix_expr.expr {
                    return path_expr.clone();
                }
            }
        }
    }
    pattern::PathExpr {
        root: pattern::PathRoot::Relative,
        steps,
    }
}

fn convert_steps(
    steps: &[ast::StepExprS],
) -> Result<Vec<pattern::StepExpr<ast::ExprS>>, ParserError> {
    steps
        .iter()
        .map(convert_step_expr)
        .collect::<Result<Vec<_>, _>>()
}

fn convert_step_expr(
    step_expr: &ast::StepExprS,
) -> Result<pattern::StepExpr<ast::ExprS>, ParserError> {
    match &step_expr.value {
        ast::StepExpr::AxisStep(axis_step) => Ok(pattern::StepExpr::AxisStep(convert_axis_step(
            axis_step,
        ))),
        ast::StepExpr::PostfixExpr { primary, postfixes } => {
            let predicates = collect_predicates(postfixes, step_expr.span)?;
            let expr = primary_expr_to_expr_pattern(primary)?;
            Ok(pattern::StepExpr::PostfixExpr(pattern::PostfixExpr {
                expr,
                predicates,
            }))
        }
        ast::StepExpr::PrimaryExpr(primary) => {
            let expr = primary_expr_to_expr_pattern(primary)?;
            Ok(pattern::StepExpr::PostfixExpr(pattern::PostfixExpr {
                expr,
                predicates: Vec::new(),
            }))
        }
    }
}

fn primary_expr_to_expr_pattern(
    primary: &ast::PrimaryExprS,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    match &primary.value {
        ast::PrimaryExpr::Expr(expr_or_empty) => expr_or_empty_to_expr_pattern(expr_or_empty),
        _ => Err(unsupported_pattern(primary.span)),
    }
}

fn expr_or_empty_to_expr_pattern(
    expr_or_empty: &ast::ExprOrEmptyS,
) -> Result<pattern::ExprPattern<ast::ExprS>, ParserError> {
    match &expr_or_empty.value {
        Some(expr) => expr_to_expr_pattern(expr, expr_or_empty.span),
        None => Err(unsupported_pattern(expr_or_empty.span)),
    }
}

fn convert_axis_step(axis_step: &ast::AxisStep) -> pattern::AxisStep<ast::ExprS> {
    pattern::AxisStep {
        forward: match axis_step.axis {
            ast::Axis::Child => pattern::ForwardAxis::Child,
            ast::Axis::Descendant => pattern::ForwardAxis::Descendant,
            ast::Axis::Attribute => pattern::ForwardAxis::Attribute,
            ast::Axis::Self_ => pattern::ForwardAxis::Self_,
            ast::Axis::DescendantOrSelf => pattern::ForwardAxis::DescendantOrSelf,
            ast::Axis::Namespace => pattern::ForwardAxis::Namespace,
            _ => pattern::ForwardAxis::Child,
        },
        node_test: axis_step.node_test.clone(),
        predicates: axis_step.predicates.clone(),
    }
}

fn collect_predicates(
    postfixes: &[ast::Postfix],
    span: Span,
) -> Result<Vec<ast::ExprS>, ParserError> {
    let mut predicates = Vec::new();
    for postfix in postfixes {
        match postfix {
            ast::Postfix::Predicate(expr) => predicates.push(expr.clone()),
            _ => return Err(unsupported_pattern(span)),
        }
    }
    Ok(predicates)
}

fn context_item_predicates(
    path_expr: &ast::PathExpr,
) -> Result<Option<Vec<ast::ExprS>>, ParserError> {
    if path_expr.steps.len() != 1 {
        return Ok(None);
    }
    let step = &path_expr.steps[0];
    match &step.value {
        ast::StepExpr::PrimaryExpr(primary) => {
            if matches!(primary.value, ast::PrimaryExpr::ContextItem) {
                Ok(Some(Vec::new()))
            } else {
                Ok(None)
            }
        }
        ast::StepExpr::PostfixExpr { primary, postfixes } => {
            if matches!(primary.value, ast::PrimaryExpr::ContextItem) {
                Ok(Some(collect_predicates(postfixes, step.span)?))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn root_step_info(
    step_expr: Option<&ast::StepExprS>,
) -> Result<Option<RootStep>, ParserError> {
    let step_expr = match step_expr {
        Some(step_expr) => step_expr,
        None => return Ok(None),
    };
    match &step_expr.value {
        ast::StepExpr::PrimaryExpr(primary) => {
            if let Some(root) = root_from_primary(primary)? {
                Ok(Some(RootStep {
                    root,
                    predicates: Vec::new(),
                }))
            } else {
                Ok(None)
            }
        }
        ast::StepExpr::PostfixExpr { primary, postfixes } => {
            if let Some(root) = root_from_primary(primary)? {
                let predicates = collect_predicates(postfixes, step_expr.span)?;
                Ok(Some(RootStep { root, predicates }))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn root_from_primary(
    primary: &ast::PrimaryExprS,
) -> Result<Option<pattern::RootExpr>, ParserError> {
    match &primary.value {
        ast::PrimaryExpr::VarRef(name) => Ok(Some(pattern::RootExpr::VarRef(name.clone()))),
        ast::PrimaryExpr::FunctionCall(call) => {
            Ok(Some(pattern::RootExpr::FunctionCall(convert_function_call(
                call, primary.span,
            )?)))
        }
        _ => Ok(None),
    }
}

fn convert_function_call(
    call: &ast::FunctionCall,
    span: Span,
) -> Result<pattern::FunctionCall, ParserError> {
    let name = convert_outer_function_name(&call.name)?;
    if call.arguments.is_empty() {
        return Err(unsupported_pattern(span));
    }
    let args = call
        .arguments
        .iter()
        .map(convert_argument)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(pattern::FunctionCall { name, args })
}

fn convert_outer_function_name(
    name: &ast::NameS,
) -> Result<pattern::OuterFunctionName, ParserError> {
    let value = &name.value;
    if value.namespace() == FN_NAMESPACE || value.namespace().is_empty() {
        match value.local_name() {
            "doc" => Ok(pattern::OuterFunctionName::Doc),
            "id" => Ok(pattern::OuterFunctionName::Id),
            "element-with-id" => Ok(pattern::OuterFunctionName::ElementWithId),
            "key" => Ok(pattern::OuterFunctionName::Key),
            "root" => Ok(pattern::OuterFunctionName::Root),
            _ => Err(ParserError::IllegalFunctionInPattern {
                name: value.clone(),
                span: name.span,
            }),
        }
    } else {
        Err(ParserError::IllegalFunctionInPattern {
            name: value.clone(),
            span: name.span,
        })
    }
}

fn convert_argument(expr_single: &ast::ExprSingleS) -> Result<pattern::Argument, ParserError> {
    match &expr_single.value {
        ast::ExprSingle::Path(path_expr) => argument_from_path(path_expr, expr_single.span),
        _ => Err(unsupported_pattern(expr_single.span)),
    }
}

fn argument_from_path(
    path_expr: &ast::PathExpr,
    span: Span,
) -> Result<pattern::Argument, ParserError> {
    if path_expr.steps.len() != 1 {
        return Err(unsupported_pattern(span));
    }
    match &path_expr.steps[0].value {
        ast::StepExpr::PrimaryExpr(primary) => match &primary.value {
            ast::PrimaryExpr::VarRef(name) => Ok(pattern::Argument::VarRef(name.clone())),
            ast::PrimaryExpr::Literal(literal) => {
                Ok(pattern::Argument::Literal(literal.clone()))
            }
            _ => Err(unsupported_pattern(primary.span)),
        },
        _ => Err(unsupported_pattern(span)),
    }
}

fn implicit_root_info(
    steps: &[ast::StepExprS],
) -> Option<(pattern::PathRoot<ast::ExprS>, usize)> {
    let (first, rest) = steps.split_first()?;
    if !is_implicit_root_step(first) {
        return None;
    }
    if let Some((second, _rest)) = rest.split_first() {
        if is_implicit_descendant_step(second, first.span) {
            return Some((pattern::PathRoot::AbsoluteDoubleSlash, 2));
        }
    }
    Some((pattern::PathRoot::AbsoluteSlash, 1))
}

fn is_implicit_root_step(step: &ast::StepExprS) -> bool {
    match &step.value {
        ast::StepExpr::PrimaryExpr(primary) => match &primary.value {
            ast::PrimaryExpr::FunctionCall(call) => {
                if !span_is_empty(call.name.span) {
                    return false;
                }
                let name = &call.name.value;
                if name.namespace() != FN_NAMESPACE || name.local_name() != "root" {
                    return false;
                }
                call.arguments.len() == 1 && argument_is_self_node(&call.arguments[0])
            }
            _ => false,
        },
        _ => false,
    }
}

fn argument_is_self_node(expr_single: &ast::ExprSingleS) -> bool {
    match &expr_single.value {
        ast::ExprSingle::Path(path_expr) => {
            if path_expr.steps.len() != 1 {
                return false;
            }
            match &path_expr.steps[0].value {
                ast::StepExpr::AxisStep(axis_step) => {
                    axis_step.axis == ast::Axis::Self_
                        && matches!(axis_step.node_test, ast::NodeTest::KindTest(ast::KindTest::Any))
                        && axis_step.predicates.is_empty()
                }
                _ => false,
            }
        }
        _ => false,
    }
}

fn is_implicit_descendant_step(step: &ast::StepExprS, root_span: Span) -> bool {
    if !is_descendant_or_self_any(step) {
        return false;
    }
    if span_is_empty(step.span) {
        return true;
    }
    let step_len = span_len(step.span);
    let root_len = span_len(root_span);
    step_len == root_len && step_len <= 2
}

fn is_descendant_or_self_any(step: &ast::StepExprS) -> bool {
    match &step.value {
        ast::StepExpr::AxisStep(axis_step) => {
            axis_step.axis == ast::Axis::DescendantOrSelf
                && matches!(axis_step.node_test, ast::NodeTest::KindTest(ast::KindTest::Any))
                && axis_step.predicates.is_empty()
        }
        _ => false,
    }
}

impl pattern::Pattern<ast::ExprS> {
    pub fn parse<'a>(
        input: &'a str,
        namespaces: &'a Namespaces,
        variable_names: &'a VariableNames,
    ) -> Result<Self, ParserError> {
        let ast::XPath(expr) = ast::XPath::parse(input, namespaces, variable_names)?;
        expr_to_pattern(&expr.value, expr.span)
    }
}

#[cfg(test)]
mod tests {
    use chumsky::prelude::*;
    use insta::assert_ron_snapshot;
    use std::borrow::Cow;
    use xee_xpath_lexer::Token;

    use super::super::axis_node_test::parser_axis_node_test;
    use super::super::kind_test::parser_kind_test;
    use super::super::name::parser_name;
    use super::super::primary::parser_primary;
    use super::super::{parse, tokens};

    use super::*;

    #[test]
    fn test_predicate_pattern_no_predicates() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(".", &namespaces, &variable_names));
    }

    #[test]
    fn test_predicate_pattern_single_predicate() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            ".[1]",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_predicate_pattern_dot_equals() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            ".[.='10']",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_text_predicate_pattern() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "text()[.='10']",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_text_pattern() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "text()",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_text_predicate_numeric_pattern() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "text()[1]",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_axis_node_test_parse() {
        let namespaces = Namespaces::default();
        let parser_name_output = parser_name();
        let name = parser_name_output.eqname;
        let ncname = parser_name_output.ncname;
        let parser_primary_output = parser_primary(name.clone());
        let string = parser_primary_output.string;
        let empty_call = just(Token::LeftParen)
            .ignore_then(just(Token::RightParen))
            .boxed();
        let kind_test = parser_kind_test(name.clone(), empty_call, ncname, string).kind_test;
        let parser_axis_node_test_output = parser_axis_node_test(name, kind_test);
        let axis_node_test = parser_axis_node_test_output
            .axis_node_test
            .then_ignore(end())
            .boxed();
        assert_ron_snapshot!(parse(
            axis_node_test,
            tokens("text()"),
            Cow::Borrowed(&namespaces),
        ));
    }

    #[test]
    fn test_predicate_expr_parse() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(ast::XPath::parse(
            ".='10'",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_expr_pattern() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "$a | $b",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_expr_pattern_rooted_path() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "$a/foo",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_expr_pattern_absolute_slash() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "/foo",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_expr_pattern_absolute_double_slash() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "//foo",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_absolute_slash_without_steps() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse("/", &namespaces, &variable_names));
    }

    #[test]
    fn test_absolute_slash_without_steps_in_parenthesis() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse("(/)", &namespaces, &variable_names));
    }

    #[test]
    fn test_expr_pattern_relative() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse("foo", &namespaces, &variable_names));
    }

    #[test]
    fn test_postfix_expr() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo[1]",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_nested_predicate_parses() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert!(pattern::Pattern::parse(
            "foo[(bar[2])='this']",
            &namespaces,
            &variable_names
        )
        .is_ok());
        assert!(pattern::Pattern::parse(
            "foo[(bar[2][(baz[2])='goodbye'])]",
            &namespaces,
            &variable_names
        )
        .is_ok());
    }

    #[test]
    fn test_union() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo | bar",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_intersect() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo intersect bar",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_union_with_intersect() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo intersect bar | baz",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_union_with_union() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo | (bar | baz)",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_intersect_with_union() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        assert_ron_snapshot!(pattern::Pattern::parse(
            "foo intersect (bar | baz)",
            &namespaces,
            &variable_names
        ));
    }

    #[test]
    fn test_root_intersect_with_other_path() {
        let namespaces = Namespaces::default();
        let variable_names = VariableNames::default();
        // have to use bracketrs here, as otherwise 'intersect' is interpreted
        // as an element name as per xpath rules
        assert_ron_snapshot!(pattern::Pattern::parse(
            "(/) intersect foo",
            &namespaces,
            &variable_names
        ));
    }
}
