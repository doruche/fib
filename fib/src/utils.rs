//! Lib-private utilities

use std::{cell::UnsafeCell, fmt::Debug};

/// A single-thread cell. \
/// SAFETY \
/// When yielding control, we have to keep a mutable reference to runtime,
/// which leads to a problem: multiple mutable references to the same data.
/// When we use STCell, we take the responsibility to ensure that
/// every time we leave current logic flow (e.g. yielding),
/// we have to update the value in STCell.
pub(crate) struct STCell<R> {
    pub(crate) inner: UnsafeCell<Option<R>>,
}

impl<T: Debug> Debug for STCell<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("STCell")
            .field("inner", unsafe { &*self.inner.get() })
            .finish()
    }
}

impl<R> STCell<R> {
    pub(crate) const fn new(value: R) -> Self {
        Self { inner: UnsafeCell::new(Some(value)) }
    }

    pub(crate) fn get(&self) -> &R {
        unsafe { (*self.inner.get()).as_ref().unwrap() }
    }

    pub(crate) fn get_mut(&self) -> &mut R {
        unsafe { (*self.inner.get()).as_mut().unwrap() }
    }
}