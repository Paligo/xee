use crate::{atomic, error, function};

use super::{Item, Sequence};

// turn a single item into a sequence
impl From<Item> for Sequence {
    fn from(item: Item) -> Self {
        Self::One(item.into())
    }
}

// turn a single node into a sequence
impl From<xot::Node> for Sequence {
    fn from(node: xot::Node) -> Self {
        let item: Item = node.into();
        item.into()
    }
}

// turn a sequence into a single node
impl TryFrom<Sequence> for xot::Node {
    type Error = error::Error;

    fn try_from(sequence: Sequence) -> Result<Self, Self::Error> {
        match sequence {
            Sequence::One(item) => item.item().try_into(),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl TryFrom<Sequence> for function::Function {
    type Error = error::Error;

    fn try_from(sequence: Sequence) -> Result<Self, Self::Error> {
        match sequence {
            Sequence::One(item) => item.item().try_into(),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

// turn a single array into a sequence
impl From<function::Array> for Sequence {
    fn from(array: function::Array) -> Self {
        let item: Item = array.into();
        item.into()
    }
}

// turn a sequence into an array
impl From<Sequence> for function::Array {
    fn from(sequence: Sequence) -> Self {
        let items = sequence
            .iter()
            .map(|item| {
                let sequence: Sequence = item.into();
                sequence
            })
            .collect::<Vec<_>>();

        Self::new(items)
    }
}

// turn a single map into a sequence
impl From<function::Map> for Sequence {
    fn from(map: function::Map) -> Self {
        let item: Item = map.into();
        item.into()
    }
}

// turn an option that can be turned into an item into a sequence
impl<T> From<Option<T>> for Sequence
where
    T: Into<Item>,
{
    fn from(item: Option<T>) -> Self {
        match item {
            Some(item) => {
                let item = item.into();
                Self::One(item.into())
            }
            None => Self::default(),
        }
    }
}

// turn something that can be turned into an atomic into a sequence
impl<T> From<T> for Sequence
where
    T: Into<atomic::Atomic>,
{
    fn from(atomic: T) -> Self {
        let atomic: atomic::Atomic = atomic.into();
        let item: Item = atomic.into();
        Self::One(item.into())
    }
}

impl<T> From<Vec<T>> for Sequence
where
    T: Into<Item>,
{
    fn from(values: Vec<T>) -> Self {
        let mut items = Vec::with_capacity(values.len());
        for value in values {
            let item: Item = value.into();
            items.push(item);
        }
        Self::new(items)
    }
}

// turn an iterator of things that can be turned into items into a sequence
impl FromIterator<Item> for Sequence {
    fn from_iter<I: IntoIterator<Item = Item>>(iter: I) -> Self {
        let items = iter.into_iter().collect::<Vec<_>>();
        items.into()
    }
}

// turn an iterator of item references into a sequence
impl<'a> FromIterator<&'a Item> for Sequence {
    fn from_iter<I: IntoIterator<Item = &'a Item>>(iter: I) -> Self {
        let items = iter.into_iter().cloned().collect::<Vec<_>>();
        items.into()
    }
}

// turn an iterator of atomics into a sequence
impl FromIterator<atomic::Atomic> for Sequence {
    fn from_iter<I: IntoIterator<Item = atomic::Atomic>>(iter: I) -> Self {
        let items = iter.into_iter().map(Item::from).collect::<Vec<_>>();
        items.into()
    }
}

// turn an iterator of nodes into a sequence
impl FromIterator<xot::Node> for Sequence {
    fn from_iter<I: IntoIterator<Item = xot::Node>>(iter: I) -> Self {
        let items = iter.into_iter().map(Item::from).collect::<Vec<_>>();
        items.into()
    }
}
