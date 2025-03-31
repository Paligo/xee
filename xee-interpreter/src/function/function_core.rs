use std::rc::Rc;

use crate::{context, stack};

use super::array::Array;
use super::map::Map;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct InlineFunctionId(pub(crate) usize);

impl InlineFunctionId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }

    pub fn get(&self) -> usize {
        self.0
    }

    pub fn as_u16(&self) -> u16 {
        self.0 as u16
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct StaticFunctionId(pub(crate) usize);

impl StaticFunctionId {
    pub fn as_u16(&self) -> u16 {
        self.0 as u16
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Static(Rc<StaticFunctionData>),
    Inline(Rc<InlineFunctionData>),
    Map(Map),
    Array(Array),
}

#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(Function, [u8; 16]);

#[derive(Debug, PartialEq)]
pub struct StaticFunctionData {
    pub(crate) id: StaticFunctionId,
    pub(crate) closure_vars: Box<[stack::Value]>,
}

impl From<StaticFunctionData> for Function {
    fn from(data: StaticFunctionData) -> Self {
        Self::Static(Rc::new(data))
    }
}

impl StaticFunctionData {
    pub(crate) fn new(id: StaticFunctionId, closure_vars: Vec<stack::Value>) -> Self {
        Self {
            id,
            closure_vars: closure_vars.into(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct InlineFunctionData {
    pub(crate) id: InlineFunctionId,
    pub(crate) closure_vars: Box<[stack::Value]>,
}

impl From<InlineFunctionData> for Function {
    fn from(data: InlineFunctionData) -> Self {
        Self::Inline(Rc::new(data))
    }
}

impl InlineFunctionData {
    pub(crate) fn new(id: InlineFunctionId, closure_vars: Vec<stack::Value>) -> Self {
        Self {
            id,
            closure_vars: closure_vars.into(),
        }
    }
}

impl Function {
    pub(crate) fn closure_vars(&self) -> &[stack::Value] {
        match self {
            Self::Static(data) => &data.closure_vars,
            Self::Inline(data) => &data.closure_vars,
            _ => unreachable!(),
        }
    }

    pub fn display_representation(
        &self,
        xot: &xot::Xot,
        context: &context::DynamicContext,
    ) -> String {
        match self {
            Self::Static(data) => {
                let function = context.static_function_by_id(data.id);
                function.display_representation()
            }
            Self::Inline(data) => {
                let function = context.inline_function_by_id(data.id);
                function.display_representation()
            }
            Self::Map(map) => map.display_representation(xot, context),
            Self::Array(array) => array.display_representation(xot, context),
        }
    }
}
