use crate::sequence::AtomizedItemIter;
use crate::{atomic, error};

use crate::sequence::item::Item;
use crate::sequence::traits::{SequenceCompare, SequenceCore, SequenceExt, SequenceOrder};

// this size should be below a usize
const MAXIMUM_RANGE_SIZE: i64 = 2_i64.pow(25);

#[derive(Debug, Clone, PartialEq)]
pub struct One {
    item: Item,
}

impl One {
    pub(crate) fn item(&self) -> &Item {
        &self.item
    }

    pub(crate) fn into_item(self) -> Item {
        self.item
    }
}

impl From<Item> for One {
    fn from(item: Item) -> Self {
        One { item }
    }
}

impl From<One> for Item {
    fn from(one: One) -> Self {
        one.item
    }
}

impl<'a> SequenceCore<'a, std::iter::Once<Item>> for One {
    #[inline]
    fn is_empty(&self) -> bool {
        false
    }

    #[inline]
    fn len(&self) -> usize {
        1
    }

    #[inline]
    fn get(&self, index: usize) -> Option<Item> {
        if index == 0 {
            Some(self.item.clone())
        } else {
            None
        }
    }

    #[inline]
    fn one(self) -> error::Result<Item> {
        Ok(self.item)
    }

    #[inline]
    fn option(self) -> error::Result<Option<Item>> {
        Ok(Some(self.item))
    }

    #[inline]
    fn iter(&'a self) -> std::iter::Once<Item> {
        std::iter::once(self.item.clone())
    }

    #[inline]
    fn effective_boolean_value(&self) -> error::Result<bool> {
        self.item.effective_boolean_value()
    }

    #[inline]
    fn string_value(&self, xot: &xot::Xot) -> error::Result<String> {
        self.item.string_value(xot)
    }
}

impl<'a, I> SequenceExt<'a, I> for One
where
    I: Iterator<Item = Item> + 'a,
    One: SequenceCore<'a, I>,
{
    fn atomized(
        &'a self,
        xot: &'a xot::Xot,
    ) -> impl Iterator<Item = error::Result<atomic::Atomic>> + 'a {
        AtomizedItemIter::new(&self.item, xot)
    }
}

impl<'a, I> SequenceCompare<'a, I> for One
where
    I: Iterator<Item = Item> + 'a,
    One: SequenceCore<'a, I>,
{
}

impl<'a, I> SequenceOrder<'a, I> for One
where
    I: Iterator<Item = Item>,
    One: SequenceCore<'a, I>,
{
    fn one_node(&self) -> error::Result<xot::Node> {
        match &self.item {
            Item::Node(n) => Ok(*n),
            _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
        }
    }
}
