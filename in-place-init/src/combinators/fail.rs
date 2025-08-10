use core::{marker::MetaSized, ptr::Pointee};

use crate::{Init, PinInit};

/// An initializer that will always fail to initialize.
pub struct Fail<T: MetaSized, E> {
    meta: <T as Pointee>::Metadata,
    err: E,
}

impl<T: MetaSized, E: Clone> Clone for Fail<T, E> {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta,
            err: self.err.clone(),
        }
    }
}
impl<T: MetaSized, E: Copy> Copy for Fail<T, E> {}

impl<T: MetaSized, E> Fail<T, E> {
    pub fn new(err: E) -> Self
    where
        <T as Pointee>::Metadata: Default,
    {
        Self {
            meta: Default::default(),
            err,
        }
    }

    pub fn new_with_meta(meta: <T as Pointee>::Metadata, err: E) -> Self {
        Self { meta, err }
    }
}

unsafe impl<T: MetaSized, E> PinInit<T> for Fail<T, E> {
    type Error = E;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.meta
    }

    unsafe fn init(self, _dst: *mut T, _extra: ()) -> Result<(), Self::Error> {
        Err(self.err)
    }
}
unsafe impl<T: MetaSized, E> Init<T> for Fail<T, E> {}
