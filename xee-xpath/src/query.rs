use xee_interpreter::error::Result;
use xee_interpreter::interpreter::Variables;
use xee_interpreter::{context::DynamicContext, context::StaticContext};
use xee_interpreter::{interpreter::Program, sequence::Item};

use crate::compile::parse;

pub trait Convert<V>: Fn(&Session, &Item) -> Result<V> {}
impl<V, T> Convert<V> for T where T: Fn(&Session, &Item) -> Result<V> {}

// Recursion was very hard to get right. The trick is to use an intermediate
// struct.
// https://stackoverflow.com/questions/16946888/is-it-possible-to-make-a-recursive-closure-in-rust

// The dyn and dereference are unavoidable, as closures are not allowed
// to refer to themselves:
// https://github.com/rust-lang/rust/issues/46062
type RecurseFn<'s, V> = &'s dyn Fn(&Session, &Item, &Recurse<'s, V>) -> Result<V>;

pub struct Recurse<'s, V> {
    f: RecurseFn<'s, V>,
}

impl<'s, V> Recurse<'s, V> {
    pub fn new(f: RecurseFn<'s, V>) -> Self {
        Self { f }
    }
    pub fn execute(&self, session: &Session, item: &Item) -> Result<V> {
        (self.f)(session, item, self)
    }
}

pub trait ConvertRecurse<V>: Fn(&Session, &Item) -> Result<V> {}

#[derive(Debug)]
pub struct Queries<'s> {
    queries: Vec<Program>,
    static_context: &'s StaticContext<'s>,
}

impl<'s> Queries<'s> {
    pub fn new(static_context: &'s StaticContext<'s>) -> Self {
        Self {
            queries: Vec::new(),
            static_context,
        }
    }

    pub fn session<'d>(&'d self, dynamic_context: &'d DynamicContext<'d>) -> Session {
        Session::new(dynamic_context, self)
    }

    fn register(&mut self, s: &str) -> Result<usize> {
        let program = parse(self.static_context, s).map_err(|e| e.error)?;
        let id = self.queries.len();
        self.queries.push(program);
        Ok(id)
    }

    pub fn one<V, F>(&mut self, s: &str, convert: F) -> Result<OneQuery<V, F>>
    where
        F: Convert<V>,
    {
        let id = self.register(s)?;
        Ok(OneQuery {
            id,
            convert,
            phantom: std::marker::PhantomData,
        })
    }

    pub fn one_recurse(&mut self, s: &str) -> Result<OneRecurseQuery> {
        let id = self.register(s)?;
        Ok(OneRecurseQuery { id })
    }

    pub fn option<V, F>(&mut self, s: &str, convert: F) -> Result<OptionQuery<V, F>>
    where
        F: Convert<V>,
    {
        let id = self.register(s)?;
        Ok(OptionQuery {
            id,
            convert,
            phantom: std::marker::PhantomData,
        })
    }

    pub fn option_recurse(&mut self, s: &str) -> Result<OptionRecurseQuery> {
        let id = self.register(s)?;
        Ok(OptionRecurseQuery { id })
    }

    pub fn many<V, F>(&mut self, s: &str, convert: F) -> Result<ManyQuery<V, F>>
    where
        F: Convert<V>,
    {
        let id = self.register(s)?;
        Ok(ManyQuery {
            id,
            convert,
            phantom: std::marker::PhantomData,
        })
    }

    pub fn many_recurse(&mut self, s: &str) -> Result<ManyRecurseQuery> {
        let id = self.register(s)?;
        Ok(ManyRecurseQuery { id })
    }
}

#[derive(Debug)]
pub struct Session<'s> {
    dynamic_context: &'s DynamicContext<'s>,
    queries: &'s Queries<'s>,
}

impl<'s> Session<'s> {
    pub fn new(dynamic_context: &'s DynamicContext<'s>, queries: &'s Queries<'s>) -> Self {
        Self {
            dynamic_context,
            queries,
        }
    }

    fn one_query_program(&self, id: usize) -> &Program {
        &self.queries.queries[id]
    }
}

pub trait Query<V> {
    fn execute(&self, session: &Session, item: &Item) -> Result<V>;
}

#[derive(Debug, Clone)]
pub struct OneQuery<V, F>
where
    F: Convert<V>,
{
    id: usize,
    convert: F,
    phantom: std::marker::PhantomData<V>,
}

impl<V, F> OneQuery<V, F>
where
    F: Convert<V>,
{
    pub fn execute(&self, session: &Session, item: &Item) -> Result<V> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let item = runnable
            .one(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        (self.convert)(session, &item)
    }
}

impl<V, F> Query<V> for OneQuery<V, F>
where
    F: Convert<V>,
{
    fn execute(&self, session: &Session, item: &Item) -> Result<V> {
        Self::execute(self, session, item)
    }
}

#[derive(Debug, Clone)]
pub struct OneRecurseQuery {
    id: usize,
}

impl OneRecurseQuery {
    pub fn execute<V>(&self, session: &Session, item: &Item, recurse: &Recurse<V>) -> Result<V> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let item = runnable
            .one(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        recurse.execute(session, &item)
    }
}

#[derive(Debug, Clone)]
pub struct OptionQuery<V, F>
where
    F: Convert<V>,
{
    id: usize,
    convert: F,
    phantom: std::marker::PhantomData<V>,
}

impl<V, F> OptionQuery<V, F>
where
    F: Convert<V>,
{
    pub fn execute(&self, session: &Session, item: &Item) -> Result<Option<V>> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let item = runnable
            .option(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        if let Some(item) = item {
            match (self.convert)(session, &item) {
                Ok(value) => Ok(Some(value)),
                Err(query_error) => Err(query_error),
            }
        } else {
            Ok(None)
        }
    }
}

impl<V, F> Query<Option<V>> for OptionQuery<V, F>
where
    F: Convert<V>,
{
    fn execute(&self, session: &Session, item: &Item) -> Result<Option<V>> {
        Self::execute(self, session, item)
    }
}

#[derive(Debug, Clone)]
pub struct OptionRecurseQuery {
    id: usize,
}

impl OptionRecurseQuery {
    pub fn execute<V>(
        &self,
        session: &Session,
        item: &Item,
        recurse: &Recurse<V>,
    ) -> Result<Option<V>> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let item = runnable
            .option(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        if let Some(item) = item {
            Ok(Some(recurse.execute(session, &item)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManyQuery<V, F>
where
    F: Convert<V>,
{
    id: usize,
    convert: F,
    phantom: std::marker::PhantomData<V>,
}

impl<V, F> ManyQuery<V, F>
where
    F: Convert<V>,
{
    pub fn execute(&self, session: &Session, item: &Item) -> Result<Vec<V>> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let sequence = runnable
            .many(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        let mut values = Vec::with_capacity(sequence.len());
        for item in sequence.items() {
            match (self.convert)(session, &item?) {
                Ok(value) => values.push(value),
                Err(query_error) => return Err(query_error),
            }
        }
        Ok(values)
    }
}

impl<V, F> Query<Vec<V>> for ManyQuery<V, F>
where
    F: Convert<V>,
{
    fn execute(&self, session: &Session, item: &Item) -> Result<Vec<V>> {
        Self::execute(self, session, item)
    }
}

#[derive(Debug, Clone)]
pub struct ManyRecurseQuery {
    id: usize,
}

impl ManyRecurseQuery {
    pub fn execute<V>(
        &self,
        session: &Session,
        item: &Item,
        recurse: &Recurse<V>,
    ) -> Result<Vec<V>> {
        let program = session.one_query_program(self.id);
        let runnable = program.runnable(session.dynamic_context);
        let sequence = runnable
            .many(Some(item), Variables::default())
            .map_err(|e| e.error)?;
        let mut values = Vec::with_capacity(sequence.len());
        for item in sequence.items() {
            values.push(recurse.execute(session, &item?)?);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use ibig::{ibig, IBig};
    use xee_interpreter::{error::Result, xml::Node};
    use xot::Xot;

    use super::*;

    #[test]
    fn test_one_query() {
        let static_context = StaticContext::default();
        let mut queries = Queries::new(&static_context);
        let q = queries
            .one("1 + 2", |_, item| {
                let v: Result<IBig> = item.to_atomic()?.try_into();
                v
            })
            .unwrap();

        let xot = Xot::new();
        let dynamic_context = DynamicContext::empty(xot, &static_context);
        let session = queries.session(&dynamic_context);
        let r = q.execute(&session, &1i64.into()).unwrap();
        assert_eq!(r, ibig!(3));
    }

    #[test]
    fn test_one_query_recurse() -> Result<()> {
        let static_context = StaticContext::default();
        let mut queries = Queries::new(&static_context);
        #[derive(Debug, PartialEq, Eq)]
        enum Expr {
            AnyOf(Box<Expr>),
            Value(String),
            Empty,
        }

        let any_of_recurse = queries.option_recurse("any-of")?;
        let value_query =
            queries.option("value/string()", |_, item| item.to_atomic()?.to_string())?;

        let result_query = queries.one("doc/result", |session: &Session, item: &Item| {
            let f = |session: &Session, item: &Item, recurse: &Recurse<Expr>| {
                let any_of = any_of_recurse.execute(session, item, recurse)?;
                if let Some(any_of) = any_of {
                    return Ok(Expr::AnyOf(Box::new(any_of)));
                }
                if let Some(value) = value_query.execute(session, item)? {
                    return Ok(Expr::Value(value));
                }
                Ok(Expr::Empty)
            };
            let recurse = Recurse::new(&f);
            recurse.execute(session, item)
        })?;

        let mut xot = Xot::new();
        let xml = "<doc><result><any-of><value>A</value></any-of></result></doc>";
        let root = xot.parse(xml).unwrap();
        let xml2 = "<doc><result><value>A</value></result></doc>";
        let root2 = xot.parse(xml2).unwrap();

        let dynamic_context = DynamicContext::empty(xot, &static_context);
        let session = queries.session(&dynamic_context);
        let r = result_query.execute(&session, &Item::from(Node::Xot(root)))?;
        assert_eq!(r, Expr::AnyOf(Box::new(Expr::Value("A".to_string()))));

        let session = queries.session(&dynamic_context);
        let r = result_query.execute(&session, &Item::from(Node::Xot(root2)))?;
        assert_eq!(r, Expr::Value("A".to_string()));
        Ok(())
    }
}
