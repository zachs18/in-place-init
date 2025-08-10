use core::{
    marker::{MetaSized, PhantomData},
    mem::MaybeUninit,
    ptr::Pointee,
};

use crate::{Init, PinInit};

/// An initializer for `MaybeUninit<T>` that initailizes all bytes to zero.
///
/// ```rust
/// let zeros: Box<[u32]> = in_place_init::new_boxed(unsafe {
///     in_place_init::Zeroed::new_unchecked(42)
/// });
///
/// assert_eq!(*zeros, [0; 42]);
/// ```
pub struct Zeroed<T: MetaSized> {
    result: PhantomData<fn() -> T>,
    meta: <T as Pointee>::Metadata,
}

impl<T: MetaSized> Clone for Zeroed<T> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            meta: self.meta,
        }
    }
}

impl<T: MetaSized> Copy for Zeroed<T> {}

impl Zeroed<str> {
    pub fn new_str(length: usize) -> Zeroed<str> {
        Zeroed {
            result: PhantomData,
            meta: length,
        }
    }
}

impl<T: MetaSized> Zeroed<T> {
    pub fn new() -> Zeroed<MaybeUninit<T>>
    where
        T: Sized,
    {
        Zeroed {
            result: PhantomData,
            meta: (),
        }
    }

    pub fn new_slice(length: usize) -> Zeroed<[MaybeUninit<T>]>
    where
        T: Sized,
    {
        Zeroed {
            result: PhantomData,
            meta: length,
        }
    }

    /// Create an initializer that fills a `T` with zero bytes.
    ///
    /// # Safety
    ///
    /// A `T` with metadata `meta` consisting of all bytes zero must be valid.
    ///
    /// The size of `T` with metadata `meta` must be valid.
    pub unsafe fn new_unchecked(meta: <T as Pointee>::Metadata) -> Self {
        Self {
            result: PhantomData,
            meta,
        }
    }

    /// Create an initializer that fills a `T` with zero bytes.
    #[cfg(feature = "bytemuck")]
    pub fn new_zeroable() -> Self
    where
        T: bytemuck::Zeroable,
    {
        Zeroed {
            result: PhantomData,
            meta: (),
        }
    }

    /// Create an initializer that fills a slice of `T` with zero bytes.
    #[cfg(feature = "bytemuck")]
    pub fn new_zeroable_slice(length: usize) -> Zeroed<[T]>
    where
        T: bytemuck::Zeroable,
    {
        Zeroed {
            result: PhantomData,
            meta: length,
        }
    }
}

unsafe impl<T: MetaSized> PinInit<T> for Zeroed<T> {
    type Error = !;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.meta
    }

    unsafe fn init(self, dst: *mut T, _extra: ()) -> Result<(), Self::Error> {
        let size = unsafe { core::mem::size_of_val_raw(dst) };
        unsafe { core::ptr::write_bytes(dst.cast::<u8>(), 0, size) };
        // `Self` can only be constructed if `T` with meta `self.meta`
        // is valid as all zero bytes.
        Ok(())
    }
}
unsafe impl<T: MetaSized> Init<T> for Zeroed<T> {}
