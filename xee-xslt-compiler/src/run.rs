use xee_name::{Namespaces, FN_NAMESPACE};
use xot::{Node, Xot};

use xee_interpreter::context::StaticContext;
use xee_interpreter::error;
use xee_interpreter::interpreter::Program;
use xee_interpreter::sequence;

use crate::ast_ir::parse;

pub fn evaluate_program(
    xot: &mut Xot,
    program: &Program,
    root: Node,
) -> error::SpannedResult<sequence::Sequence> {
    let mut documents = xee_interpreter::xml::Documents::new();
    let handle = documents.add_root(None, root).unwrap();
    let root = documents.get_node_by_handle(handle).unwrap();
    let mut dynamic_context_builder = program.dynamic_context_builder();
    dynamic_context_builder.context_node(root);
    dynamic_context_builder.documents(documents);
    let context = dynamic_context_builder.build();
    let runnable = program.runnable(&context);
    runnable.many(xot)
}

pub fn evaluate(xot: &mut Xot, xml: &str, xslt: &str) -> error::SpannedResult<sequence::Sequence> {
    let namespaces = Namespaces::new(
        Namespaces::default_namespaces(),
        "".to_string(),
        FN_NAMESPACE.to_string(),
    );
    let static_context = StaticContext::from_namespaces(namespaces);
    let root = xot.parse(xml).unwrap();
    let program = parse(static_context, xslt).unwrap();
    evaluate_program(xot, &program, root)
}
