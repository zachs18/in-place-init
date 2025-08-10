use core::marker::{MetaSized, PhantomData};

use crate::{Init, PinInit};

/// An initializer that is restricted to initialize a specific type.
///
/// Some initializers can be used to initialize several types, e.g. [`crate::ArrayForEach`] can
/// be used to initialize a slice or an array. Also, every `Sized` type can be used to initialize
/// itself by-value.
///
/// This type can be useful to guide type checking by restricting an intializer to one destination type.
pub struct ForType<T: MetaSized, I> {
    result: PhantomData<fn() -> T>,
    init: I,
}

impl<T: MetaSized, I: Clone> Clone for ForType<T, I> {
    fn clone(&self) -> Self {
        Self {
            result: self.result.clone(),
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, I> ForType<T, I> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Extra, I: PinInit<T, Extra>> PinInit<T, Extra> for ForType<T, I> {
    type Error = I::Error;

    fn metadata(&self) -> <T as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        // SAFETY: discharged to caller
        unsafe { self.init.init(dst, extra) }
    }
}

unsafe impl<T: MetaSized, Extra, I: Init<T, Extra>> Init<T, Extra> for ForType<T, I> {}
