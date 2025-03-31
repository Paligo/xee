use std::cmp::Ordering;

use xot::Xot;

use crate::{error, function, string::Collation};

use super::{core::Sequence, item::Item};

impl Sequence {
    /// Compare two sequences using XPath deep equal rules.
    ///
    /// <https://www.w3.org/TR/xpath-functions-31/#func-deep-equal>
    pub fn deep_equal(
        &self,
        other: &Self,
        collation: &Collation,
        default_offset: chrono::FixedOffset,
        xot: &Xot,
    ) -> error::Result<bool> {
        // https://www.w3.org/TR/xpath-functions-31/#func-deep-equal
        if self.is_empty() && other.is_empty() {
            return Ok(true);
        }
        if self.len() != other.len() {
            return Ok(false);
        }
        for (a, b) in self.iter().zip(other.iter()) {
            match (a, b) {
                (Item::Atomic(a), Item::Atomic(b)) => {
                    if !a.deep_equal(&b, collation, default_offset) {
                        return Ok(false);
                    }
                }
                (Item::Node(a), Item::Node(b)) => {
                    if !xot.deep_equal_xpath(a, b, |a, b| collation.compare(a, b).is_eq()) {
                        return Ok(false);
                    }
                }
                (Item::Function(a), Item::Function(b)) => match (a, b) {
                    (function::Function::Array(a), function::Function::Array(b)) => {
                        if !a.deep_equal(b.clone(), collation, default_offset, xot)? {
                            return Ok(false);
                        }
                    }
                    (function::Function::Map(a), function::Function::Map(b)) => {
                        if !a.deep_equal(&b, collation, default_offset, xot)? {
                            return Ok(false);
                        }
                    }
                    (function::Function::Map(_), function::Function::Array(_)) => return Ok(false),
                    (function::Function::Array(_), function::Function::Map(_)) => return Ok(false),
                    _ => return Err(error::Error::FOTY0015),
                },
                _ => {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }

    pub(crate) fn fallible_compare(
        &self,
        other: &Self,
        collation: &Collation,
        implicit_offset: chrono::FixedOffset,
    ) -> error::Result<Ordering> {
        // we get atoms not by atomizing, but by trying to turn each
        // item into an atom. If it's not an atom, it's not comparable
        // by eq, lt, gt, etc.
        let a_atoms = self.iter().map(|item| item.to_atomic());
        let mut b_atoms = other.iter().map(|item| item.to_atomic());
        for a_atom in a_atoms {
            let b_atom = b_atoms.next();
            let a_atom = a_atom?;
            if let Some(b_atom) = b_atom {
                let b_atom = b_atom?;
                let ordering = a_atom.fallible_compare(&b_atom, collation, implicit_offset)?;
                if !ordering.is_eq() {
                    return Ok(ordering);
                }
            } else {
                return Ok(Ordering::Greater);
            }
        }
        if b_atoms.next().is_some() {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Equal)
        }
    }

    /// For use in sorting. If the comparison fails, it's always Ordering::Less
    /// Another pass is required to determine whether the sequence is in order
    /// or whether the comparison failed.
    pub(crate) fn compare(
        &self,
        other: &Self,
        collation: &Collation,
        implicit_offset: chrono::FixedOffset,
    ) -> Ordering {
        self.fallible_compare(other, collation, implicit_offset)
            .unwrap_or(Ordering::Less)
    }
}
