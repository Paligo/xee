// this is unfortunately a ridiculously verbose module, wiring everything up
// carefully so we don't use performance due to dynamic dispatch on the inside.
// The verbosity is all pretty straightforward though.

// creation.rs contains various functions that create Sequence
// compare.rs contains various comparison functions

use xot::Xot;

use crate::{
    atomic::{self, AtomicCompare},
    context, error, function,
    string::Collation,
    xml,
};

use super::{
    item::Item,
    traits::{BoxedItemIter, SequenceCompare, SequenceCore, SequenceExt, SequenceOrder},
    variant::{Empty, Many, One, Range},
};

// The Sequence that goes onto the stack is the size of an single item, as
// that's the biggest thing in it.
#[derive(Debug, Clone, PartialEq)]
pub enum Sequence {
    Empty(Empty),
    One(One),
    Many(Many),
    Range(Range),
}

impl From<Range> for Sequence {
    fn from(inner: Range) -> Self {
        Self::Range(inner)
    }
}

// a static assertion to ensure that Sequence never grows in size
#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(Sequence, [u8; 24]);

impl Default for Sequence {
    fn default() -> Self {
        Self::Empty(Empty {})
    }
}

impl Sequence {
    /// Check whether the sequence is empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Empty(inner) => inner.is_empty(),
            Self::One(inner) => inner.is_empty(),
            Self::Many(inner) => inner.is_empty(),
            Self::Range(inner) => inner.is_empty(),
        }
    }

    /// Get the sequence length
    pub fn len(&self) -> usize {
        match self {
            Self::Empty(inner) => inner.len(),
            Self::One(inner) => inner.len(),
            Self::Many(inner) => inner.len(),
            Self::Range(inner) => inner.len(),
        }
    }

    /// Get an item in the index, if it exists
    pub fn get(&self, index: usize) -> Option<Item> {
        match self {
            Self::Empty(inner) => inner.get(index),
            Self::One(inner) => inner.get(index),
            Self::Many(inner) => inner.get(index),
            Self::Range(inner) => inner.get(index),
        }
    }

    /// Get a single item from the sequence, if it only contains one item
    ///
    /// Otherwise you get a type error.
    pub fn one(self) -> error::Result<Item> {
        match self {
            Self::Empty(inner) => inner.one(),
            Self::One(inner) => inner.one(),
            Self::Many(inner) => inner.one(),
            Self::Range(inner) => inner.one(),
        }
    }

    /// Get a optional item from the sequence
    ///
    /// If it contains more than one item, you get a type error.
    pub fn option(self) -> error::Result<Option<Item>> {
        match self {
            Self::Empty(inner) => inner.option(),
            Self::One(inner) => inner.option(),
            Self::Many(inner) => inner.option(),
            Self::Range(inner) => inner.option(),
        }
    }

    /// Get the items from the sequence as an iterator
    pub fn iter(&self) -> BoxedItemIter {
        match self {
            Self::Empty(inner) => Box::new(inner.iter()),
            Self::One(inner) => Box::new(inner.iter()),
            Self::Many(inner) => Box::new(inner.iter()),
            Self::Range(inner) => Box::new(inner.iter()),
        }
    }

    /// Effective boolean value
    pub fn effective_boolean_value(&self) -> error::Result<bool> {
        match self {
            Self::Empty(inner) => inner.effective_boolean_value(),
            Self::One(inner) => inner.effective_boolean_value(),
            Self::Many(inner) => inner.effective_boolean_value(),
            Self::Range(inner) => inner.effective_boolean_value(),
        }
    }

    /// String value
    pub fn string_value(&self, xot: &xot::Xot) -> error::Result<String> {
        match self {
            Self::Empty(inner) => inner.string_value(xot),
            Self::One(inner) => inner.string_value(xot),
            Self::Many(inner) => inner.string_value(xot),
            Self::Range(inner) => inner.string_value(xot),
        }
    }

    /// Iterator over the nodes in the sequence
    ///
    /// An error is returned for items that are not a node.
    pub fn nodes<'a>(&'a self) -> Box<dyn Iterator<Item = error::Result<xot::Node>> + 'a> {
        match self {
            Self::Empty(inner) => Box::new(inner.nodes()),
            Self::One(inner) => Box::new(inner.nodes()),
            Self::Many(inner) => Box::new(inner.nodes()),
            Self::Range(inner) => Box::new(inner.nodes()),
        }
    }

    /// Iterator for the atomized values in the sequence
    pub fn atomized<'a>(
        &'a self,
        xot: &'a xot::Xot,
    ) -> Box<dyn Iterator<Item = error::Result<atomic::Atomic>> + 'a> {
        match self {
            Self::Empty(inner) => Box::new(inner.atomized(xot)),
            Self::One(inner) => Box::new(inner.atomized(xot)),
            Self::Many(inner) => Box::new(inner.atomized(xot)),
            Self::Range(inner) => Box::new(inner.atomized(xot)),
        }
    }

    /// Get just one atomized value from the sequence
    ///
    /// If there are less or more, you get a type error.
    pub fn atomized_one(&self, xot: &xot::Xot) -> error::Result<atomic::Atomic> {
        match self {
            Self::Empty(inner) => inner.atomized_one(xot),
            Self::One(inner) => inner.atomized_one(xot),
            Self::Many(inner) => inner.atomized_one(xot),
            Self::Range(inner) => inner.atomized_one(xot),
        }
    }

    /// Get an optional atomized value from the sequence
    ///
    /// If there are more than one, you get a type error.
    pub fn atomized_option(&self, xot: &xot::Xot) -> error::Result<Option<atomic::Atomic>> {
        match self {
            Self::Empty(inner) => inner.atomized_option(xot),
            Self::One(inner) => inner.atomized_option(xot),
            Self::Many(inner) => inner.atomized_option(xot),
            Self::Range(inner) => inner.atomized_option(xot),
        }
    }

    /// Is used internally by the library macro.
    pub(crate) fn unboxed_atomized<'a, T: 'a>(
        &'a self,
        xot: &'a xot::Xot,
        extract: impl Fn(atomic::Atomic) -> error::Result<T> + 'a,
    ) -> Box<dyn Iterator<Item = error::Result<T>> + 'a> {
        match self {
            Self::Empty(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Self::One(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Self::Many(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Self::Range(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
        }
    }

    /// Iterator over the XPath maps in the sequence
    ///
    /// An error is returned for items that are not a map.
    pub fn map_iter<'a>(&'a self) -> Box<dyn Iterator<Item = error::Result<function::Map>> + 'a> {
        match self {
            Self::Empty(inner) => Box::new(inner.map_iter()),
            Self::One(inner) => Box::new(inner.map_iter()),
            Self::Many(inner) => Box::new(inner.map_iter()),
            Self::Range(inner) => Box::new(inner.map_iter()),
        }
    }

    /// Iterator over the XPath arrays in the sequence
    ///
    /// An error is returned for items that are not an array.
    pub fn array_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = error::Result<function::Array>> + 'a> {
        match self {
            Self::Empty(inner) => Box::new(inner.array_iter()),
            Self::One(inner) => Box::new(inner.array_iter()),
            Self::Many(inner) => Box::new(inner.array_iter()),
            Self::Range(inner) => Box::new(inner.array_iter()),
        }
    }

    /// Oterator over elements nodes in the sequence
    ///
    /// An error is returned for items that are not an element.
    pub fn elements<'a>(
        &'a self,
        xot: &'a xot::Xot,
    ) -> error::Result<Box<dyn Iterator<Item = error::Result<xot::Node>> + 'a>> {
        match self {
            Self::Empty(inner) => Ok(Box::new(inner.elements(xot)?)),
            Self::One(inner) => Ok(Box::new(inner.elements(xot)?)),
            Self::Many(inner) => Ok(Box::new(inner.elements(xot)?)),
            Self::Range(inner) => Ok(Box::new(inner.elements(xot)?)),
        }
    }

    /// Create an XPath array from this sequence.
    pub fn to_array(&self) -> error::Result<function::Array> {
        match self {
            Self::Empty(inner) => inner.to_array(),
            Self::One(inner) => inner.to_array(),
            Self::Many(inner) => inner.to_array(),
            Self::Range(inner) => inner.to_array(),
        }
    }

    pub(crate) fn general_comparison<O>(
        &self,
        other: &Self,
        op: O,
        context: &context::DynamicContext,
        xot: &xot::Xot,
    ) -> error::Result<bool>
    where
        O: AtomicCompare,
    {
        match (self, other) {
            (Self::Empty(_a), Self::Empty(_b)) => Ok(false),
            (Self::Empty(_a), Self::One(_b)) => Ok(false),
            (Self::Empty(_a), Self::Many(_b)) => Ok(false),
            (Self::Empty(_a), Self::Range(_b)) => Ok(false),
            (Self::One(_a), Self::Empty(_b)) => Ok(false),
            (Self::One(a), Self::One(b)) => a.general_comparison(b, op, context, xot),
            (Self::One(a), Self::Many(b)) => a.general_comparison(b, op, context, xot),
            (Self::One(a), Self::Range(b)) => {
                if let Item::Atomic(atomic::Atomic::Integer(_, i)) = a.item() {
                    Ok(b.general_comparison_integer(i, O::value()))
                } else {
                    a.general_comparison(b, op, context, xot)
                }
            }
            (Self::Many(_a), Self::Empty(_b)) => Ok(false),
            (Self::Many(a), Self::One(b)) => a.general_comparison(b, op, context, xot),
            (Self::Many(a), Self::Many(b)) => a.general_comparison(b, op, context, xot),
            (Self::Many(a), Self::Range(b)) => a.general_comparison(b, op, context, xot),
            (Self::Range(_a), Self::Empty(_b)) => Ok(false),
            (Self::Range(a), Self::One(b)) => a.general_comparison(b, op, context, xot),
            (Self::Range(a), Self::Many(b)) => a.general_comparison(b, op, context, xot),
            (Self::Range(a), Self::Range(b)) => a.general_comparison(b, op, context, xot),
        }
    }

    pub(crate) fn value_compare<O>(
        &self,
        other: &Self,
        op: O,
        collation: &Collation,
        timezone: chrono::FixedOffset,
        xot: &Xot,
    ) -> error::Result<bool>
    where
        O: AtomicCompare,
    {
        match (self, other) {
            (Self::Empty(a), Self::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Empty(a), Self::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Empty(a), Self::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Empty(a), Self::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::One(a), Self::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::One(a), Self::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::One(a), Self::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::One(a), Self::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Many(a), Self::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Many(a), Self::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Many(a), Self::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Many(a), Self::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Range(a), Self::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Range(a), Self::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Range(a), Self::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Self::Range(a), Self::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
        }
    }

    pub(crate) fn one_node(&self) -> error::Result<xot::Node> {
        match self {
            Self::Empty(inner) => inner.one_node(),
            Self::One(inner) => inner.one_node(),
            Self::Many(inner) => inner.one_node(),
            Self::Range(inner) => inner.one_node(),
        }
    }

    pub(crate) fn is(&self, other: &Self) -> error::Result<bool> {
        match (self, other) {
            (Self::Empty(a), Self::Empty(b)) => a.is(b),
            (Self::Empty(a), Self::One(b)) => a.is(b),
            (Self::Empty(a), Self::Many(b)) => a.is(b),
            (Self::Empty(a), Self::Range(b)) => a.is(b),
            (Self::One(a), Self::Empty(b)) => a.is(b),
            (Self::One(a), Self::One(b)) => a.is(b),
            (Self::One(a), Self::Many(b)) => a.is(b),
            (Self::One(a), Self::Range(b)) => a.is(b),
            (Self::Many(a), Self::Empty(b)) => a.is(b),
            (Self::Many(a), Self::One(b)) => a.is(b),
            (Self::Many(a), Self::Many(b)) => a.is(b),
            (Self::Many(a), Self::Range(b)) => a.is(b),
            (Self::Range(a), Self::Empty(b)) => a.is(b),
            (Self::Range(a), Self::One(b)) => a.is(b),
            (Self::Range(a), Self::Many(b)) => a.is(b),
            (Self::Range(a), Self::Range(b)) => a.is(b),
        }
    }

    pub(crate) fn precedes(
        &self,
        other: &Self,
        annotations: &xml::Annotations,
    ) -> error::Result<bool> {
        match (self, other) {
            (Self::Empty(a), Self::Empty(b)) => a.precedes(b, annotations),
            (Self::Empty(a), Self::One(b)) => a.precedes(b, annotations),
            (Self::Empty(a), Self::Many(b)) => a.precedes(b, annotations),
            (Self::Empty(a), Self::Range(b)) => a.precedes(b, annotations),
            (Self::One(a), Self::Empty(b)) => a.precedes(b, annotations),
            (Self::One(a), Self::One(b)) => a.precedes(b, annotations),
            (Self::One(a), Self::Many(b)) => a.precedes(b, annotations),
            (Self::One(a), Self::Range(b)) => a.precedes(b, annotations),
            (Self::Many(a), Self::Empty(b)) => a.precedes(b, annotations),
            (Self::Many(a), Self::One(b)) => a.precedes(b, annotations),
            (Self::Many(a), Self::Many(b)) => a.precedes(b, annotations),
            (Self::Many(a), Self::Range(b)) => a.precedes(b, annotations),
            (Self::Range(a), Self::Empty(b)) => a.precedes(b, annotations),
            (Self::Range(a), Self::One(b)) => a.precedes(b, annotations),
            (Self::Range(a), Self::Many(b)) => a.precedes(b, annotations),
            (Self::Range(a), Self::Range(b)) => a.precedes(b, annotations),
        }
    }

    pub(crate) fn follows(
        &self,
        other: &Self,
        annotations: &xml::Annotations,
    ) -> error::Result<bool> {
        match (self, other) {
            (Self::Empty(a), Self::Empty(b)) => a.follows(b, annotations),
            (Self::Empty(a), Self::One(b)) => a.follows(b, annotations),
            (Self::Empty(a), Self::Many(b)) => a.follows(b, annotations),
            (Self::Empty(a), Self::Range(b)) => a.follows(b, annotations),
            (Self::One(a), Self::Empty(b)) => a.follows(b, annotations),
            (Self::One(a), Self::One(b)) => a.follows(b, annotations),
            (Self::One(a), Self::Many(b)) => a.follows(b, annotations),
            (Self::One(a), Self::Range(b)) => a.follows(b, annotations),
            (Self::Many(a), Self::Empty(b)) => a.follows(b, annotations),
            (Self::Many(a), Self::One(b)) => a.follows(b, annotations),
            (Self::Many(a), Self::Many(b)) => a.follows(b, annotations),
            (Self::Many(a), Self::Range(b)) => a.follows(b, annotations),
            (Self::Range(a), Self::Empty(b)) => a.follows(b, annotations),
            (Self::Range(a), Self::One(b)) => a.follows(b, annotations),
            (Self::Range(a), Self::Many(b)) => a.follows(b, annotations),
            (Self::Range(a), Self::Range(b)) => a.follows(b, annotations),
        }
    }
}
