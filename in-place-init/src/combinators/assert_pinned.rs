use core::marker::{MetaSized, PhantomData};

use crate::{Init, PinInit};

pub struct AssertPinned<T: MetaSized, Error, Extra, I> {
    result: PhantomData<fn(Extra) -> T>,
    // This needs to know the `Error` type it was constructed with
    // to ensure we don't call a diffetent `PinInit` impl than the one intended
    error: PhantomData<fn() -> Error>,
    init: I,
}

impl<T: MetaSized, Error, Extra, I> AssertPinned<T, Error, Extra, I> {
    pub fn new(init: I) -> Self
    where
        I: Init<T, Error, Extra>,
    {
        Self {
            result: PhantomData,
            error: PhantomData,
            init,
        }
    }

    pub unsafe fn new_unchecked(init: I) -> Self
    where
        I: PinInit<T, Error, Extra>,
    {
        Self {
            result: PhantomData,
            error: PhantomData,
            init,
        }
    }

    pub fn new_unpin(init: I) -> Self
    where
        I: PinInit<T, Error, Extra>,
        T: Unpin,
    {
        Self {
            result: PhantomData,
            error: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: PinInit<T, Error, Extra>> PinInit<T, Error, Extra>
    for AssertPinned<T, Error, Extra, I>
{
    fn metadata(&self) -> <T as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Error> {
        // SAFETY: discharged to caller
        unsafe { self.init.init(dst, extra) }
    }
}
// SAFETY: By the safety contract of `Self`'s constructor TODO
unsafe impl<T: MetaSized, Error, Extra, I: PinInit<T, Error, Extra>> Init<T, Error, Extra>
    for AssertPinned<T, Error, Extra, I>
{
}
