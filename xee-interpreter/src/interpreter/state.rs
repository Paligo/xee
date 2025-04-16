use std::cell::RefCell;
use std::rc::Rc;

use ahash::HashMap;
use ahash::HashMapExt;
use arrayvec::ArrayVec;
use xot::Xot;

use crate::error;
use crate::function;
use crate::sequence;
use crate::stack;

const FRAMES_MAX: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    function: function::InlineFunctionId,
    base: usize,
    pub(crate) ip: usize,
}

impl Frame {
    pub(crate) fn function(&self) -> function::InlineFunctionId {
        self.function
    }
    pub(crate) fn base(&self) -> usize {
        self.base
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RegexKey {
    pattern: String,
    flags: String,
}

#[derive(Debug)]
pub struct State<'a> {
    stack: Vec<stack::Value>,
    build_stack: Vec<BuildStackEntry>,
    frames: ArrayVec<Frame, FRAMES_MAX>,
    regex_cache: RefCell<HashMap<RegexKey, Rc<regexml::Regex>>>,
    pub(crate) xot: &'a mut Xot,
}

#[derive(Debug)]
struct ItemBuildStackEntry {
    build_stack: Vec<sequence::Item>,
}

#[derive(Debug)]
struct BuildStackEntry {
    item: ItemBuildStackEntry,
}

impl BuildStackEntry {
    fn new() -> Self {
        Self {
            item: ItemBuildStackEntry {
                build_stack: Vec::new(),
            },
        }
    }

    fn push(&mut self, item: sequence::Item) {
        self.item.build_stack.push(item);
    }

    fn extend<I: Iterator<Item = sequence::Item>>(
        &mut self,
        items: impl IntoIterator<Item = sequence::Item, IntoIter = I>,
    ) {
        self.item.build_stack.extend(items);
    }
}

impl From<BuildStackEntry> for sequence::Sequence {
    fn from(build: BuildStackEntry) -> Self {
        sequence::Sequence::new(build.item.build_stack)
    }
}

impl<'a> State<'a> {
    pub(crate) fn new(xot: &'a mut Xot) -> Self {
        Self {
            stack: vec![],
            build_stack: vec![],
            frames: ArrayVec::new(),
            regex_cache: RefCell::new(HashMap::new()),
            xot,
        }
    }

    pub(crate) fn push<T>(&mut self, sequence: T)
    where
        T: Into<sequence::Sequence>,
    {
        let sequence: sequence::Sequence = sequence.into();
        self.stack.push(sequence.into());
    }

    pub(crate) fn push_value<T>(&mut self, value: T)
    where
        T: Into<stack::Value>,
    {
        self.stack.push(value.into());
    }

    pub(crate) fn build_new(&mut self) {
        self.build_stack.push(BuildStackEntry::new());
    }

    pub(crate) fn build_push(&mut self) -> error::Result<()> {
        let value = self.pop()?;
        let build = self.build_stack.last_mut().unwrap();
        match value {
            sequence::Sequence::Empty(_) => {}
            sequence::Sequence::One(item) => build.push(item.into_item()),
            // any other sequence
            sequence => build.extend(sequence.iter()),
        }
        Ok(())
    }

    pub(crate) fn build_complete(&mut self) {
        let build = self.build_stack.pop().unwrap();
        self.stack.push(build.into());
    }

    pub(crate) fn push_var(&mut self, index: usize) {
        self.stack
            .push(self.stack[self.frame().base + index].clone());
    }

    pub(crate) fn push_closure_var(&mut self, index: usize) -> error::Result<()> {
        let function = self.function()?;
        let closure_vars = function.closure_vars();
        self.stack.push(closure_vars[index].clone());
        Ok(())
    }

    pub(crate) fn set_var(&mut self, index: usize) {
        let base = self.frame().base;
        self.stack[base + index] = self.stack.pop().unwrap();
    }

    #[inline]
    pub(crate) fn pop(&mut self) -> error::Result<sequence::Sequence> {
        self.pop_value().try_into()
    }

    #[inline]
    pub(crate) fn pop_value(&mut self) -> stack::Value {
        self.stack.pop().unwrap()
    }

    pub(crate) fn function(&self) -> error::Result<function::Function> {
        // the function is always just below the base
        let value = &self.stack[self.frame().base - 1];
        match value {
            stack::Value::Sequence(sequence) => sequence.clone().try_into(),
            stack::Value::Absent => Err(error::Error::new(error::ErrorCode::XPDY0002)),
        }
    }

    pub(crate) fn push_start_frame(&mut self, function_id: function::InlineFunctionId) {
        self.frames.push(Frame {
            function: function_id,
            ip: 0,
            base: 0,
        });
    }

    pub(crate) fn push_frame(
        &mut self,
        function_id: function::InlineFunctionId,
        arity: usize,
    ) -> error::Result<()> {
        if self.frames.len() >= self.frames.capacity() {
            return Err(error::Error::new(error::ErrorCode::StackOverflow));
        }
        self.frames.push(Frame {
            function: function_id,
            ip: 0,
            base: self.stack.len() - arity,
        });
        Ok(())
    }

    pub(crate) fn frame(&self) -> &Frame {
        self.frames.last().unwrap()
    }

    pub(crate) fn frame_mut(&mut self) -> &mut Frame {
        self.frames.last_mut().unwrap()
    }

    pub(crate) fn jump(&mut self, displacement: i32) {
        self.frame_mut().ip = (self.frame().ip as i32 + displacement) as usize;
    }

    pub(crate) fn callable(&self, arity: usize) -> error::Result<function::Function> {
        let value = &self.stack[self.stack.len() - (arity + 1)];
        match value {
            stack::Value::Sequence(sequence) => sequence.clone().try_into(),
            stack::Value::Absent => Err(error::Error::new(error::ErrorCode::XPDY0002)),
        }
    }

    pub(crate) fn arguments(&self, arity: usize) -> &[stack::Value] {
        &self.stack[self.stack.len() - arity..]
    }

    pub(crate) fn truncate_arguments(&mut self, arity: usize) {
        self.stack.truncate(self.stack.len() - arity);
    }

    pub(crate) fn inline_return(&mut self, start_base: usize) -> bool {
        let return_value = self.stack.pop().unwrap();

        // truncate the stack to the base
        let base = self.frame().base;
        self.stack.truncate(base);

        // pop off the function id we just called
        // for the outer main function this is the context item
        if !self.stack.is_empty() {
            self.stack.pop();
        }

        // push back return value
        self.stack.push(return_value);

        // now pop off the frame
        self.frames.pop();

        // if the start base is the same as the base we just popped off,
        // we are done
        base == start_base
    }

    pub(crate) fn top(&self) -> error::Result<sequence::Sequence> {
        self.stack.last().unwrap().try_into()
    }

    pub fn stack(&self) -> &[stack::Value] {
        &self.stack
    }

    pub fn regex(&self, pattern: &str, flags: &str) -> error::Result<Rc<regexml::Regex>> {
        // TODO: would be nice if we could not do to_string here but use &str
        // but unfortunately otherwise lifetime issues bubble up all the way to
        // the library bindings if we do so
        let key = RegexKey {
            pattern: pattern.to_string(),
            flags: flags.to_string(),
        };
        let mut cache = self.regex_cache.borrow_mut();
        let entry = cache.entry(key);
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let v = entry.insert(Rc::new(regexml::Regex::xpath(pattern, flags)?));
                Ok(v.clone())
            }
        }
    }

    pub fn xot(&self) -> &Xot {
        self.xot
    }

    pub fn xot_mut(&mut self) -> &mut Xot {
        self.xot
    }
}
