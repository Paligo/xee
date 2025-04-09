use xot::Xot;

use crate::{
    atomic::{self, AtomicCompare},
    context, error, function,
    string::Collation,
    xml,
};

use super::{
    comparison,
    item::Item,
    iter::{self, AtomizedIter, NodeIter},
};

pub(crate) type BoxedItemIter<'a> = Box<dyn Iterator<Item = Item> + 'a>;

/// The core sequence interface: a sequence must implement this to function.
///
/// If you do, SequenceExt provides a whole of APIs on top of it.
pub trait SequenceCore<'a, I>
where
    I: Iterator<Item = Item>,
{
    /// Check whether the sequence is empty
    fn is_empty(&self) -> bool;

    /// Get the sequence length
    fn len(&self) -> usize;

    /// Get an item in the index, if it exists
    fn get(&self, index: usize) -> Option<Item>;

    /// Get the items from the sequence as an iterator
    fn iter(&'a self) -> I;

    /// Get a single item from the sequence, if it only contains one item
    fn one(self) -> error::Result<Item>;

    /// Get a optional item from the sequence
    fn option(self) -> error::Result<Option<Item>>;

    /// Effective boolean value
    fn effective_boolean_value(&'a self) -> error::Result<bool>;

    /// String value
    fn string_value(&'a self, xot: &Xot) -> error::Result<String>;
}

pub trait SequenceExt<'a, I>: SequenceCore<'a, I>
where
    I: Iterator<Item = Item> + 'a,
{
    /// Access an iterator over the nodes in the sequence
    ///
    /// An error is returned for items that are not a node.
    fn nodes(&'a self) -> impl Iterator<Item = error::Result<xot::Node>> + 'a {
        NodeIter::new(self.iter())
    }

    /// Access an iterator for the atomized values in the sequence
    fn atomized(
        &'a self,
        xot: &'a Xot,
    ) -> impl Iterator<Item = error::Result<atomic::Atomic>> + 'a {
        AtomizedIter::new(xot, self.iter())
    }

    /// Get just one atomized value from the sequence
    fn atomized_one(&'a self, xot: &'a Xot) -> error::Result<atomic::Atomic> {
        iter::one(self.atomized(xot))?
    }

    /// Get an optional atomized value from the sequence
    fn atomized_option(&'a self, xot: &'a Xot) -> error::Result<Option<atomic::Atomic>> {
        iter::option(self.atomized(xot))?.transpose()
    }

    /// Is used internally by the library macro.
    fn unboxed_atomized<T: 'a>(
        &'a self,
        xot: &'a Xot,
        extract: impl Fn(atomic::Atomic) -> error::Result<T> + 'a,
    ) -> impl Iterator<Item = error::Result<T>> + 'a {
        self.atomized(xot).map(move |a| extract(a?))
    }

    /// Access an iterator over the XPath maps in the sequence
    ///
    /// An error is returned for items that are not a map.
    fn map_iter(&'a self) -> impl Iterator<Item = error::Result<function::Map>> {
        self.iter().map(|item| item.to_map())
    }

    /// Access an iterator over the XPath arrays in the sequence
    ///
    /// An error is returned for items that are not an array.
    fn array_iter(&'a self) -> impl Iterator<Item = error::Result<function::Array>> {
        self.iter().map(|item| item.to_array())
    }

    /// Access an iterator over elements nodes in the sequence
    ///
    /// An error is returned for items that are not an element.
    fn elements(
        &'a self,
        xot: &'a Xot,
    ) -> error::Result<impl Iterator<Item = error::Result<xot::Node>>> {
        Ok(self.nodes().map(|n| match n {
            Ok(n) => {
                if xot.is_element(n) {
                    Ok(n)
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
            Err(n) => Err(n),
        }))
    }

    /// Create an XPath array from this sequence.
    fn to_array(&'a self) -> error::Result<function::Array> {
        let mut array = Vec::with_capacity(self.len());
        for item in self.iter() {
            array.push(item.into());
        }
        // TODO: array.into() is somehow returning a Result, that seems weird
        // if this is really fallible, it should be try into. If it's not,
        // this whole function should be infallible.
        Ok(array.into())
    }
}

pub(crate) trait SequenceCompare<'a, I>: SequenceExt<'a, I>
where
    I: Iterator<Item = Item> + 'a,
{
    fn general_comparison<O, J>(
        &'a self,
        other: &'a impl SequenceExt<'a, J>,
        op: O,
        context: &context::DynamicContext,
        xot: &'a Xot,
    ) -> error::Result<bool>
    where
        O: AtomicCompare,
        J: Iterator<Item = Item> + 'a,
    {
        let a_atomized = self.atomized(xot);
        let b_atomized = other.atomized(xot);
        // optimization:
        // if a is actually smaller than b, then we want to pass a as the second argument,
        // because a gets collected by the general comparison logic, and we'd rather
        // collect the smallest vector
        let (a_lower_bound, _) = a_atomized.size_hint();
        let (b_lower_bound, _) = b_atomized.size_hint();
        // if the lower bound of a is smaller than that of b, we invert the comparison
        // so that a (a smaller sequence) is actually collected and then compared with.
        if a_lower_bound < b_lower_bound {
            comparison::general_comparison(b_atomized, a_atomized, context, O::arguments_inverted())
        } else {
            comparison::general_comparison(a_atomized, b_atomized, context, op)
        }
    }

    fn value_compare<O, J>(
        &'a self,
        other: &'a impl SequenceExt<'a, J>,
        _op: O,
        collation: &Collation,
        timezone: chrono::FixedOffset,
        xot: &'a Xot,
    ) -> error::Result<bool>
    where
        O: AtomicCompare,
        J: Iterator<Item = Item> + 'a,
    {
        let a = self.atomized_one(xot)?;
        let b = other.atomized_one(xot)?;
        O::atomic_compare(a, b, |a: &str, b: &str| collation.compare(a, b), timezone)
    }
}

pub(crate) trait SequenceOrder<'a, I>: SequenceCore<'a, I>
where
    I: Iterator<Item = Item>,
{
    fn one_node(&self) -> error::Result<xot::Node>;

    fn is<J>(&'a self, other: &'a impl SequenceOrder<'a, J>) -> error::Result<bool>
    where
        J: Iterator<Item = Item> + 'a,
    {
        let a = self.one_node()?;
        let b = other.one_node()?;
        Ok(a == b)
    }

    fn precedes<J>(
        &'a self,
        other: &'a impl SequenceOrder<'a, J>,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<bool>
    where
        J: Iterator<Item = Item> + 'a,
    {
        let a = self.one_node()?;
        let b = other.one_node()?;
        let a_order = annotations.get(a);
        let b_order = annotations.get(b);
        Ok(a_order < b_order)
    }

    fn follows<J>(
        &'a self,
        other: &'a impl SequenceOrder<'a, J>,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<bool>
    where
        J: Iterator<Item = Item> + 'a,
    {
        let a = self.one_node()?;
        let b = other.one_node()?;
        let a_order = annotations.get(a);
        let b_order = annotations.get(b);
        Ok(a_order > b_order)
    }
}
