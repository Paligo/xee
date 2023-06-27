use xee_xpath_ast::ast::parse_xpath;

use crate::context::{DynamicContext, StaticContext};
use crate::error::{Error, Result};
use crate::interpreter::{FunctionBuilder, Interpreter, InterpreterCompiler, Program, Scopes};
use crate::ir;
use crate::ir::IrConverter;
use crate::occurrence::ResultOccurrence;
use crate::output;
use crate::stack;
use crate::xml;

#[derive(Debug)]
pub struct XPath {
    pub(crate) program: Program,
    main: stack::FunctionId,
}

impl XPath {
    pub fn new(static_context: &StaticContext, xpath: &str) -> Result<Self> {
        let ast = parse_xpath(xpath, static_context.namespaces, &static_context.variables)?;
        let mut ir_converter = IrConverter::new(xpath, static_context);
        let expr = ir_converter.convert_xpath(&ast)?;
        // this expression contains a function definition, we're getting it
        // in the end
        let mut program = Program::new(xpath.to_string());
        let mut scopes = Scopes::new(ir::Name("dummy".to_string()));
        let builder = FunctionBuilder::new(&mut program);
        let mut compiler = InterpreterCompiler {
            builder,
            scopes: &mut scopes,
            static_context,
        };
        compiler.compile_expr(&expr)?;

        // the inline function should be the last finished function
        let inline_id = stack::FunctionId(program.functions.len() - 1);
        Ok(Self {
            program,
            main: inline_id,
        })
    }

    pub(crate) fn run_value(
        &self,
        dynamic_context: &DynamicContext,
        context_item: Option<&stack::Item>,
    ) -> Result<stack::Value> {
        let mut interpreter = Interpreter::new(&self.program, dynamic_context);
        let arguments = dynamic_context.arguments()?;
        interpreter.start(self.main, context_item, arguments);
        interpreter.run()?;
        // the stack has to be 1 values and return the result of the expression
        // why 1 value if the context item is on the top of the stack? This is because
        // the outer main function will pop the context item; this code is there to
        // remove the function id from the stack but the main function has no function id
        assert_eq!(
            interpreter.stack().len(),
            1,
            "stack must only have 1 value but found {:?}",
            interpreter.stack()
        );
        let value = interpreter.stack().last().unwrap().clone();
        match value {
            stack::Value::Absent => Err(Error::SpannedComponentAbsentInDynamicContext {
                src: self.program.src.clone(),
                span: (0, self.program.src.len()).into(),
            }),
            _ => Ok(value),
        }
    }

    pub fn many_xot_node(
        &self,
        dynamic_context: &DynamicContext,
        node: xot::Node,
    ) -> Result<output::Sequence> {
        let node = xml::Node::Xot(node);
        let item = stack::Item::Node(node);
        let output_item = output::Item::from(item);
        self.many(dynamic_context, Some(&output_item))
    }

    pub fn many(
        &self,
        dynamic_context: &DynamicContext,
        item: Option<&output::Item>,
    ) -> Result<output::Sequence> {
        let context_item: Option<stack::Item> = item.map(|item| item.into());
        let value = self.run_value(dynamic_context, context_item.as_ref())?;
        Ok(value.into_output())
    }

    pub fn one(
        &self,
        dynamic_context: &DynamicContext,
        item: Option<&output::Item>,
    ) -> Result<output::Item> {
        let context_item: Option<stack::Item> = item.map(|item| item.into());
        let value = self.run_value(dynamic_context, context_item.as_ref())?;
        value.into_output().items().one()
    }

    pub fn option(
        &self,
        dynamic_context: &DynamicContext,
        item: Option<&output::Item>,
    ) -> Result<Option<output::Item>> {
        let context_item: Option<stack::Item> = item.map(|item| item.into());
        let value = self.run_value(dynamic_context, context_item.as_ref())?;
        value.into_output().items().option()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use xee_xpath_ast::{Namespaces, FN_NAMESPACE};
    use xot::Xot;

    use crate::context::StaticContext;

    #[test]
    fn test_parse_error() {
        let mut xot = Xot::new();
        let uri = xml::Uri("http://example.com".to_string());
        let mut documents = xml::Documents::new();
        documents.add(&mut xot, &uri, "<doc/>").unwrap();
        let namespaces = Namespaces::new(None, Some(FN_NAMESPACE));
        let static_context = StaticContext::new(&namespaces);
        let xpath = "1 + 2 +";
        let r = XPath::new(&static_context, xpath);
        assert!(r.is_err())
    }
}
