// the stack::Value abstraction is a sequence partitioned into special cases:
// empty sequence, sequence with a single item, and sequence with multiple
// items. This partitioning makes it easier to optimize various common cases
// and keeps the code cleaner.
use std::rc::Rc;

use ahash::{HashSet, HashSetExt};
use xot::Xot;

use crate::atomic;
use crate::atomic::AtomicCompare;
use crate::context;
use crate::error;
use crate::function;
use crate::sequence;
use crate::stack;
use crate::xml;

use super::comparison;

#[derive(Debug, Clone)]
pub enum Value {
    Empty,
    One(sequence::Item),
    Many(Rc<Vec<sequence::Item>>),
    Absent,
}

impl Value {
    pub(crate) fn len(self) -> error::Result<usize> {
        match self {
            Value::Empty => Ok(0),
            Value::One(_) => Ok(1),
            Value::Many(items) => Ok(items.len()),
            Value::Absent => Err(error::Error::XPDY0002),
        }
    }

    pub(crate) fn index(self, index: usize) -> error::Result<sequence::Item> {
        match self {
            Value::Empty => Err(error::Error::XPTY0004),
            Value::One(item) => {
                if index == 0 {
                    Ok(item)
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
            Value::Many(items) => items
                .get(index)
                .ok_or(error::Error::XPTY0004)
                .map(|item| item.clone()),
            Value::Absent => Err(error::Error::XPDY0002),
        }
    }
    pub(crate) fn items(&self) -> ValueIter {
        ValueIter::new(self.clone())
    }

    pub(crate) fn atomized<'a>(&self, xot: &'a Xot) -> AtomizedIter<'a> {
        AtomizedIter::new(self.clone(), xot)
    }

    pub(crate) fn effective_boolean_value(&self) -> error::Result<bool> {
        match self {
            Value::Empty => Ok(false),
            Value::One(item) => item.effective_boolean_value(),
            Value::Many(items) => {
                // handle the case where the first item is a node
                // it has to be a singleton otherwise
                if matches!(items[0], sequence::Item::Node(_)) {
                    Ok(true)
                } else {
                    Err(error::Error::XPTY0004)
                }
            }
            Value::Absent => Err(error::Error::XPDY0002),
        }
    }

    pub(crate) fn is_empty_sequence(&self) -> bool {
        matches!(self, Value::Empty)
    }

    pub(crate) fn string_value(&self, xot: &Xot) -> error::Result<String> {
        match self {
            Value::Empty => Ok("".to_string()),
            Value::One(item) => item.string_value(xot),
            Value::Many(_) => Err(error::Error::XPTY0004),
            Value::Absent => Err(error::Error::XPDY0002),
        }
    }

    pub(crate) fn general_comparison<O>(
        &self,
        other: Value,
        context: &context::DynamicContext,
        op: O,
    ) -> error::Result<bool>
    where
        O: AtomicCompare,
    {
        comparison::general_comparison(
            self.atomized(context.xot()),
            other.atomized(context.xot()),
            context,
            op,
        )
    }

    pub(crate) fn concat(self, other: stack::Value) -> stack::Value {
        match (self, other) {
            (Value::Empty, Value::Empty) => Value::Empty,
            (Value::Empty, Value::One(item)) => Value::One(item),
            (Value::One(item), Value::Empty) => Value::One(item),
            (Value::Empty, Value::Many(items)) => Value::Many(items),
            (Value::Many(items), Value::Empty) => Value::Many(items),
            (Value::One(item1), Value::One(item2)) => Value::Many(Rc::new(vec![item1, item2])),
            (Value::One(item), Value::Many(items)) => {
                let mut many = vec![item];
                many.extend(Rc::as_ref(&items).clone());
                Value::Many(Rc::new(many))
            }
            (Value::Many(items), Value::One(item)) => {
                let mut many = Rc::as_ref(&items).clone();
                many.push(item);
                Value::Many(Rc::new(many))
            }
            (Value::Many(items1), Value::Many(items2)) => {
                let mut many = Rc::as_ref(&items1).clone();
                many.extend(Rc::as_ref(&items2).clone());
                Value::Many(Rc::new(many))
            }
            _ => unreachable!(),
        }
    }

    fn one_node(self) -> error::Result<xml::Node> {
        match self {
            Value::One(item) => item.to_node(),
            _ => Err(error::Error::XPTY0004),
        }
    }

    pub(crate) fn is(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<bool> {
        let a = self.one_node()?;
        let b = other.one_node()?;
        let a_annotation = annotations.get(a).unwrap();
        let b_annotation = annotations.get(b).unwrap();
        Ok(a_annotation.document_order == b_annotation.document_order)
    }

    pub(crate) fn precedes(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<bool> {
        let a = self.one_node()?;
        let b = other.one_node()?;
        let a_annotation = annotations.get(a).unwrap();
        let b_annotation = annotations.get(b).unwrap();
        Ok(a_annotation.document_order < b_annotation.document_order)
    }

    pub(crate) fn follows(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<bool> {
        let a = self.one_node()?;
        let b = other.one_node()?;
        let a_annotation = annotations.get(a).unwrap();
        let b_annotation = annotations.get(b).unwrap();
        Ok(a_annotation.document_order > b_annotation.document_order)
    }

    pub(crate) fn union(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<stack::Value> {
        let mut s = HashSet::new();
        for item in self.items() {
            let node = item?.to_node()?;
            s.insert(node);
        }
        for item in other.items() {
            let node = item?.to_node()?;
            s.insert(node);
        }

        Ok(Self::process_set_result(s, annotations))
    }

    pub(crate) fn intersect(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<stack::Value> {
        let mut s = HashSet::new();
        let mut r = HashSet::new();
        for item in self.items() {
            let node = item?.to_node()?;
            s.insert(node);
        }
        for item in other.items() {
            let node = item?.to_node()?;
            if s.contains(&node) {
                r.insert(node);
            }
        }
        Ok(Self::process_set_result(r, annotations))
    }

    pub(crate) fn except(
        self,
        other: stack::Value,
        annotations: &xml::Annotations,
    ) -> error::Result<stack::Value> {
        let mut s = HashSet::new();
        for item in self.items() {
            let node = item?.to_node()?;
            s.insert(node);
        }
        for item in other.items() {
            let node = item?.to_node()?;
            s.remove(&node);
        }
        Ok(Self::process_set_result(s, annotations))
    }

    // https://www.w3.org/TR/xpath-31/#id-path-operator
    pub(crate) fn deduplicate(self, annotations: &xml::Annotations) -> error::Result<stack::Value> {
        let mut s = HashSet::new();
        let mut non_node_seen = false;
        for item in self.items() {
            match item? {
                sequence::Item::Node(n) => {
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

    fn process_set_result(s: HashSet<xml::Node>, annotations: &xml::Annotations) -> stack::Value {
        // sort nodes by document order
        let mut nodes = s.into_iter().collect::<Vec<_>>();
        nodes.sort_by_key(|n| annotations.document_order(*n));

        let items = nodes
            .into_iter()
            .map(sequence::Item::Node)
            .collect::<Vec<_>>();
        items.into()
    }
}

impl<T> From<T> for Value
where
    T: Into<sequence::Item>,
{
    fn from(item: T) -> Self {
        Value::One(item.into())
    }
}

impl From<Vec<sequence::Item>> for Value {
    fn from(mut items: Vec<sequence::Item>) -> Self {
        if items.is_empty() {
            Value::Empty
        } else if items.len() == 1 {
            Value::One(items.pop().unwrap())
        } else {
            Value::Many(Rc::new(items))
        }
    }
}

impl TryFrom<&stack::Value> for Rc<function::Function> {
    type Error = error::Error;

    fn try_from(value: &stack::Value) -> error::Result<Self> {
        match value {
            stack::Value::One(sequence::Item::Function(c)) => Ok(c.clone()),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl TryFrom<stack::Value> for xml::Node {
    type Error = error::Error;

    fn try_from(value: stack::Value) -> error::Result<xml::Node> {
        TryFrom::try_from(&value)
    }
}

impl TryFrom<&stack::Value> for xml::Node {
    type Error = error::Error;

    fn try_from(value: &stack::Value) -> error::Result<xml::Node> {
        match value {
            stack::Value::One(sequence::Item::Node(n)) => Ok(*n),
            _ => Err(error::Error::XPTY0004),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Empty, Value::Empty) => true,
            (Value::One(a), Value::One(b)) => a == b,
            (Value::Many(a), Value::Many(b)) => a == b,
            _ => false,
        }
    }
}

pub(crate) enum ValueIter {
    Empty,
    OneIter(std::iter::Once<sequence::Item>),
    ManyIter(std::vec::IntoIter<sequence::Item>),
    AbsentIter(std::iter::Once<error::Result<sequence::Item>>),
}

impl ValueIter {
    fn new(value: Value) -> Self {
        match value {
            Value::Empty => ValueIter::Empty,
            Value::One(item) => ValueIter::OneIter(std::iter::once(item)),
            Value::Many(items) => ValueIter::ManyIter(Rc::as_ref(&items).clone().into_iter()),
            Value::Absent => ValueIter::AbsentIter(std::iter::once(Err(error::Error::XPDY0002))),
        }
    }
}

impl Iterator for ValueIter {
    type Item = error::Result<sequence::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ValueIter::Empty => None,
            ValueIter::OneIter(iter) => iter.next().map(Ok),
            ValueIter::ManyIter(iter) => iter.next().map(Ok),
            ValueIter::AbsentIter(iter) => iter.next(),
        }
    }
}

#[derive(Clone)]
pub enum AtomizedIter<'a> {
    Empty,
    One(sequence::AtomizedItemIter<'a>),
    Many(AtomizedManyIter<'a>),
    Erroring(std::iter::Once<error::Result<atomic::Atomic>>),
    Absent(std::iter::Once<error::Result<atomic::Atomic>>),
}

impl<'a> AtomizedIter<'a> {
    fn new(value: Value, xot: &'a Xot) -> Self {
        match value {
            Value::Empty => AtomizedIter::Empty,
            Value::One(item) => AtomizedIter::One(sequence::AtomizedItemIter::new(item, xot)),
            Value::Many(items) => AtomizedIter::Many(AtomizedManyIter::new(
                // TODO: this clone is expensive, can't we use the
                // Rc and do without cloning?
                Rc::as_ref(&items).clone().into_iter(),
                xot,
            )),
            Value::Absent => AtomizedIter::Absent(std::iter::once(Err(error::Error::XPDY0002))),
        }
    }
}

impl<'a> Iterator for AtomizedIter<'a> {
    type Item = error::Result<atomic::Atomic>;

    fn next(&mut self) -> Option<error::Result<atomic::Atomic>> {
        match self {
            AtomizedIter::Empty => None,
            AtomizedIter::One(iter) => iter.next(),
            AtomizedIter::Many(iter) => iter.next(),
            AtomizedIter::Erroring(iter) => iter.next(),
            AtomizedIter::Absent(iter) => iter.next(),
        }
    }
}

#[derive(Clone)]
pub struct AtomizedManyIter<'a> {
    xot: &'a Xot,
    iter: std::vec::IntoIter<sequence::Item>,
    item_iter: Option<sequence::AtomizedItemIter<'a>>,
}

impl<'a> AtomizedManyIter<'a> {
    fn new(iter: std::vec::IntoIter<sequence::Item>, xot: &'a Xot) -> Self {
        Self {
            xot,
            iter,
            item_iter: None,
        }
    }
}

impl<'a> Iterator for AtomizedManyIter<'a> {
    type Item = error::Result<atomic::Atomic>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // if there there are any more atoms in this node,
            // supply those
            if let Some(item_iter) = &mut self.item_iter {
                if let Some(item) = item_iter.next() {
                    return Some(item);
                } else {
                    self.item_iter = None;
                }
            }
            // if not, move on to the next item
            let item = self.iter.next();
            if let Some(item) = item {
                self.item_iter = Some(sequence::AtomizedItemIter::new(item, self.xot));
                continue;
            } else {
                // no more items, we're done
                return None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use ibig::ibig;
    use rust_decimal::Decimal;

    use crate::atomic;

    #[test]
    fn test_integer_compares_with_decimal() {
        let a = atomic::Atomic::Integer(atomic::IntegerType::Integer, ibig!(1).into());
        let b = atomic::Atomic::Decimal(Rc::new(Decimal::from(1)));
        assert!(a.simple_equal(&b));
    }
}
