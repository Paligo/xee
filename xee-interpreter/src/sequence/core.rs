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
            Sequence::Empty(inner) => inner.is_empty(),
            Sequence::One(inner) => inner.is_empty(),
            Sequence::Many(inner) => inner.is_empty(),
            Sequence::Range(inner) => inner.is_empty(),
        }
    }

    /// Get the sequence length
    pub fn len(&self) -> usize {
        match self {
            Sequence::Empty(inner) => inner.len(),
            Sequence::One(inner) => inner.len(),
            Sequence::Many(inner) => inner.len(),
            Sequence::Range(inner) => inner.len(),
        }
    }

    /// Get an item in the index, if it exists
    pub fn get(&self, index: usize) -> Option<Item> {
        match self {
            Sequence::Empty(inner) => inner.get(index),
            Sequence::One(inner) => inner.get(index),
            Sequence::Many(inner) => inner.get(index),
            Sequence::Range(inner) => inner.get(index),
        }
    }

    /// Get a single item from the sequence, if it only contains one item
    ///
    /// Otherwise you get a type error.
    pub fn one(self) -> error::Result<Item> {
        match self {
            Sequence::Empty(inner) => inner.one(),
            Sequence::One(inner) => inner.one(),
            Sequence::Many(inner) => inner.one(),
            Sequence::Range(inner) => inner.one(),
        }
    }

    /// Get a optional item from the sequence
    ///
    /// If it contains more than one item, you get a type error.
    pub fn option(self) -> error::Result<Option<Item>> {
        match self {
            Sequence::Empty(inner) => inner.option(),
            Sequence::One(inner) => inner.option(),
            Sequence::Many(inner) => inner.option(),
            Sequence::Range(inner) => inner.option(),
        }
    }

    /// Get the items from the sequence as an iterator
    pub fn iter(&self) -> BoxedItemIter {
        match self {
            Sequence::Empty(inner) => Box::new(inner.iter()),
            Sequence::One(inner) => Box::new(inner.iter()),
            Sequence::Many(inner) => Box::new(inner.iter()),
            Sequence::Range(inner) => Box::new(inner.iter()),
        }
    }

    /// Effective boolean value
    pub fn effective_boolean_value(&self) -> error::Result<bool> {
        match self {
            Sequence::Empty(inner) => inner.effective_boolean_value(),
            Sequence::One(inner) => inner.effective_boolean_value(),
            Sequence::Many(inner) => inner.effective_boolean_value(),
            Sequence::Range(inner) => inner.effective_boolean_value(),
        }
    }

    /// String value
    pub fn string_value(&self, xot: &xot::Xot) -> error::Result<String> {
        match self {
            Sequence::Empty(inner) => inner.string_value(xot),
            Sequence::One(inner) => inner.string_value(xot),
            Sequence::Many(inner) => inner.string_value(xot),
            Sequence::Range(inner) => inner.string_value(xot),
        }
    }

    /// Iterator over the nodes in the sequence
    ///
    /// An error is returned for items that are not a node.
    pub fn nodes<'a>(&'a self) -> Box<dyn Iterator<Item = error::Result<xot::Node>> + 'a> {
        match self {
            Sequence::Empty(inner) => Box::new(inner.nodes()),
            Sequence::One(inner) => Box::new(inner.nodes()),
            Sequence::Many(inner) => Box::new(inner.nodes()),
            Sequence::Range(inner) => Box::new(inner.nodes()),
        }
    }

    /// Iterator for the atomized values in the sequence
    pub fn atomized<'a>(
        &'a self,
        xot: &'a xot::Xot,
    ) -> Box<dyn Iterator<Item = error::Result<atomic::Atomic>> + 'a> {
        match self {
            Sequence::Empty(inner) => Box::new(inner.atomized(xot)),
            Sequence::One(inner) => Box::new(inner.atomized(xot)),
            Sequence::Many(inner) => Box::new(inner.atomized(xot)),
            Sequence::Range(inner) => Box::new(inner.atomized(xot)),
        }
    }

    /// Get just one atomized value from the sequence
    ///
    /// If there are less or more, you get a type error.
    pub fn atomized_one(&self, xot: &xot::Xot) -> error::Result<atomic::Atomic> {
        match self {
            Sequence::Empty(inner) => inner.atomized_one(xot),
            Sequence::One(inner) => inner.atomized_one(xot),
            Sequence::Many(inner) => inner.atomized_one(xot),
            Sequence::Range(inner) => inner.atomized_one(xot),
        }
    }

    /// Get an optional atomized value from the sequence
    ///
    /// If there are more than one, you get a type error.
    pub fn atomized_option(&self, xot: &xot::Xot) -> error::Result<Option<atomic::Atomic>> {
        match self {
            Sequence::Empty(inner) => inner.atomized_option(xot),
            Sequence::One(inner) => inner.atomized_option(xot),
            Sequence::Many(inner) => inner.atomized_option(xot),
            Sequence::Range(inner) => inner.atomized_option(xot),
        }
    }

    /// Is used internally by the library macro.
    pub(crate) fn unboxed_atomized<'a, T: 'a>(
        &'a self,
        xot: &'a xot::Xot,
        extract: impl Fn(atomic::Atomic) -> error::Result<T> + 'a,
    ) -> Box<dyn Iterator<Item = error::Result<T>> + 'a> {
        match self {
            Sequence::Empty(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Sequence::One(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Sequence::Many(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
            Sequence::Range(inner) => Box::new(inner.unboxed_atomized(xot, extract)),
        }
    }

    /// Iterator over the XPath maps in the sequence
    ///
    /// An error is returned for items that are not a map.
    pub fn map_iter<'a>(&'a self) -> Box<dyn Iterator<Item = error::Result<function::Map>> + 'a> {
        match self {
            Sequence::Empty(inner) => Box::new(inner.map_iter()),
            Sequence::One(inner) => Box::new(inner.map_iter()),
            Sequence::Many(inner) => Box::new(inner.map_iter()),
            Sequence::Range(inner) => Box::new(inner.map_iter()),
        }
    }

    /// Iterator over the XPath arrays in the sequence
    ///
    /// An error is returned for items that are not an array.
    pub fn array_iter<'a>(
        &'a self,
    ) -> Box<dyn Iterator<Item = error::Result<function::Array>> + 'a> {
        match self {
            Sequence::Empty(inner) => Box::new(inner.array_iter()),
            Sequence::One(inner) => Box::new(inner.array_iter()),
            Sequence::Many(inner) => Box::new(inner.array_iter()),
            Sequence::Range(inner) => Box::new(inner.array_iter()),
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
            Sequence::Empty(inner) => Ok(Box::new(inner.elements(xot)?)),
            Sequence::One(inner) => Ok(Box::new(inner.elements(xot)?)),
            Sequence::Many(inner) => Ok(Box::new(inner.elements(xot)?)),
            Sequence::Range(inner) => Ok(Box::new(inner.elements(xot)?)),
        }
    }

    /// Create an XPath array from this sequence.
    pub fn to_array(&self) -> error::Result<function::Array> {
        match self {
            Sequence::Empty(inner) => inner.to_array(),
            Sequence::One(inner) => inner.to_array(),
            Sequence::Many(inner) => inner.to_array(),
            Sequence::Range(inner) => inner.to_array(),
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
            (Sequence::Empty(_a), Sequence::Empty(_b)) => Ok(false),
            (Sequence::Empty(_a), Sequence::One(_b)) => Ok(false),
            (Sequence::Empty(_a), Sequence::Many(_b)) => Ok(false),
            (Sequence::Empty(_a), Sequence::Range(_b)) => Ok(false),
            (Sequence::One(_a), Sequence::Empty(_b)) => Ok(false),
            (Sequence::One(a), Sequence::One(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::One(a), Sequence::Many(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::One(a), Sequence::Range(b)) => {
                if let Item::Atomic(atomic::Atomic::Integer(_, i)) = a.item() {
                    Ok(b.general_comparison_integer(i, O::value()))
                } else {
                    a.general_comparison(b, op, context, xot)
                }
            }
            (Sequence::Many(_a), Sequence::Empty(_b)) => Ok(false),
            (Sequence::Many(a), Sequence::One(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::Many(a), Sequence::Many(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::Many(a), Sequence::Range(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::Range(_a), Sequence::Empty(_b)) => Ok(false),
            (Sequence::Range(a), Sequence::One(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::Range(a), Sequence::Many(b)) => a.general_comparison(b, op, context, xot),
            (Sequence::Range(a), Sequence::Range(b)) => a.general_comparison(b, op, context, xot),
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
            (Sequence::Empty(a), Sequence::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Empty(a), Sequence::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Empty(a), Sequence::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Empty(a), Sequence::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::One(a), Sequence::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::One(a), Sequence::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::One(a), Sequence::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::One(a), Sequence::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Many(a), Sequence::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Many(a), Sequence::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Many(a), Sequence::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Many(a), Sequence::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Range(a), Sequence::Empty(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Range(a), Sequence::One(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Range(a), Sequence::Many(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
            (Sequence::Range(a), Sequence::Range(b)) => {
                a.value_compare(b, op, collation, timezone, xot)
            }
        }
    }

    pub(crate) fn one_node(&self) -> error::Result<xot::Node> {
        match self {
            Sequence::Empty(inner) => inner.one_node(),
            Sequence::One(inner) => inner.one_node(),
            Sequence::Many(inner) => inner.one_node(),
            Sequence::Range(inner) => inner.one_node(),
        }
    }

    pub(crate) fn is(&self, other: &Self) -> error::Result<bool> {
        match (self, other) {
            (Sequence::Empty(a), Sequence::Empty(b)) => a.is(b),
            (Sequence::Empty(a), Sequence::One(b)) => a.is(b),
            (Sequence::Empty(a), Sequence::Many(b)) => a.is(b),
            (Sequence::Empty(a), Sequence::Range(b)) => a.is(b),
            (Sequence::One(a), Sequence::Empty(b)) => a.is(b),
            (Sequence::One(a), Sequence::One(b)) => a.is(b),
            (Sequence::One(a), Sequence::Many(b)) => a.is(b),
            (Sequence::One(a), Sequence::Range(b)) => a.is(b),
            (Sequence::Many(a), Sequence::Empty(b)) => a.is(b),
            (Sequence::Many(a), Sequence::One(b)) => a.is(b),
            (Sequence::Many(a), Sequence::Many(b)) => a.is(b),
            (Sequence::Many(a), Sequence::Range(b)) => a.is(b),
            (Sequence::Range(a), Sequence::Empty(b)) => a.is(b),
            (Sequence::Range(a), Sequence::One(b)) => a.is(b),
            (Sequence::Range(a), Sequence::Many(b)) => a.is(b),
            (Sequence::Range(a), Sequence::Range(b)) => a.is(b),
        }
    }

    pub(crate) fn precedes(
        &self,
        other: &Self,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<bool> {
        match (self, other) {
            (Sequence::Empty(a), Sequence::Empty(b)) => a.precedes(b, annotations),
            (Sequence::Empty(a), Sequence::One(b)) => a.precedes(b, annotations),
            (Sequence::Empty(a), Sequence::Many(b)) => a.precedes(b, annotations),
            (Sequence::Empty(a), Sequence::Range(b)) => a.precedes(b, annotations),
            (Sequence::One(a), Sequence::Empty(b)) => a.precedes(b, annotations),
            (Sequence::One(a), Sequence::One(b)) => a.precedes(b, annotations),
            (Sequence::One(a), Sequence::Many(b)) => a.precedes(b, annotations),
            (Sequence::One(a), Sequence::Range(b)) => a.precedes(b, annotations),
            (Sequence::Many(a), Sequence::Empty(b)) => a.precedes(b, annotations),
            (Sequence::Many(a), Sequence::One(b)) => a.precedes(b, annotations),
            (Sequence::Many(a), Sequence::Many(b)) => a.precedes(b, annotations),
            (Sequence::Many(a), Sequence::Range(b)) => a.precedes(b, annotations),
            (Sequence::Range(a), Sequence::Empty(b)) => a.precedes(b, annotations),
            (Sequence::Range(a), Sequence::One(b)) => a.precedes(b, annotations),
            (Sequence::Range(a), Sequence::Many(b)) => a.precedes(b, annotations),
            (Sequence::Range(a), Sequence::Range(b)) => a.precedes(b, annotations),
        }
    }

    pub(crate) fn follows(
        &self,
        other: &Self,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<bool> {
        match (self, other) {
            (Sequence::Empty(a), Sequence::Empty(b)) => a.follows(b, annotations),
            (Sequence::Empty(a), Sequence::One(b)) => a.follows(b, annotations),
            (Sequence::Empty(a), Sequence::Many(b)) => a.follows(b, annotations),
            (Sequence::Empty(a), Sequence::Range(b)) => a.follows(b, annotations),
            (Sequence::One(a), Sequence::Empty(b)) => a.follows(b, annotations),
            (Sequence::One(a), Sequence::One(b)) => a.follows(b, annotations),
            (Sequence::One(a), Sequence::Many(b)) => a.follows(b, annotations),
            (Sequence::One(a), Sequence::Range(b)) => a.follows(b, annotations),
            (Sequence::Many(a), Sequence::Empty(b)) => a.follows(b, annotations),
            (Sequence::Many(a), Sequence::One(b)) => a.follows(b, annotations),
            (Sequence::Many(a), Sequence::Many(b)) => a.follows(b, annotations),
            (Sequence::Many(a), Sequence::Range(b)) => a.follows(b, annotations),
            (Sequence::Range(a), Sequence::Empty(b)) => a.follows(b, annotations),
            (Sequence::Range(a), Sequence::One(b)) => a.follows(b, annotations),
            (Sequence::Range(a), Sequence::Many(b)) => a.follows(b, annotations),
            (Sequence::Range(a), Sequence::Range(b)) => a.follows(b, annotations),
        }
    }
}
