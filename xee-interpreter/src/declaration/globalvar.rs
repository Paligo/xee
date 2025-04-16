use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use std::{cell::RefCell, rc::Rc};

use xee_name::Name;

use crate::error::{Error, ErrorCode, Result};

type Resolver<'a, V> = dyn Fn(Box<dyn Fn(&'a Name) -> Result<V> + 'a>) -> Result<V> + 'a;

pub(crate) struct GlobalVariables<'a, V: Clone + 'a> {
    declarations: HashSet<&'a Name>,
    resolvers: HashMap<&'a Name, Box<Resolver<'a, V>>>,
    resolved: RefCell<HashMap<&'a Name, V>>,
}

impl<'a, V: Clone + 'a> GlobalVariables<'a, V> {
    pub(crate) fn new() -> Self {
        Self {
            declarations: HashSet::new(),
            resolvers: HashMap::new(),
            resolved: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn add_declaration(&mut self, name: &'a Name) {
        self.declarations.insert(name);
    }

    pub(crate) fn add_resolver<F>(&mut self, name: &'a Name, resolver: F)
    where
        F: Fn(Box<dyn Fn(&'a Name) -> Result<V> + 'a>) -> Result<V> + 'a,
    {
        self.resolvers.insert(name, Box::new(resolver));
    }

    pub(crate) fn get(self: &Rc<Self>, name: &'a Name) -> Result<V> {
        self.get_internal(name, HashSet::new())
    }

    fn get_resolve(
        self: &Rc<Self>,
        name_seen: &'a Name,
        seen: HashSet<&'a Name>,
    ) -> Box<dyn Fn(&'a Name) -> Result<V> + 'a> {
        let s = self.clone();
        Box::new(move |name: &'a Name| {
            let mut new_seen = seen.clone();
            new_seen.insert(name_seen);
            s.get_internal(name, new_seen)
        })
    }

    fn get_internal(self: &Rc<Self>, name: &'a Name, seen: HashSet<&'a Name>) -> Result<V> {
        if let Some(value) = self.resolved.borrow().get(name) {
            return Ok(value.clone());
        }
        let resolve = self.resolvers.get(name).unwrap();
        if seen.contains(name) {
            return Err(Error::new(ErrorCode::XTDE0640));
        }

        let value = resolve(self.get_resolve(name, seen))?;

        let mut resolved = self.resolved.borrow_mut();
        resolved.insert(name, value.clone());
        Ok(value)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_single_global_variable() {
        let foo = Name::name("foo");
        let bar = Name::name("bar");
        // first declare a few global variables
        let mut global_variables = GlobalVariables::<u64>::new();
        global_variables.add_declaration(&foo);
        global_variables.add_declaration(&bar);

        // now something that uses the global variables
        global_variables.add_resolver(&bar, |_| Ok(2));
        global_variables.add_resolver(&foo, |resolve| Ok(resolve(&bar)? + 1));

        // now we can resolve foo and bar
        let global_variables = Rc::new(global_variables);
        assert_eq!(global_variables.get(&foo), Ok(3));
        assert_eq!(global_variables.get(&bar), Ok(2));
    }

    #[test]
    fn test_circular() {
        let foo = Name::name("foo");
        let bar = Name::name("bar");
        // first declare a few global variables
        let mut global_variables = GlobalVariables::<u64>::new();
        global_variables.add_declaration(&foo);
        global_variables.add_declaration(&bar);

        // now something that uses the global variables
        global_variables.add_resolver(&bar, |resolve| resolve(&foo));
        global_variables.add_resolver(&foo, |resolve| Ok(resolve(&bar)? + 1));

        // now we can resolve foo but resolution fails as there is a circular dependency
        let global_variables = Rc::new(global_variables);
        assert_eq!(
            global_variables.get(&foo),
            Err(Error::new(ErrorCode::XTDE0640))
        );
    }

    #[test]
    fn test_cache() {
        let foo = Name::name("foo");
        let bar = Name::name("bar");
        // first declare a few global variables
        let mut global_variables = GlobalVariables::<u64>::new();
        global_variables.add_declaration(&foo);
        global_variables.add_declaration(&bar);

        struct Counter {
            count: RefCell<usize>,
        }
        impl Counter {
            fn plus(&self) {
                let mut c = self.count.borrow_mut();
                (*c) += 1;
            }
            fn get(&self) -> usize {
                *self.count.borrow()
            }
        }
        let counter = Rc::new(Counter {
            count: RefCell::new(0),
        });
        let current_counter = counter.clone();
        global_variables.add_resolver(&foo, move |_resolve| {
            current_counter.plus();
            Ok(1)
        });

        // now we can resolve foo and if we resolve it twice, the resolver is only called once
        let global_variables = Rc::new(global_variables);
        assert_eq!(global_variables.get(&foo), Ok(1));
        assert_eq!(global_variables.get(&foo), Ok(1));
        assert_eq!(counter.get(), 1);
    }
}
