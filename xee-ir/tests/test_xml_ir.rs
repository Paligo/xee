use ahash::HashMapExt;
use insta::assert_debug_snapshot;

use xee_interpreter::interpreter::{instruction::decode_instructions, Program};
use xee_ir::{ir, FunctionBuilder, FunctionCompiler, ModeIds, Scopes};
use xee_xpath_ast::span::Spanned;

fn spanned<T>(t: T) -> Spanned<T> {
    Spanned::new(t, (0..0).into())
}

#[test]
fn test_generate_element() {
    // we create an element name from consts
    let local_name = ir::Atom::Const(ir::Const::String("foo".to_string()));
    let namespace = ir::Atom::Const(ir::Const::String("".to_string()));
    let name = ir::XmlName {
        local_name: spanned(local_name),
        namespace: spanned(namespace),
    };

    let root_name = ir::Name::new("root".to_string());
    let element_name = ir::Name::new("element".to_string());

    // create a root element
    let root_expr = ir::Expr::XmlDocument(ir::XmlRoot {});

    // create an element of element_name
    let element_expr = ir::Expr::XmlElement(ir::XmlElement {
        name: spanned(ir::Atom::Variable(element_name.clone())),
    });

    // we need to make sure the element name exists within scope of element_expr
    let let_element = ir::Expr::Let(ir::Let {
        name: element_name.clone(),
        var_expr: Box::new(spanned(ir::Expr::XmlName(name))),
        return_expr: Box::new(spanned(element_expr)),
    });

    // we can now appedn the element to the root
    let append = ir::Expr::XmlAppend(ir::XmlAppend {
        parent: spanned(ir::Atom::Variable(root_name.clone())),
        child: spanned(ir::Atom::Variable(element_name.clone())),
    });

    // we need to make sure element_name exists within the scope of append
    let element_name_append = ir::Expr::Let(ir::Let {
        name: element_name.clone(),
        var_expr: Box::new(spanned(let_element)),
        return_expr: Box::new(spanned(append)),
    });

    // we need to make sure root name exists within the scope of append too
    let let_root = ir::Expr::Let(ir::Let {
        name: root_name,
        var_expr: Box::new(spanned(root_expr)),
        return_expr: Box::new(spanned(element_name_append)),
    });

    // wrap all of this into a function definition
    let function_definition = ir::FunctionDefinition {
        params: vec![
            ir::Param {
                name: ir::Name::new("item".to_string()),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: ir::Name::new("position".to_string()),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
            ir::Param {
                name: ir::Name::new("last".to_string()),
                type_: None,
                default: None,
                required: false,
                original_name: None,
                tunnel: false,
            },
        ],
        return_type: None,
        body: Box::new(spanned(let_root)),
    };

    let outer_expr = spanned(ir::Expr::FunctionDefinition(function_definition));

    // now that we have the IR, create bytecode
    let static_context = xee_interpreter::context::StaticContext::default();
    let mut program = Program::new(static_context, (0..0).into());
    let function_builder = FunctionBuilder::new(&mut program);
    let mut scopes = Scopes::new();
    let empty_mode_ids = ModeIds::new();
    let empty_template_ids = ahash::HashMap::new();
    let empty_template_params = ahash::HashMap::new();
    let empty_global_variable_ids = ahash::HashMap::new();
    let mut compiler = FunctionCompiler::new(
        function_builder,
        &mut scopes,
        &empty_mode_ids,
        &empty_template_ids,
        &empty_template_params,
        &empty_global_variable_ids,
    );

    compiler.compile_expr(&outer_expr).unwrap();

    assert_debug_snapshot!(decode_instructions(&program.functions[0].chunk));

    let dynamic_context_builder = program.dynamic_context_builder();
    let context = dynamic_context_builder.build();

    let mut xot = xot::Xot::new();
    let runnable = program.runnable(&context);
    let sequence = runnable.many(&mut xot).unwrap();
    // we should have the newly created element on top of the stack
    assert_eq!(sequence.len(), 1);
    assert_eq!(
        xot.to_string(sequence.iter().next().unwrap().to_node().unwrap())
            .unwrap(),
        "<foo/>"
    );
}
