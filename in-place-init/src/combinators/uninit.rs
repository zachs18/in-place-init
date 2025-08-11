use core::{
    marker::{MetaSized, PhantomData},
    mem::MaybeUninit,
    ptr::Pointee,
};

use crate::{Init, PinInit};

/// An initializer for `MaybeUninit<T>` that does not initailize anything.
pub struct Uninit<T: MetaSized> {
    result: PhantomData<fn() -> T>,
    meta: <T as Pointee>::Metadata,
}

impl<T: MetaSized> Clone for Uninit<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: MetaSized> Copy for Uninit<T> {}

impl<T> Uninit<MaybeUninit<T>> {
    pub fn new() -> Self {
        Self {
            result: PhantomData,
            meta: (),
        }
    }
}

impl<T> Uninit<[MaybeUninit<T>]> {
    pub fn new_slice(length: usize) -> Self {
        Self {
            result: PhantomData,
            meta: length,
        }
    }
}

impl<T: MetaSized> Uninit<T> {
    /// # Safety
    ///
    /// A `T` with metadata `meta` consisting of uninitialized bytes must be valid.
    pub unsafe fn new_unchecked(meta: <T as Pointee>::Metadata) -> Self {
        Self {
            result: PhantomData,
            meta,
        }
    }
}

unsafe impl<T: MetaSized> PinInit<T> for Uninit<T> {
    type Error = !;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.meta
    }

    unsafe fn init(self, _dst: *mut T, _extra: ()) -> Result<(), Self::Error> {
        // `Self` can only be constructed if `T` with meta `self.meta`
        // is valid to leave uninitialized.
        Ok(())
    }
}
unsafe impl<T: MetaSized> Init<T> for Uninit<T> {}
