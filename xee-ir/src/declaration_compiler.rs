use crate::{function_compiler::Scopes, ir, FunctionBuilder, FunctionCompiler};

use ahash::{HashMap, HashMapExt};
use xee_interpreter::{error, interpreter, pattern::ModeId};

pub type ModeIds = HashMap<ir::ApplyTemplatesModeValue, ModeId>;

pub struct DeclarationCompiler<'a> {
    program: &'a mut interpreter::Program,
    scopes: Scopes,
    mode_ids: ModeIds,
}

impl<'a> DeclarationCompiler<'a> {
    pub fn new(program: &'a mut interpreter::Program) -> Self {
        Self {
            program,
            scopes: Scopes::new(),
            mode_ids: HashMap::new(),
        }
    }

    fn function_compiler(&mut self) -> FunctionCompiler<'_> {
        let function_builder = FunctionBuilder::new(self.program);
        FunctionCompiler::new(function_builder, &mut self.scopes, &self.mode_ids)
    }

    pub fn compile_declarations(
        &mut self,
        declarations: &ir::Declarations,
    ) -> error::SpannedResult<()> {
        let declarations = declarations.clone();
        let mut function_compiler = self.function_compiler();
        function_compiler.compile_function_definition(
            &ir::FunctionDefinition {
                params: declarations.main.params,
                return_type: declarations.main.return_type,
                body: Box::new(ir::ExprS::new(
                    ir::Expr::DefineTemplates(ir::DefineTemplates {
                        rules: declarations.rules,
                        modes: declarations.modes,
                        body: declarations.main.body,
                    }),
                    (0..0).into(),
                )),
            },
            (0..0).into(),
        )
    }
}
