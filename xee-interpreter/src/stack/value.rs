// the stack::Value is either a sequence or absent

use crate::error;

use crate::sequence;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Absent,
    Sequence(sequence::Sequence),
}

// a static assertion to ensure that Value never grows in size
#[cfg(target_arch = "x86_64")]
static_assertions::assert_eq_size!(Value, [u8; 24]);

impl Value {
    pub(crate) fn is_absent(&self) -> bool {
        matches!(self, Value::Absent)
    }
}

impl TryFrom<Value> for sequence::Sequence {
    type Error = error::Error;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Absent => Err(error::Error::new(error::ErrorCode::XPDY0002)),
            Value::Sequence(sequence) => Ok(sequence),
        }
    }
}

impl TryFrom<&Value> for sequence::Sequence {
    type Error = error::Error;

    fn try_from(value: &Value) -> Result<Self, Self::Error> {
        match value {
            Value::Absent => Err(error::Error::new(error::ErrorCode::XPDY0002)),
            Value::Sequence(sequence) => Ok(sequence.clone()),
        }
    }
}

impl<T> From<T> for Value
where
    T: Into<sequence::Sequence>,
{
    fn from(t: T) -> Self {
        let sequence: sequence::Sequence = t.into();
        Value::Sequence(sequence)
    }
}
