use ahash::{HashSet, HashSetExt};
use std::rc::Rc;
use xot::Xot;

use crate::{context, error, string::Collation, xml};

use super::{
    core::Sequence,
    item::Item,
    normalization::normalize,
    serialization::{serialize_sequence, SerializationParameters},
    traits::SequenceCore,
    variant::{Empty, Range, RangeIterator},
};

impl Sequence {
    pub(crate) fn new(items: Vec<Item>) -> Self {
        match items.len() {
            0 => Self::Empty(Empty {}),
            1 => Self::One(items.into_iter().next().unwrap().into()),
            _ => Self::Many(items.into()),
        }
    }

    /// Concatenate two sequences producing a new sequence.
    pub fn concat(self, other: Self) -> error::Result<Self> {
        Ok(match (self, other) {
            (Self::Empty(_), Self::Empty(_)) => Self::Empty(Empty {}),
            (Self::Empty(_), Self::One(item)) => Self::One(item),
            (Self::One(item), Self::Empty(_)) => Self::One(item),
            (Self::Empty(_), Self::Many(items)) => Self::Many(items),
            (Self::Many(items), Self::Empty(_)) => Self::Many(items),
            (Self::One(item1), Self::One(item2)) => {
                Self::Many((vec![item1.into_item(), item2.into_item()]).into())
            }
            (Self::One(item), Self::Many(items)) => {
                let mut many = Vec::with_capacity(items.len() + 1);
                many.push(item.into_item());
                for item in items.iter() {
                    many.push(item);
                }
                Self::Many(many.into())
            }
            (Self::Many(items), Self::One(item)) => {
                let mut many = Vec::with_capacity(items.len() + 1);
                for item in items.iter() {
                    many.push(item);
                }
                many.push(item.into_item());
                Self::Many(many.into())
            }
            (Self::Many(items1), Self::Many(items2)) => {
                let mut many = Vec::with_capacity(items1.len() + items2.len());
                for item in items1.iter() {
                    many.push(item);
                }
                for item in items2.iter() {
                    many.push(item);
                }
                Self::Many(many.into())
            }
            (Self::Range(a), Self::Range(b)) => {
                // if the ranges are consecutive we can merge them
                if a.end() == b.start() {
                    Self::Range(Range::new(a.start().clone(), b.end().clone())?)
                } else if b.end() == a.start() {
                    Self::Range(Range::new(b.start().clone(), a.end().clone())?)
                } else {
                    // otherwise unfortunately we have to construct the sequence
                    let mut v = Vec::with_capacity(a.len() + b.len());
                    for i in RangeIterator::new(a.start().clone(), a.end().clone()) {
                        v.push(i);
                    }
                    for i in RangeIterator::new(b.start().clone(), b.end().clone()) {
                        v.push(i);
                    }
                    Self::new(v)
                }
            }
            // handle other cases in less efficient way
            (a, b) => {
                let mut v = Vec::with_capacity(a.len() + b.len());
                for item in a.iter() {
                    v.push(item);
                }
                for item in b.iter() {
                    v.push(item);
                }
                Self::new(v)
            }
        })
    }

    // https://www.w3.org/TR/xpath-31/#id-path-operator
    pub(crate) fn deduplicate(self, annotations: xml::DocumentOrderAccess) -> error::Result<Self> {
        let mut s = HashSet::new();
        let mut non_node_seen = false;

        for item in self.iter() {
            match item {
                Item::Node(n) => {
                    if non_node_seen {
                        return Err(error::Error::XPTY0004);
                    }
                    s.insert(n);
                }
                _ => {
                    if !s.is_empty() {
                        return Err(error::Error::XPTY0004);
                    }
                    non_node_seen = true;
                }
            }
        }
        if non_node_seen {
            Ok(self)
        } else {
            Ok(Self::process_set_result(s, annotations))
        }
    }

    pub(crate) fn process_set_result(
        s: HashSet<xot::Node>,
        annotations: xml::DocumentOrderAccess,
    ) -> Self {
        // sort nodes by document order
        let mut nodes = s.into_iter().collect::<Vec<_>>();
        nodes.sort_by_key(|n| annotations.get(*n));
        nodes.into()
    }

    pub fn sorted(
        &self,
        context: &context::DynamicContext,
        collation: Rc<Collation>,
        xot: &Xot,
    ) -> error::Result<Self> {
        self.sorted_by_key(context, collation, |item| {
            // the equivalent of fn:data()
            let seq: Self = item.into();
            seq.atomized(xot).collect::<error::Result<Sequence>>()
        })
    }

    pub fn sorted_by_key<F>(
        &self,
        context: &context::DynamicContext,
        collation: Rc<Collation>,
        get: F,
    ) -> error::Result<Self>
    where
        F: FnMut(Item) -> error::Result<Sequence>,
    {
        // see also sort_by_sequence in array.rs. The signatures are
        // sufficiently different we don't want to try to unify them.

        let items = self.iter().collect::<Vec<_>>();
        let keys = self.iter().map(get).collect::<error::Result<Vec<_>>>()?;

        let mut keys_and_items = keys.into_iter().zip(items).collect::<Vec<_>>();
        // sort by key. unfortunately sort_by requires the compare function
        // to be infallible. It's not in reality, so we make any failures
        // sort less, so they appear early on in the sequence.
        keys_and_items.sort_by(|(a_key, _), (b_key, _)| {
            a_key.compare(b_key, &collation, context.implicit_timezone())
        });
        // a pass to detect any errors; if sorting between two items is
        // impossible we want to raise a type error
        for ((a_key, _), (b_key, _)) in keys_and_items.iter().zip(keys_and_items.iter().skip(1)) {
            a_key.fallible_compare(b_key, &collation, context.implicit_timezone())?;
        }
        // now pick up items again
        let result = keys_and_items
            .into_iter()
            .map(|(_, item)| item)
            .collect::<Sequence>();
        Ok(result)
    }

    /// Flatten all arrays in this sequence
    pub fn flatten(&self) -> error::Result<Self> {
        let mut result = vec![];
        for item in self.iter() {
            if let Ok(array) = item.to_array() {
                for sequence in array.iter() {
                    for item in sequence.flatten()?.iter() {
                        result.push(item);
                    }
                }
            } else {
                result.push(item);
            }
        }
        Ok(result.into())
    }

    pub(crate) fn union(
        self,
        other: Self,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<Self> {
        let mut s = HashSet::new();
        for node in self.nodes() {
            s.insert(node?);
        }
        for node in other.nodes() {
            s.insert(node?);
        }

        Ok(Self::process_set_result(s, annotations))
    }

    pub(crate) fn intersect(
        self,
        other: Self,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<Self> {
        let mut s = HashSet::new();
        let mut r = HashSet::new();
        for node in self.nodes() {
            s.insert(node?);
        }
        for node in other.nodes() {
            let node = node?;
            if s.contains(&node) {
                r.insert(node);
            }
        }
        Ok(Self::process_set_result(r, annotations))
    }

    pub(crate) fn except(
        self,
        other: Self,
        annotations: xml::DocumentOrderAccess,
    ) -> error::Result<Self> {
        let mut s = HashSet::new();
        for node in self.nodes() {
            s.insert(node?);
        }
        for node in other.nodes() {
            s.remove(&node?);
        }
        Ok(Self::process_set_result(s, annotations))
    }

    /// Normalize this sequence into a document node, according to
    /// <https://www.w3.org/TR/xslt-xquery-serialization-31/#serdm>
    pub fn normalize(&self, item_separator: &str, xot: &mut Xot) -> error::Result<xot::Node> {
        normalize(self, item_separator, xot)
    }

    /// Serialize this sequence according to serialization parameters
    pub(crate) fn serialize(
        &self,
        params: SerializationParameters,
        xot: &mut Xot,
    ) -> error::Result<String> {
        serialize_sequence(self, params, xot)
    }

    /// Display representation of the sequence
    pub fn display_representation(&self, xot: &Xot, context: &context::DynamicContext) -> String {
        // TODO: various unwraps
        match &self {
            Sequence::Empty(_) => "()".to_string(),
            Sequence::One(item) => item.item().display_representation(xot, context).unwrap(),
            Sequence::Many(items) => {
                let mut representations = Vec::with_capacity(self.len());
                for item in items.iter() {
                    representations.push(item.display_representation(xot, context).unwrap());
                }
                format!("(\n{}\n)", representations.join(",\n"))
            }
            Sequence::Range(range) => {
                format!("{} to {}", range.start(), range.end())
            }
        }
    }
}
