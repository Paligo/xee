use crate::error;

pub fn one<'a, T>(mut iter: impl Iterator<Item = error::Result<T>> + 'a) -> error::Result<T> {
    if let Some(one) = iter.next() {
        if iter.next().is_none() {
            Ok(one?)
        } else {
            Err(error::Error::new(error::ErrorCode::XPTY0004))
        }
    } else {
        Err(error::Error::new(error::ErrorCode::XPTY0004))
    }
}

pub fn option<'a, T>(
    mut iter: impl Iterator<Item = error::Result<T>> + 'a,
) -> error::Result<Option<T>> {
    if let Some(one) = iter.next() {
        if iter.next().is_none() {
            Ok(Some(one?))
        } else {
            Err(error::Error::new(error::ErrorCode::XPTY0004))
        }
    } else {
        Ok(None)
    }
}
