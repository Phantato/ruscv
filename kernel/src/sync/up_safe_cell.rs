#![allow(unused)]
use core::cell::{Ref, RefCell, RefMut};

pub struct UPSafeCell<T> {
    inner: RefCell<T>,
}
unsafe impl<T> Sync for UPSafeCell<T> {}

impl<T> UPSafeCell<T> {
    pub const unsafe fn new(val: T) -> Self {
        Self {
            inner: RefCell::new(val),
        }
    }
    pub fn get(&self) -> Ref<'_, T> {
        self.inner.borrow()
    }
    pub fn get_mut(&self) -> RefMut<'_, T> {
        self.inner.borrow_mut()
    }
}
