//! Queries you can execute against a document.

use std::rc::Rc;

use xee_interpreter::context::{self, StaticContext};
use xee_interpreter::error::SpannedResult as Result;
use xee_interpreter::interpreter::Program;
use xee_interpreter::sequence::{Item, Sequence};

use crate::{Documents, Itemable};

// import only for documentation purposes
#[cfg(doc)]
use crate::context::DynamicContextBuilder;
#[cfg(doc)]
use crate::Queries;

/// A query that can be executed against an [`Itemable`]
///
/// It gives back a result of type `V`
pub trait Query<V> {
    /// Get the program for the query
    fn program(&self) -> &Program;

    /// Get the static context for the query.
    fn static_context(&self) -> &StaticContext {
        self.program().static_context()
    }

    /// Execute the query against a dynamic context
    ///
    /// You can construct one using a [`DynamicContextBuilder`]
    fn execute_with_context(
        &self,
        documents: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<V>;

    /// Get a dynamic context builder for the query, configured with the
    /// query's static context and the document's documents.
    ///
    /// You can use this if you want to construct your own dynamic context
    /// to use with `execute_with_context`.
    fn dynamic_context_builder(&self, documents: &Documents) -> context::DynamicContextBuilder<'_> {
        let mut context = self.program().dynamic_context_builder();
        context.documents(documents.documents().clone());
        context
    }

    /// Map the the result of the query to a different type.
    ///
    /// You need to provide a function that takes the result of the query,
    /// the document, and the item, and returns a new result.
    fn map<T, F>(self, f: F) -> MapQuery<V, T, Self, F>
    where
        Self: Sized,
        F: Fn(V, &mut Documents, &context::DynamicContext) -> Result<T> + Clone,
    {
        MapQuery {
            query: self,
            f,
            v: std::marker::PhantomData,
            t: std::marker::PhantomData,
        }
    }

    /// Excute the query against an itemable
    fn execute(&self, documents: &mut Documents, item: impl Itemable) -> Result<V> {
        let context_item = item.to_item(documents)?;
        self.execute_build_context(documents, move |builder| {
            builder.context_item(context_item);
        })
    }

    /// Execute a query with a specific dynamic context.
    ///
    /// This is useful if you want to build a dynamic context with specific
    /// settings (such as variables), and then execute a query against it.
    fn execute_build_context(
        &self,
        documents: &mut Documents,
        build: impl FnOnce(&mut context::DynamicContextBuilder),
    ) -> Result<V> {
        let mut dynamic_context_builder = self.dynamic_context_builder(documents);
        build(&mut dynamic_context_builder);
        let context = dynamic_context_builder.build();
        self.execute_with_context(documents, &context)
    }
}

/// A recursive query that can be executed against an [`Itemable`]
///
/// It gives back a result of type `V`
pub trait RecurseQuery<C, V> {
    /// Get the program for the query
    fn program(&self) -> &Program;

    /// Get the static context for the query.
    fn static_context(&self) -> &StaticContext {
        self.program().static_context()
    }

    /// Execute the query against an itemable, with context.
    ///
    /// To do the conversion pass in a [`Recurse`] object. This
    /// allows you to use a convert function recursively.
    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<C>;

    /// Get a dynamic context builder for the query, configured with the
    /// query's static context and the document's documents.
    ///
    /// You can use this if you want to construct your own dynamic context
    /// to use with `execute_with_context`.
    fn dynamic_context_builder(&self, document: &Documents) -> context::DynamicContextBuilder<'_> {
        let mut context = self.program().dynamic_context_builder();
        context.documents(document.documents.clone());
        context
    }

    /// Execute the query against an itemable.
    ///
    /// To do the conversion pass in a [`Recurse`] object. This
    /// allows you to use a convert function recursively.
    fn execute(&self, document: &mut Documents, item: &Item, recurse: &Recurse<V>) -> Result<C> {
        self.execute_build_context(document, recurse, |builder| {
            builder.context_item(item.clone());
        })
    }

    /// Execute a query against an itemable, building the context.
    ///
    /// This is useful if you want to build a dynamic context with specific
    /// settings (such as variables), and then execute a query against it.
    fn execute_build_context(
        &self,
        document: &mut Documents,
        recurse: &Recurse<V>,
        build: impl FnOnce(&mut context::DynamicContextBuilder),
    ) -> Result<C> {
        let mut dynamic_context_builder = self.dynamic_context_builder(document);
        build(&mut dynamic_context_builder);
        let context = dynamic_context_builder.build();
        self.execute_with_context(document, &context, recurse)
    }
}

/// This is the core conversion function that can be used to turn
/// an item that results from an XPath query into something useful
/// in Rust.
///
/// Given a [`Documents`] and an [`Item`], convert the item to a value of type `V`
pub trait Convert<V>: Fn(&mut Documents, &Item) -> Result<V> {}
impl<V, T> Convert<V> for T where T: Fn(&mut Documents, &Item) -> Result<V> {}

// Recursion was very hard to get right. The trick is to use an intermediate
// struct.
// https://stackoverflow.com/questions/16946888/is-it-possible-to-make-a-recursive-closure-in-rust

// The dyn and reference are unavoidable, as closures are not allowed
// to refer to themselves:
// https://github.com/rust-lang/rust/issues/46062
type RecurseFn<'s, V> = &'s dyn Fn(&mut Documents, &Item, &Recurse<'s, V>) -> Result<V>;

/// An object that can be used to use a conversion function recursively.
pub struct Recurse<'s, V> {
    f: RecurseFn<'s, V>,
}

impl<'s, V> Recurse<'s, V> {
    /// Create a new recurse object given a conversion function.
    pub fn new(f: RecurseFn<'s, V>) -> Self {
        Self { f }
    }

    /// Execute the conversion function against an item.
    pub fn execute(&self, document: &mut Documents, item: &Item) -> Result<V> {
        (self.f)(document, item, self)
    }
}

/// This is a query that expects a sequence that contains exactly one single item.
///
/// Construct this using [`Queries::one`].
///
/// If it's empty or has more than one item, an error is returned.
///
/// The resulting item is converted into a Rust value using the `convert` function
/// when constructing this query.
///
/// This is useful if you expect a single item to be returned from an XPath query.
#[derive(Debug, Clone)]
pub struct OneQuery<V, F>
where
    F: Convert<V>,
{
    pub(crate) program: Rc<Program>,
    pub(crate) convert: F,
    pub(crate) phantom: std::marker::PhantomData<V>,
}

impl<V, F> OneQuery<V, F>
where
    F: Convert<V>,
{
    /// Execute the query against a context
    pub fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<V> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let item = sequence.one()?;
        (self.convert)(document, &item)
    }
}

impl<V, F> Query<V> for OneQuery<V, F>
where
    F: Convert<V>,
{
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<V> {
        OneQuery::execute_with_context(self, document, context)
    }
}

/// A recursive query that expects a single item as a result.
#[derive(Debug, Clone)]
pub struct OneRecurseQuery {
    pub(crate) program: Rc<Program>,
}

impl OneRecurseQuery {
    /// Execute the query against an itemable, with context.
    ///
    /// To do the conversion pass in a [`Recurse`] object. This
    /// allows you to use a convert function recursively.
    pub fn execute_with_context<V>(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<V> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let item = sequence.one()?;
        recurse.execute(document, &item)
    }
}

impl<V> RecurseQuery<V, V> for OneRecurseQuery {
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<V> {
        OneRecurseQuery::execute_with_context(self, document, context, recurse)
    }
}

/// This is a query that expects an optional single item.
///
/// Construct this using ['Queries::option'].
///
/// If the sequence is empty, `None` is returned. If it contains more than one
/// item, an error is returned.
///
/// The result is converted into a Rust value using the `convert` function
/// when constructing this query.
///
/// This is useful if you expect an optional single item to be returned from an XPath query.
#[derive(Debug, Clone)]
pub struct OptionQuery<V, F>
where
    F: Convert<V>,
{
    pub(crate) program: Rc<Program>,
    pub(crate) convert: F,
    pub(crate) phantom: std::marker::PhantomData<V>,
}

impl<V, F> OptionQuery<V, F>
where
    F: Convert<V>,
{
    /// Execute the query against an itemable, with explicit
    /// dynamic context.
    pub fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Option<V>> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let item = sequence.option()?;
        item.map(|item| (self.convert)(document, &item)).transpose()
    }
}

impl<V, F> Query<Option<V>> for OptionQuery<V, F>
where
    F: Convert<V>,
{
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Option<V>> {
        Self::execute_with_context(self, document, context)
    }
}

/// A recursive query that expects an optional single item.
#[derive(Debug, Clone)]
pub struct OptionRecurseQuery {
    pub(crate) program: Rc<Program>,
}

impl OptionRecurseQuery {
    /// Execute the recursive query against an explicit dynamic context.
    pub fn execute_with_context<V>(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<Option<V>> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let item = sequence.option()?;
        item.map(|item| recurse.execute(document, &item))
            .transpose()
    }
}

impl<V> RecurseQuery<Option<V>, V> for OptionRecurseQuery {
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<Option<V>> {
        OptionRecurseQuery::execute_with_context(self, document, context, recurse)
    }
}

/// A query that expects many items as a result.
///
/// Construct this using [`Queries::many`].
///
/// The items are converted into Rust values using the supplied `convert` function.
///
/// This is useful if you expect many items to be returned from an XPath query.
///
/// The result is converted into a Rust value using the `convert` function
/// when constructing this query.
#[derive(Debug, Clone)]
pub struct ManyQuery<V, F>
where
    F: Convert<V>,
{
    pub(crate) program: Rc<Program>,
    pub(crate) convert: F,
    pub(crate) phantom: std::marker::PhantomData<V>,
}

impl<V, F> ManyQuery<V, F>
where
    F: Convert<V>,
{
    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Vec<V>> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let items = sequence
            .iter()
            .map(|item| (self.convert)(document, &item))
            .collect::<Result<Vec<V>>>()?;
        Ok(items)
    }
}

impl<V, F> Query<Vec<V>> for ManyQuery<V, F>
where
    F: Convert<V>,
{
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Vec<V>> {
        Self::execute_with_context(self, document, context)
    }
}

/// A recursive query that expects many items as a result.
#[derive(Debug, Clone)]
pub struct ManyRecurseQuery {
    pub(crate) program: Rc<Program>,
}

impl ManyRecurseQuery {
    /// Execute the query against an itemable, with variables.
    ///
    /// To do the conversion pass in a [`Recurse`] object. This
    /// allows you to use a convert function recursively.
    pub fn execute_with_context<V>(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<Vec<V>> {
        let sequence = self.program.runnable(context).many(document.xot_mut())?;
        let items = sequence
            .iter()
            .map(|item| recurse.execute(document, &item))
            .collect::<Result<Vec<V>>>()?;
        Ok(items)
    }
}

impl<V> RecurseQuery<Vec<V>, V> for ManyRecurseQuery {
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
        recurse: &Recurse<V>,
    ) -> Result<Vec<V>> {
        ManyRecurseQuery::execute_with_context(self, document, context, recurse)
    }
}

/// A query that returns a sequence.
///
/// This query returns a [`Sequence`] object that can be used to access
/// the items in the sequence. It represents an XPath sequence. The items
/// in the sequence are not converted.
///
/// Construct this using [`Queries::sequence`].
///
/// This is useful if you want to work with the sequence directly.
#[derive(Debug, Clone)]
pub struct SequenceQuery {
    pub(crate) program: Rc<Program>,
}

impl SequenceQuery {
    /// Execute the query against an itemable with an explict dynamic context.
    pub fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Sequence> {
        self.program.runnable(context).many(document.xot_mut())
    }
}

impl Query<Sequence> for SequenceQuery {
    fn program(&self) -> &Program {
        &self.program
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<Sequence> {
        Self::execute_with_context(self, document, context)
    }
}

/// A query maps the result of another query to a different type.
#[derive(Debug, Clone)]
pub struct MapQuery<V, T, Q: Query<V> + Sized, F>
where
    F: Fn(V, &mut Documents, &context::DynamicContext) -> Result<T> + Clone,
{
    query: Q,
    f: F,
    v: std::marker::PhantomData<V>,
    t: std::marker::PhantomData<T>,
}

impl<V, T, Q, F> MapQuery<V, T, Q, F>
where
    Q: Query<V> + Sized,
    F: Fn(V, &mut Documents, &context::DynamicContext) -> Result<T> + Clone,
{
    /// Execute the query against an item.
    pub fn execute(&self, document: &mut Documents, item: &Item) -> Result<T> {
        let mut dynamic_context_builder = self.query.program().dynamic_context_builder();
        dynamic_context_builder.context_item(item.clone());
        let context = dynamic_context_builder.build();
        self.execute_with_context(document, &context)
    }

    /// Execute the query against a dynamic context.
    pub fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<T> {
        let v = self.query.execute_with_context(document, context)?;
        // TODO: this isn't right. need to rewrite in terms of dynamic context too?
        (self.f)(v, document, context)
    }
}

impl<V, T, Q: Query<V> + Sized, F> Query<T> for MapQuery<V, T, Q, F>
where
    F: Fn(V, &mut Documents, &context::DynamicContext) -> Result<T> + Clone,
{
    fn program(&self) -> &Program {
        self.query.program()
    }

    fn execute_with_context(
        &self,
        document: &mut Documents,
        context: &context::DynamicContext,
    ) -> Result<T> {
        let v = self.query.execute_with_context(document, context)?;
        (self.f)(v, document, context)
    }
}
