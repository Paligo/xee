use crate::atomic;
use crate::error;

pub(crate) fn unary_plus(atomic: atomic::Atomic) -> error::Result<atomic::Atomic> {
    match &atomic {
        atomic::Atomic::Integer(_, _)
        | atomic::Atomic::Decimal(_)
        | atomic::Atomic::Float(_)
        | atomic::Atomic::Double(_) => Ok(atomic.clone()),
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}

pub(crate) fn unary_minus(atomic: atomic::Atomic) -> error::Result<atomic::Atomic> {
    match atomic {
        atomic::Atomic::Integer(_, i) => Ok((-i.as_ref().clone()).into()),
        atomic::Atomic::Decimal(d) => Ok((-*d.as_ref()).into()),
        atomic::Atomic::Float(f) => Ok((-f).into()),
        atomic::Atomic::Double(d) => Ok((-d).into()),
        _ => Err(error::Error::new(error::ErrorCode::XPTY0004)),
    }
}
