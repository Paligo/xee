use xot::Xot;

use crate::{atomic, error};

use crate::sequence::traits::{SequenceCompare, SequenceCore, SequenceExt, SequenceOrder};
use crate::sequence::Item;

// this size should be below a usize
const MAXIMUM_RANGE_SIZE: i64 = 2_i64.pow(25);

#[derive(Debug, Clone, PartialEq)]
pub struct Empty {}

impl SequenceCore<'_, std::iter::Empty<Item>> for Empty {
    #[inline]
    fn is_empty(&self) -> bool {
        true
    }

    #[inline]
    fn len(&self) -> usize {
        0
    }

    #[inline]
    fn get(&self, _index: usize) -> Option<Item> {
        None
    }

    #[inline]
    fn one(self) -> error::Result<Item> {
        Err(error::Error::XPTY0004)
    }

    #[inline]
    fn option(self) -> error::Result<Option<Item>> {
        Ok(None)
    }

    #[inline]
    fn iter(&self) -> std::iter::Empty<Item> {
        std::iter::empty()
    }

    #[inline]
    fn effective_boolean_value(&self) -> error::Result<bool> {
        Ok(false)
    }

    #[inline]
    fn string_value(&self, _xot: &xot::Xot) -> error::Result<String> {
        Ok(String::new())
    }
}

// We implement specific interfaces here, instead of generically for all
// T that implement sequenceCore, because we want to provide a few specialized
// implementations.
impl<'a, I> SequenceExt<'a, I> for Empty
where
    I: Iterator<Item = Item> + 'a,
    Self: SequenceCore<'a, I>,
{
    fn atomized(
        &'a self,
        _xot: &'a xot::Xot,
    ) -> impl Iterator<Item = error::Result<atomic::Atomic>> + 'a {
        std::iter::empty()
    }

    /// Get just one atomized value from the sequence
    fn atomized_one(&'a self, _xot: &'a Xot) -> error::Result<atomic::Atomic> {
        Err(error::Error::XPTY0004)
    }

    /// Get an optional atomized value from the sequence
    fn atomized_option(&'a self, _xot: &'a Xot) -> error::Result<Option<atomic::Atomic>> {
        Ok(None)
    }
}

impl<'a, I> SequenceCompare<'a, I> for Empty
where
    I: Iterator<Item = Item> + 'a,
    Self: SequenceCore<'a, I>,
{
}

impl<'a, I> SequenceOrder<'a, I> for Empty
where
    I: Iterator<Item = Item>,
    Self: SequenceCore<'a, I>,
{
    fn one_node(&self) -> error::Result<xot::Node> {
        Err(error::Error::XPTY0004)
    }
}
