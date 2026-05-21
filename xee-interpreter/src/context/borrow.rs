use std::{cell::{Ref, RefCell, RefMut}, sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard}};


pub trait BorrowWith<'a> {
    type Read: 'a;
    type Write: 'a;

    fn borrow(&'a self) -> Self::Read;
    fn borrow_mut(&'a self) -> Self::Write;
}

impl<'a, T: 'a> BorrowWith<'a> for RefCell<T> {
    type Read = Ref<'a, T>;
    type Write = RefMut<'a, T>;

    fn borrow(&'a self) -> Self::Read {
        self.borrow()
    }

    fn borrow_mut(&'a self) -> Self::Write {
        self.borrow_mut()
    }
}

impl<'a, T: 'a> BorrowWith<'a> for Arc<RwLock<T>> {
    type Read = RwLockReadGuard<'a, T>;
    type Write = RwLockWriteGuard<'a, T>;

    fn borrow(&'a self) -> Self::Read {
        self.read().unwrap()
    }

    fn borrow_mut(&'a self) -> Self::Write {
        self.write().unwrap()
    }
}
