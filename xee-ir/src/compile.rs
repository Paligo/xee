use ahash::{HashMap, HashMapExt};
use xee_interpreter::{
    context::StaticContext,
    error::SpannedResult,
    function,
    interpreter::Program,
};

use crate::{
    declaration_compiler::{DeclarationCompiler, ModeIds},
    ir, FunctionBuilder, FunctionCompiler, Scopes,
};

pub fn compile_xpath(expr: ir::ExprS, static_context: StaticContext) -> SpannedResult<Program> {
    let mut program = Program::new(static_context, expr.span);
    let mut scopes = Scopes::new();
    let builder = FunctionBuilder::new(&mut program);
    let empty_mode_ids = ModeIds::new();
    let empty_user_functions: Vec<function::InlineFunctionId> = Vec::new();
    let empty_named_templates: HashMap<xot::xmlname::OwnedName, function::InlineFunctionId> =
        HashMap::new();
    let mut compiler = FunctionCompiler::new(
        builder,
        &mut scopes,
        &empty_mode_ids,
        &empty_user_functions,
        &empty_named_templates,
    );
    compiler.compile_expr(&expr)?;
    Ok(program)
}

pub fn compile_xslt(
    declarations: ir::Declarations,
    static_context: StaticContext,
) -> SpannedResult<Program> {
    let mut program = Program::new(static_context, (0..0).into());
    let mut compiler = DeclarationCompiler::new(&mut program);
    compiler.compile_declarations(&declarations)?;
    Ok(program)
}
