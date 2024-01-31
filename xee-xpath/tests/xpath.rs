use insta::assert_debug_snapshot;
use xee_xpath_ast::{ast, Namespaces};
use xot::Xot;

use xee_interpreter::{
    atomic::Atomic, context::DynamicContext, context::StaticContext, context::Variables,
    error::SpannedResult, sequence::Item, sequence::Sequence, xml::Document, xml::Documents,
    xml::Node, xml::Uri,
};
use xee_xpath::{evaluate, evaluate_without_focus, evaluate_without_focus_with_variables, parse};

fn xot_nodes_to_items(node: &[xot::Node]) -> Sequence {
    Sequence::from(
        node.iter()
            .map(|&node| Item::from(Node::Xot(node)))
            .collect::<Vec<_>>(),
    )
}

fn run(s: &str) -> SpannedResult<Sequence> {
    evaluate_without_focus(s)
}

fn run_with_variables(s: &str, variables: Variables) -> SpannedResult<Sequence> {
    evaluate_without_focus_with_variables(s, variables)
}

// fn run_debug(s: &str) -> SpannedResult<stack::Value> {
//     let xot = Xot::new();
//     let namespaces = Namespaces::default();
//     let static_context = StaticContext::new(&namespaces);
//     let context = DynamicContext::new(&xot, &static_context);
//     let xpath = XPath::new(context.static_context, s)?;
//     dbg!(&xpath.program.get_function(0).decoded());
//     xpath.run_value(&context, None)
// }

fn run_xml(xml: &str, xpath: &str) -> SpannedResult<Sequence> {
    evaluate(xml, xpath, None)
}

fn run_xml_default_ns(xml: &str, xpath: &str, ns: &str) -> SpannedResult<Sequence> {
    evaluate(xml, xpath, Some(ns))
}

fn assert_nodes<S>(xml: &str, xpath: &str, get_nodes: S) -> SpannedResult<()>
where
    S: Fn(&Xot, &Document) -> Vec<xot::Node>,
{
    let mut xot = Xot::new();
    let uri = Uri::new("http://example.com");
    let mut documents = Documents::new();
    documents.add(&mut xot, &uri, xml).unwrap();
    let namespaces = Namespaces::new(Namespaces::default_namespaces(), None, None);
    let static_context = StaticContext::from_namespaces(namespaces);
    let context = DynamicContext::from_documents(xot, &static_context, &documents);
    let document = documents.get(&uri).unwrap();
    let nodes = get_nodes(context.xot(), document);

    let xpath = parse(context.static_context, xpath)?;
    let result = xpath.runnable(&context).many_xot_node(document.root)?;
    assert_eq!(result, xot_nodes_to_items(&nodes));
    Ok(())
}

#[test]
fn test_compile_add() {
    assert_debug_snapshot!(run("1 + 2"));
}

#[test]
fn test_nested() {
    assert_debug_snapshot!(run("1 + (8 - 2)"));
}

#[test]
fn test_arith_empty_sequence_left() {
    assert_debug_snapshot!(run("() + 1"));
}

#[test]
fn test_arith_empty_sequence_right() {
    assert_debug_snapshot!(run("1 + ()"));
}

#[test]
fn test_comma() {
    assert_debug_snapshot!(run("1, 2"));
}

#[test]
fn test_empty_sequence() {
    assert_debug_snapshot!(run("()"));
}

#[test]
fn test_comma_squences() {
    assert_debug_snapshot!(run("(1, 2), (3, 4)"));
}

#[test]
fn test_let() {
    assert_debug_snapshot!(run("let $x := 1 return $x + 2"));
}

#[test]
fn test_let_nested() {
    assert_debug_snapshot!(run("let $x := 1, $y := $x + 3 return $y + 5"));
}

#[test]
fn test_let_on_right_side() {
    assert_debug_snapshot!(run("1 + (let $x := 2 return $x + 10)"));
}

#[test]
fn test_if() {
    assert_debug_snapshot!(run("if (1) then 2 else 3"));
}

#[test]
fn test_if_false() {
    assert_debug_snapshot!(run("if (0) then 2 else 3"));
}

#[test]
fn test_if_with_let_true() {
    assert_debug_snapshot!(run(
        "if (1) then (let $x := 2 return $x) else (let $x := 3 return $x)"
    ));
}

#[test]
fn test_if_with_let_false() {
    assert_debug_snapshot!(run(
        "if (0) then (let $x := 2 return $x) else (let $x := 3 return $x)"
    ));
}

#[test]
fn test_value_eq_true() {
    assert_debug_snapshot!(run("1 eq 1"));
}

#[test]
fn test_value_eq_false() {
    assert_debug_snapshot!(run("1 eq 2"));
}

#[test]
fn test_value_ne_true() {
    assert_debug_snapshot!(run("1 ne 2"));
}

#[test]
fn test_value_ne_false() {
    assert_debug_snapshot!(run("1 ne 1"));
}

#[test]
fn test_value_lt_true() {
    assert_debug_snapshot!(run("1 lt 2"));
}

#[test]
fn test_value_lt_false() {
    assert_debug_snapshot!(run("2 lt 1"));
}

#[test]
fn test_inline_function_without_args() {
    assert_debug_snapshot!(run("function() { 5 } ()"));
}

#[test]
fn test_inline_function_with_single_arg() {
    assert_debug_snapshot!(run("function($x) { $x + 5 } (3)"));
}

#[test]
fn test_inline_function_with_multiple_args() {
    assert_debug_snapshot!(run("function($x, $y) { $x + $y } (3, 5)"));
}

#[test]
fn test_function_nested() {
    assert_debug_snapshot!(run("function($x) { function($y) { $y + 2 }($x + 1) } (5)"));
}

#[test]
fn test_function_closure() {
    assert_debug_snapshot!(run(
        "function() { let $x := 3 return function() { $x + 2 } }()()"
    ));
}

#[test]
fn test_function_closure_with_multiple_variables() {
    assert_debug_snapshot!(run(
        "function() { let $x := 3, $y := 1 return function() { $x - $y } }()()"
    ));
}

#[test]
fn test_function_closure_with_multiple_variables_arguments() {
    assert_debug_snapshot!(run(
        "function() { let $x := 3 return function($y) { $x - $y } }()(1)"
    ));
}

#[test]
fn test_function_closure_nested() {
    assert_debug_snapshot!(run(
            "function() { let $x := 3 return function() { let $y := 4 return function() { $x + $y }} }()()()"
        ));
}

#[test]
fn test_static_function_call() {
    assert_debug_snapshot!(run("my_function(5, 2)"));
}

#[test]
fn test_named_function_ref_call() {
    assert_debug_snapshot!(run("my_function#2(5, 2)"));
}

#[test]
fn test_static_call_with_placeholders() {
    assert_debug_snapshot!(run("my_function(?, 2)(5)"));
}

#[test]
fn test_inline_function_with_args_placeholdered() {
    assert_debug_snapshot!(run("function($x, $y) { $x - $y } ( ?, 3 ) (5)"));
}

#[test]
fn test_inline_function_with_args_placeholdered2() {
    assert_debug_snapshot!(run("function($x, $y) { $x - $y } ( ?, 3 ) (?) (5)"));
}

#[test]
fn test_inline_function_call_with_let() {
    assert_debug_snapshot!(run(
        "function($x, $y) { $x + $y }(let $a := 1 return $a, let $b := 2 return $b)"
    ));
}

#[test]
fn test_inline_function_call_with_let2() {
    assert_debug_snapshot!(run(
        "let $a := 1 return function($x, $y) { $x + $y }($a, let $b := 2 return $b)"
    ));
}

#[test]
fn test_range() {
    assert_debug_snapshot!(run("1 to 5"));
}

#[test]
fn test_range_greater() {
    assert_debug_snapshot!(run("5 to 1"));
}

#[test]
fn test_range_equal() {
    assert_debug_snapshot!(run("1 to 1"));
}

#[test]
fn test_for_loop() {
    assert_debug_snapshot!(run("for $x in 1 to 5 return $x + 2"));
}

#[test]
fn test_nested_for_loop() {
    assert_debug_snapshot!(run("for $i in (10, 20, 30), $j in (1, 2) return $i + $j"));
}

#[test]
fn test_nested_for_loop_variable_scope() {
    assert_debug_snapshot!(run(
        "for $i in (10, 20), $j in ($i + 1, $i + 2) return $i + $j"
    ));
}

#[test]
fn test_simple_map() {
    assert_debug_snapshot!(run("(1, 2) ! (. + 1)"));
}

#[test]
fn test_simple_map_sequence() {
    assert_debug_snapshot!(run("(1, 2) ! (., 0)"));
}

#[test]
fn test_simple_map_single() {
    assert_debug_snapshot!(run("1 ! (. , 0)"));
}

#[test]
fn test_simple_map_multiple_steps() {
    assert_debug_snapshot!(run("(1, 2) ! (. + 1) ! (. + 2)"));
}

#[test]
fn test_simple_map_multiple_steps2() {
    assert_debug_snapshot!(run("(1, 2) ! (. + 1) ! (. + 2) ! (. + 3)"));
}

#[test]
fn test_simple_map_position() {
    assert_debug_snapshot!(run("(4, 5, 6) ! (fn:position())"));
}

#[test]
fn test_simple_map_last() {
    assert_debug_snapshot!(run("(4, 5, 6) ! (fn:last())"));
}

#[test]
fn test_some_quantifier_expr_true() {
    assert_debug_snapshot!(run("some $x in (1, 2, 3) satisfies $x eq 2"));
}

#[test]
fn test_some_quantifier_expr_false() {
    assert_debug_snapshot!(run("some $x in (1, 2, 3) satisfies $x eq 5"));
}

#[test]
fn test_nested_some_quantifier_expr_true() {
    assert_debug_snapshot!(run("some $x in (1, 2, 3), $y in (2, 3) satisfies $x gt $y"));
}

#[test]
fn test_every_quantifier_expr_true() {
    assert_debug_snapshot!(run("every $x in (1, 2, 3) satisfies $x lt 5"));
}

#[test]
fn test_every_quantifier_expr_false() {
    assert_debug_snapshot!(run("every $x in (1, 2, 3) satisfies $x gt 2"));
}

#[test]
fn test_every_quantifier_nested_true() {
    assert_debug_snapshot!(run(
        "every $x in (2, 3, 4), $y in (0, 1) satisfies $x gt $y"
    ));
}

#[test]
fn test_every_quantifier_nested_false() {
    assert_debug_snapshot!(run(
        "every $x in (2, 3, 4), $y in (1, 2) satisfies $x gt $y"
    ));
}

#[test]
fn test_some_quantifier_empty_sequence() {
    assert_debug_snapshot!(run("some $x in () satisfies $x eq 5"));
}

#[test]
fn test_every_quantifier_empty_sequence() {
    assert_debug_snapshot!(run("every $x in () satisfies $x eq 5"));
}

#[test]
fn test_predicate() {
    assert_debug_snapshot!(run("(1, 2, 3)[. ge 2]"));
}

#[test]
fn test_predicate_empty_sequence() {
    assert_debug_snapshot!(run("() [. ge 1]"));
}

#[test]
fn test_predicate_multiple() {
    assert_debug_snapshot!(run("(1, 2, 3)[. ge 2][. ge 3]"));
}

#[test]
fn test_comma_simple_map() {
    assert_debug_snapshot!(run("(1, 2), (3, 4) ! (. + 1)"));
}

#[test]
fn test_comma_simple_map2() {
    assert_debug_snapshot!(run("(1, 2), (3, 4), (5, 6) ! (. + 1)"));
}

#[test]
fn test_simple_map_empty_sequence() {
    assert_debug_snapshot!(run("() ! (. + 1)"));
}

#[test]
fn test_predicate_index() {
    assert_debug_snapshot!(run("(1, 2, 3)[2]"));
}

#[test]
fn test_predicate_index2() {
    assert_debug_snapshot!(run("(1, 2, 3)[2] + (4, 5)[1]"));
}

#[test]
fn test_predicate_index_all() {
    assert_debug_snapshot!(run("(1, 2, 3)[fn:position()]"));
}

#[test]
fn test_predicate_index_not_whole_number() {
    // since no position matches, we should get the empty sequence
    assert_debug_snapshot!(run("(1, 2, 3)[2.5]"));
}

#[test]
fn test_sequence_predicate() {
    // this should succeed, as IsNumeric sees the sequence as non-numeric.
    // We create the sequence with (2, 3)[. > 2] to ensure it's indeed a
    // sequence underneath, and not atomic. The sequence of a single value
    // is interpreted as boolean and thus we see the full sequence
    assert_debug_snapshot!(run("(1, 2, 3)[(2, 3)[. > 2]]"));
}

#[test]
fn test_sequence_predicate_sequence_too_long() {
    // this should fail: 2, 3 is not a numeric nor an effective boolean value
    assert_debug_snapshot!(run("(1, 2, 3)[(2, 3)]"));
}

#[test]
fn test_sequence_predicate_sequence_empty() {
    // the empty sequence is an effective boolean of false, so we should
    // get the result of the empty sequence
    assert_debug_snapshot!(run("(1, 2, 3)[()]"));
}

#[test]
fn test_child_axis_step1() -> SpannedResult<()> {
    assert_nodes(r#"<doc><a/><b/></doc>"#, "doc/*", |xot, document| {
        let doc_el = xot.document_element(document.root).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let b = xot.next_sibling(a).unwrap();
        vec![a, b]
    })
}

#[test]
fn test_child_axis_step2() -> SpannedResult<()> {
    assert_nodes(r#"<doc><a/><b/></doc>"#, "doc/a", |xot, document| {
        let doc_el = xot.document_element(document.root).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        vec![a]
    })
}

#[test]
fn test_step_with_predicate() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a/><b/></doc>"#,
        "doc/*[fn:position() eq 2]",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let b = xot.next_sibling(a).unwrap();
            vec![b]
        },
    )
}

#[test]
fn test_descendant_axis_step() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a/><b><c/></b></doc>"#,
        "descendant::*",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let b = xot.next_sibling(a).unwrap();
            let c = xot.first_child(b).unwrap();
            vec![doc_el, a, b, c]
        },
    )
}

#[test]
fn test_descendant_axis_position() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "descendant::* / fn:position()"
    ));
}

#[test]
fn test_descendant_axis_step2() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a><c/></a><b/></doc>"#,
        "descendant::*",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let b = xot.next_sibling(a).unwrap();
            let c = xot.first_child(a).unwrap();
            vec![doc_el, a, c, b]
        },
    )
}

#[test]
fn test_comma_nodes() -> SpannedResult<()> {
    assert_nodes(r#"<doc><a/><b/></doc>"#, "doc/b, doc/a", |xot, document| {
        let doc_el = xot.document_element(document.root).unwrap();
        let a = xot.first_child(doc_el).unwrap();
        let b = xot.next_sibling(a).unwrap();
        vec![b, a]
    })
}

#[test]
fn test_union() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a/><b/><c/></doc>"#,
        "doc/c | doc/a | doc/b | doc/a",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let b = xot.next_sibling(a).unwrap();
            let c = xot.next_sibling(b).unwrap();
            vec![a, b, c]
        },
    )
}

#[test]
fn test_default_position() {
    assert_debug_snapshot!(run_xml("<doc/>", "fn:position()"));
}

#[test]
fn test_default_position_no_context() {
    assert_debug_snapshot!(run("fn:position()"));
}

#[test]
fn test_default_last() {
    assert_debug_snapshot!(run_xml("<doc/>", "fn:last()"));
}

#[test]
fn test_default_last_no_context() {
    assert_debug_snapshot!(run("fn:last()"));
}

#[test]
fn test_position_closure() {
    assert_debug_snapshot!(run("(3, 4) ! (let $p := fn:position#0 return $p())"));
}

#[test]
fn test_simple_string() {
    assert_debug_snapshot!(run("'hello'"));
}

#[test]
fn test_simple_string_concat() {
    assert_debug_snapshot!(run("'hello' || 'world'"));
}

#[test]
fn test_string_compare_eq_true() {
    assert_debug_snapshot!(run("'hello' eq 'hello'"));
}

#[test]
fn test_string_compare_eq_false() {
    assert_debug_snapshot!(run("'hello' eq 'world'"));
}

#[test]
fn test_local_name_element() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "descendant::* / fn:local-name()"
    ));
}

#[test]
fn test_local_name_empty() {
    assert_debug_snapshot!(run("fn:local-name(())"));
}

#[test]
fn test_namespace_uri_element() {
    assert_debug_snapshot!(run_xml(
        r#"<doc xmlns="http://example.com/" xmlns:e="http://example.com/e"><a/><b><e:c/></b></doc>"#,
        "descendant::* / fn:namespace-uri()"
    ));
}

#[test]
fn test_count() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "fn:count(descendant::*)"
    ));
}

#[test]
fn test_fn_root() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "doc/a / fn:root() / doc / fn:local-name()"
    ));
}

#[test]
fn test_fn_root_explicit() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "fn:root(doc/a) / doc / b / fn:local-name()"
    ));
}

#[test]
fn test_fn_root_absent() {
    assert_debug_snapshot!(run("fn:root()"));
}

#[test]
fn test_fn_root_implicit() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "/doc/a / fn:local-name()"
    ));
}

#[test]
fn test_fn_double_slash_root_implicit() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "//a / fn:local-name()"
    ));
}

#[test]
fn test_fn_namespace_default() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/><b><c/></b></doc>"#,
        "descendant::* / local-name()"
    ));
}

#[test]
fn test_element_namespace_wrong() {
    // we expect no match, as doc is in a namespace and the default is None
    assert_debug_snapshot!(run_xml(
        r#"<doc xmlns="http://example.com"><a/></doc>"#,
        "doc / local-name()",
    ));
}

#[test]
fn test_element_namespace_default() {
    // here we set the default element namespace for xpath expressions
    assert_debug_snapshot!(run_xml_default_ns(
        r#"<doc xmlns="http://example.com"><a/></doc>"#,
        "doc / local-name()",
        "http://example.com"
    ));
}

#[test]
fn test_attribute_namespace_no_default() {
    // here we set the default element namespace for xpath expressions
    assert_debug_snapshot!(run_xml_default_ns(
        r#"<doc xmlns="http://example.com" a="hello"/>"#,
        "doc / @a / local-name()",
        "http://example.com"
    ));
}

#[test]
fn test_string_document_node() {
    assert_debug_snapshot!(run_xml(r#"<doc><a>A</a><b>B</b></doc>"#, "string(doc)"));
}

#[test]
fn test_string_element_node() {
    assert_debug_snapshot!(run_xml(r#"<doc><a>A</a><b>B</b></doc>"#, "string(doc/a)"));
}

#[test]
fn test_string_integer() {
    assert_debug_snapshot!(run("fn:string(1)"));
}

#[test]
fn test_atomize() {
    assert_debug_snapshot!(run("(1) eq (1)"));
}

#[test]
fn test_atomize_xml_eq_true() {
    assert_debug_snapshot!(run_xml(r#"<doc><a>A</a><b>A</b></doc>"#, "doc/a eq doc/b",));
}

#[test]
fn test_atomize_xml_eq_false() {
    assert_debug_snapshot!(run_xml(r#"<doc><a>A</a><b>B</b></doc>"#, "doc/a eq doc/b",));
}

#[test]
fn test_atomize_xml_attribute_eq_true() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a f="FOO"/><b f="FOO"/></doc>"#,
        "doc/a/@f eq doc/b/@f",
    ));
}

#[test]
fn test_atomize_xml_attribute_eq_false() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a f="FOO"/><b f="BAR"/></doc>"#,
        "doc/a/@f eq doc/b/@f",
    ));
}

#[test]
fn test_atomize_xml_attribute_present() {
    assert_debug_snapshot!(run_xml(r#"<doc><a f="FOO"/></doc>"#, "doc/a/@f eq 'FOO'",));
}

#[test]
fn test_atomize_xml_attribute_missing() {
    assert_debug_snapshot!(run_xml(r#"<doc><a/></doc>"#, "doc/a/@f eq 'FOO'",));
}

#[test]
fn test_attribute_predicate() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a/><b foo="FOO"/><c/></doc>"#,
        "//*[@foo eq 'FOO']",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let b = xot.next_sibling(a).unwrap();
            vec![b]
        },
    )
}

#[test]
fn test_external_variable() {
    assert_debug_snapshot!(run_with_variables(
        "$foo",
        Variables::from([(
            ast::Name::unprefixed("foo"),
            Item::from(Atomic::from("FOO")).into()
        )]),
    ))
}

#[test]
fn test_external_variables() {
    assert_debug_snapshot!(run_with_variables(
        "$foo + $bar",
        Variables::from([
            (
                ast::Name::unprefixed("foo"),
                Item::from(Atomic::from(1i64)).into()
            ),
            (
                ast::Name::unprefixed("bar"),
                Item::from(Atomic::from(2i64)).into()
            )
        ])
    ))
}

#[test]
fn test_absent_context() {
    assert_debug_snapshot!(run("."));
}

// This results in a type error, because the context is absent and no
// operations with absent are permitted. This is not ideal - better would
// be if the access to . already resulted in a XPDY0002 error. But
// . is compiled away and no function call takes place (unlike for fn:position
// fn:last), so we don't get an error at that level.
#[test]
fn test_absent_context_with_operation() {
    assert_debug_snapshot!(run(". + 1"));
}

// Same problem as before, type error instead of XPDY0002 error
#[test]
fn test_default_position_with_operation() {
    assert_debug_snapshot!(run("fn:position() + 1"));
}

#[test]
fn test_string_compare_general_eq_true() {
    assert_debug_snapshot!(run("'hello' = 'hello'"));
}

#[test]
fn test_compare_general_eq_sequence_true() {
    assert_debug_snapshot!(run("(1, 2) = (3, 2)"));
}

#[test]
fn test_compare_general_eq_sequence_false() {
    assert_debug_snapshot!(run("(1, 2) = (3, 4)"));
}

#[test]
fn test_generate_id() {
    assert_debug_snapshot!(run_xml(r#"<doc><a/><b/><c/></doc>"#, "generate-id(doc/a)",));
}

#[test]
fn test_fn_string() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><p>Hello world!</p></doc>"#,
        "/doc/p/string()",
    ));
}

#[test]
fn test_let_uses_own_variable() {
    assert_debug_snapshot!(run("let $x := $x return $x"));
}

#[test]
fn test_static_function_call_nested() {
    assert_debug_snapshot!(run(r#"fn:string-join(("A"),xs:string("A"))"#));
}

#[test]
fn test_run_unary_minus() {
    assert_debug_snapshot!(run("-1"));
}

#[test]
fn test_cast_integer_as_string() {
    assert_debug_snapshot!(run("1 cast as xs:string"));
}

#[test]
fn test_cast_empty_sequence_as_string() {
    assert_debug_snapshot!(run("() cast as xs:string"));
}

#[test]
fn test_cast_empty_sequence_as_string_question_mark() {
    assert_debug_snapshot!(run("() cast as xs:string?"));
}

#[test]
fn test_case_as_any_uri() {
    assert_debug_snapshot!(run("'http://example.com' cast as xs:anyURI"));
}

#[test]
fn test_cast_as_normalized_string() {
    assert_debug_snapshot!(run("'foo\nbar' cast as xs:normalizedString"));
}

#[test]
fn test_cast_as_token() {
    assert_debug_snapshot!(run("'  foo\n\nbar ' cast as xs:token"));
}

#[test]
fn test_cast_as_language() {
    assert_debug_snapshot!(run("'en' cast as xs:language"));
}

#[test]
fn test_cast_as_language_fails() {
    assert_debug_snapshot!(run("'en us' cast as xs:language"));
}

#[test]
fn test_cast_as_nmtoken() {
    assert_debug_snapshot!(run("'foobar' cast as xs:NMTOKEN"));
}

#[test]
fn test_cast_as_nmtoken_fails() {
    assert_debug_snapshot!(run("'foo bar' cast as xs:NMTOKEN"));
}

#[test]
fn test_cast_as_name() {
    assert_debug_snapshot!(run("'foobar' cast as xs:Name"));
}

#[test]
fn test_cast_as_name_with_colon() {
    assert_debug_snapshot!(run("'foo:bar' cast as xs:Name"));
}

#[test]
fn test_cast_as_name_fails() {
    assert_debug_snapshot!(run("'foo bar' cast as xs:Name"));
}

#[test]
fn test_cast_as_ncname() {
    assert_debug_snapshot!(run("'foobar' cast as xs:NCName"));
}

#[test]
fn test_cast_as_ncname_with_colon_fails() {
    assert_debug_snapshot!(run("'foo:bar' cast as xs:NCName"));
}

#[test]
fn test_cast_as_qname() {
    assert_debug_snapshot!(run("'xs:bar' cast as xs:QName"));
}

#[test]
fn test_cast_as_qname_fails_unknown_prefix() {
    assert_debug_snapshot!(run("'foo:bar' cast as xs:QName"));
}

#[test]
fn test_cast_as_qname_fails_multiple_prefixes() {
    assert_debug_snapshot!(run("'xs:bar:baz' cast as xs:QName"));
}

#[test]
fn test_cast_as_qname_fails_illegal_value() {
    assert_debug_snapshot!(run("'bar baz' cast as xs:QName"));
}

#[test]
fn test_xs_qname() {
    assert_debug_snapshot!(run("xs:QName('xs:bar')"));
}

#[test]
fn test_cast_as_hex_binary() {
    assert_debug_snapshot!(run("'ff' cast as xs:hexBinary"));
}

#[test]
fn test_case_as_hex_binary_fails() {
    assert_debug_snapshot!(run("'f' cast as xs:hexBinary"));
}

#[test]
fn test_cast_as_base64_binary() {
    // "hello" encoded
    assert_debug_snapshot!(run("'aGVsbG8=' cast as xs:base64Binary"));
}

#[test]
fn test_cast_as_base64_binary_fails() {
    assert_debug_snapshot!(run("'flurb' cast as xs:base64Binary"));
}

#[test]
fn test_cast_as_year_month_duration() {
    assert_debug_snapshot!(run("'P1Y2M' cast as xs:yearMonthDuration"));
}

#[test]
fn test_cast_as_year_month_duration_back_to_string() {
    assert_debug_snapshot!(run(
        "('P14M' cast as xs:yearMonthDuration) cast as xs:string"
    ));
}

#[test]
fn test_cast_as_day_time_duration_back_to_string() {
    assert_debug_snapshot!(run(
        "('P1DT2H3M4S' cast as xs:dayTimeDuration) cast as xs:string"
    ));
}

#[test]
fn test_cast_as_day_time_duration_back_to_string2() {
    // should come out at P1DT1h, as 90000 seconds is 25 hours
    assert_debug_snapshot!(run(
        "('PT90000S' cast as xs:dayTimeDuration) cast as xs:string"
    ));
}

#[test]
fn test_cast_duration_back_to_string() {
    assert_debug_snapshot!(run(
        "('P1Y2M3DT4H5M6S' cast as xs:duration) cast as xs:string"
    ));
}

#[test]
fn test_cast_duration_back_to_string2() {
    assert_debug_snapshot!(run("('P20M' cast as xs:duration) cast as xs:string"));
}

#[test]
fn test_cast_date_time() {
    assert_debug_snapshot!(run("'2019-01-01T00:00:00' cast as xs:dateTime"));
}

#[test]
fn test_cast_date_time_z() {
    assert_debug_snapshot!(run("'2019-01-01T00:00:00Z' cast as xs:dateTime"));
}

#[test]
fn test_cast_date_time_offset() {
    assert_debug_snapshot!(run("'2019-01-01T00:00:00+01:00' cast as xs:dateTime"));
}

#[test]
fn test_cast_date_time_back_to_string_naive() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30' cast as xs:dateTime) cast as xs:string"
    ));
}

#[test]
fn test_cast_date_time_back_to_string_z() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30Z' cast as xs:dateTime) cast as xs:string"
    ));
}

#[test]
fn test_cast_date_time_back_to_string_offset() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30+01:00' cast as xs:dateTime) cast as xs:string"
    ));
}

#[test]
fn test_cast_date_time_millis_back_to_string() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30.125' cast as xs:dateTime) cast as xs:string"
    ));
}

#[test]
fn test_cast_date_time_stamp() {
    assert_debug_snapshot!(run(
        "'2019-01-01T00:00:00.123+01:00' cast as xs:dateTimeStamp"
    ));
}

#[test]
fn test_cast_date_time_stamp_no_millis_back_to_string() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30+01:00' cast as xs:dateTimeStamp) cast as xs:string"
    ));
}

#[test]
fn test_cast_date_time_stamp_millis_back_to_string() {
    assert_debug_snapshot!(run(
        "('2019-01-03T15:14:30.123+01:00' cast as xs:dateTimeStamp) cast as xs:string"
    ));
}

#[test]
fn test_cast_time() {
    assert_debug_snapshot!(run("'00:00:00' cast as xs:time"));
}

#[test]
fn test_cast_time_z() {
    assert_debug_snapshot!(run("'00:00:00Z' cast as xs:time"));
}

#[test]
fn test_cast_time_naive_back_to_string() {
    assert_debug_snapshot!(run("('15:14:30' cast as xs:time) cast as xs:string"));
}

#[test]
fn test_cast_time_millis_back_to_string() {
    assert_debug_snapshot!(run("('15:14:30.123' cast as xs:time) cast as xs:string"));
}

#[test]
fn test_cast_time_z_back_to_string() {
    assert_debug_snapshot!(run("('15:14:30Z' cast as xs:time) cast as xs:string"));
}

#[test]
fn test_cast_date() {
    assert_debug_snapshot!(run("'2019-01-01' cast as xs:date"));
}

#[test]
fn test_cast_date_naive_back_to_string() {
    assert_debug_snapshot!(run("('2019-01-03' cast as xs:date) cast as xs:string"));
}

#[test]
fn test_cast_date_z_back_to_string() {
    assert_debug_snapshot!(run("('2019-01-03Z' cast as xs:date) cast as xs:string"));
}

#[test]
fn test_cast_date_z() {
    assert_debug_snapshot!(run("'2019-01-01Z' cast as xs:date"));
}

#[test]
fn test_cast_g_year_month() {
    assert_debug_snapshot!(run("'2019-01' cast as xs:gYearMonth"));
}

#[test]
fn test_cast_g_year_month_back_to_string() {
    assert_debug_snapshot!(run("('2019-01' cast as xs:gYearMonth) cast as xs:string"));
}

#[test]
fn test_cast_g_year_month_tz() {
    assert_debug_snapshot!(run("'2019-01Z' cast as xs:gYearMonth"));
}

#[test]
fn test_cast_g_year() {
    assert_debug_snapshot!(run("'2019' cast as xs:gYear"));
}

#[test]
fn test_cast_g_year_back_to_string() {
    assert_debug_snapshot!(run("('2019' cast as xs:gYear) cast as xs:string"));
}

#[test]
fn test_cast_g_year_tz() {
    assert_debug_snapshot!(run("'2019Z' cast as xs:gYear"));
}

#[test]
fn test_cast_g_year_longer() {
    assert_debug_snapshot!(run("'20190' cast as xs:gYear"));
}

#[test]
fn test_cast_g_year_longer_back_to_string() {
    assert_debug_snapshot!(run("('20190' cast as xs:gYear) cast as xs:string"));
}

#[test]
fn test_cast_g_month_day() {
    assert_debug_snapshot!(run("'--01-01' cast as xs:gMonthDay"));
}

#[test]
fn test_cast_g_month_day_back_to_string() {
    assert_debug_snapshot!(run("('--06-21' cast as xs:gMonthDay) cast as xs:string"));
}

#[test]
fn test_cast_g_month_day_tz() {
    assert_debug_snapshot!(run("'--01-01Z' cast as xs:gMonthDay"));
}

#[test]
fn test_cast_g_day() {
    assert_debug_snapshot!(run("'---01' cast as xs:gDay"));
}

#[test]
fn test_cast_g_day_back_to_string() {
    assert_debug_snapshot!(run("('---01' cast as xs:gDay) cast as xs:string"));
}

#[test]
fn test_cast_g_day_tz() {
    assert_debug_snapshot!(run("'---01Z' cast as xs:gDay"));
}

#[test]
fn test_cast_g_month() {
    assert_debug_snapshot!(run("'--01' cast as xs:gMonth"));
}

#[test]
fn test_cast_g_month_back_to_string() {
    assert_debug_snapshot!(run("('--01' cast as xs:gMonth) cast as xs:string"));
}

#[test]
fn test_cast_g_month_tz() {
    assert_debug_snapshot!(run("'--01Z' cast as xs:gMonth"));
}

#[test]
fn test_castable_as_integer_success() {
    assert_debug_snapshot!(run("1 castable as xs:integer"));
}

#[test]
fn test_castable_as_integer_failure() {
    assert_debug_snapshot!(run("'A' castable as xs:integer"));
}

#[test]
fn test_castable_as_integer_empty_sequence_fails() {
    assert_debug_snapshot!(run("() castable as xs:integer"));
}

#[test]
fn test_castable_as_integer_empty_sequence_question_mark() {
    assert_debug_snapshot!(run("() castable as xs:integer?"));
}

#[test]
fn test_instance_of_one() {
    assert_debug_snapshot!(run("1 instance of xs:integer"));
}

#[test]
fn test_instance_of_one_fails() {
    assert_debug_snapshot!(run("() instance of xs:integer"));
}

#[test]
fn test_instance_of_many() {
    assert_debug_snapshot!(run("(1, 2) instance of xs:integer*"));
}

#[test]
fn test_instance_of_node() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/></doc>"#,
        "doc/a instance of element(a)",
    ));
}

#[test]
fn test_instance_of_node_fails() {
    assert_debug_snapshot!(run_xml(
        r#"<doc><a/></doc>"#,
        "doc/a instance of element(b)",
    ));
}

#[test]
fn test_kind_test_in_path() -> SpannedResult<()> {
    assert_nodes(
        r#"<doc><a/>foo<b/></doc>"#,
        "doc/element()",
        |xot, document| {
            let doc_el = xot.document_element(document.root).unwrap();
            let a = xot.first_child(doc_el).unwrap();
            let text = xot.next_sibling(a).unwrap();
            let b = xot.next_sibling(text).unwrap();
            vec![a, b]
        },
    )
}

#[test]
fn test_negative_result_when_adding_day_time_durations() {
    assert_debug_snapshot!(run(
        "(xs:date('0001-01-01Z') + xs:dayTimeDuration('-P11DT02H02M')) cast as xs:string"
    ));
}

#[test]
fn test_negative_result_when_subtracting_day_time_durations() {
    assert_debug_snapshot!(run(
        "(xs:date('0001-01-01Z') - xs:dayTimeDuration('P11DT02H02M')) cast as xs:string"
    ));
}

#[test]
fn test_compare_same() {
    assert_debug_snapshot!(run("compare('str', 'str') instance of xs:integer"));
}

#[test]
fn test_compare_complex_collation_argument() {
    assert_debug_snapshot!(run(
        "compare('a', 'b', ((), 'http://www.w3.org/2005/xpath-functions/collation/codepoint', ()))"
    ));
}

#[test]
fn test_xs_double_nan() {
    assert_debug_snapshot!(run("xs:double('NaN')"));
}

#[test]
fn test_xs_double_nan_ne_to_itself() {
    assert_debug_snapshot!(run("xs:double('NaN') ne xs:double('NaN')"));
}

#[test]
fn test_round_1_1() {
    assert_debug_snapshot!(run("round(1.1)"));
}

#[test]
fn test_negative_round_integer() {
    assert_debug_snapshot!(run("round(123, -2)"));
}

#[test]
fn test_negative_round_integer2() {
    assert_debug_snapshot!(run("round(151, -2)"));
}

#[test]
fn test_negative_round_integer3() {
    assert_debug_snapshot!(run("round(-123, -2)"));
}

#[test]
fn test_negative_round_integer4() {
    assert_debug_snapshot!(run("round(-151, -2)"));
}

#[test]
fn test_deep_equal_equal_to_itself() {
    assert_debug_snapshot!(run_xml(r#"<doc><a/></doc>"#, "deep-equal(/, /)",));
}

#[test]
fn test_function_parameters() {
    assert_debug_snapshot!(run(
        "let $apply := function($x as xs:integer, $f as function(xs:integer) as xs:integer) as xs:integer {
            $f($x)
         } return $apply(3, function($x) { $x + 1 })"
    ))
}

#[test]
fn test_qname_without_prefix() {
    assert_debug_snapshot!(run("QName('http://example.com', 'foo')"));
}

#[test]
fn test_run_focus_independent_function_on_focus() {
    assert_debug_snapshot!(run_xml(r#"<doc><a/></doc>"#, "doc/a/default-collation()"));
}

#[test]
fn test_run_function_lookup_on_focus() {
    assert_debug_snapshot!(run_xml(
        r#"<root/>"#,
        "/root/function-lookup(fn:QName('http://www.w3.org/2005/xpath-functions', 'node-name'), 0)()"
    ));
}

#[test]
fn test_curly_array() {
    assert_debug_snapshot!(run("array {'a', 2, 3}(1)"));
}

#[test]
fn test_square_array() {
    assert_debug_snapshot!(run("['a', 2, 3](1)"));
}

#[test]
fn test_square_array_sequence() {
    assert_debug_snapshot!(run("[('a', 'b'), 2, 3](1)"));
}

#[test]
fn test_curly_map() {
    assert_debug_snapshot!(run("map {'a' : 'b'}('a')"));
}

#[test]
fn test_cast_negative_zero() {
    assert_debug_snapshot!(run("xs:unsignedLong('-0')"));
}
