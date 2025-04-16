use std::rc::Rc;

use crate::error;
use crate::sequence::traits::{SequenceCompare, SequenceCore, SequenceExt, SequenceOrder};
use crate::sequence::Item;

// this size should be below a usize
const MAXIMUM_RANGE_SIZE: i64 = 2_i64.pow(25);

#[derive(Debug, Clone, PartialEq)]
pub struct Many {
    items: Rc<[Item]>,
}

impl Many {}

impl From<Vec<Item>> for Many {
    fn from(items: Vec<Item>) -> Self {
        Many {
            items: items.into(),
        }
    }
}

impl<'a> SequenceCore<'a, std::iter::Cloned<std::slice::Iter<'a, Item>>> for Many {
    #[inline]
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.items.len()
    }

    #[inline]
    fn get(&self, index: usize) -> Option<Item> {
        self.items.get(index).cloned()
    }

    #[inline]
    fn one(self) -> error::Result<Item> {
        Err(error::Error::new(error::ErrorCode::XPTY0004))
    }

    #[inline]
    fn option(self) -> error::Result<Option<Item>> {
        Err(error::Error::new(error::ErrorCode::XPTY0004))
    }

    #[inline]
    fn iter(&'a self) -> std::iter::Cloned<std::slice::Iter<'a, Item>> {
        self.items.iter().cloned()
    }

    #[inline]
    fn effective_boolean_value(&self) -> error::Result<bool> {
        // handle the case where the first item is a node
        // it has to be a singleton otherwise
        if matches!(self.items[0], Item::Node(_)) {
            Ok(true)
        } else {
            Err(error::Error::new(error::ErrorCode::FORG0006))
        }
    }

    #[inline]
    fn string_value(&self, _xot: &xot::Xot) -> error::Result<String> {
        Err(error::Error::new(error::ErrorCode::XPTY0004))
    }
}

impl<'a, I> SequenceExt<'a, I> for Many
where
    I: Iterator<Item = Item> + 'a,
    Many: SequenceCore<'a, I>,
{
}

impl<'a, I> SequenceCompare<'a, I> for Many
where
    I: Iterator<Item = Item> + 'a,
    Many: SequenceCore<'a, I>,
{
}

impl<'a, I> SequenceOrder<'a, I> for Many
where
    I: Iterator<Item = Item>,
    Many: SequenceCore<'a, I>,
{
    fn one_node(&self) -> error::Result<xot::Node> {
        Err(error::Error::new(error::ErrorCode::XPTY0004))
    }
}
