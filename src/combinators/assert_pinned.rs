use core::marker::{MetaSized, PhantomData};

use crate::{Init, PinInit};

pub struct AssertPinned<T: MetaSized, Extra, I> {
    result: PhantomData<fn(Extra) -> T>,
    init: I,
}

impl<T: MetaSized, Extra, I> AssertPinned<T, Extra, I> {
    pub fn new(init: I) -> Self
    where
        I: Init<T, Extra>,
    {
        Self {
            result: PhantomData,
            init,
        }
    }

    pub unsafe fn new_unchecked(init: I) -> Self
    where
        I: PinInit<T, Extra>,
    {
        Self {
            result: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Extra, I: PinInit<T, Extra>> PinInit<T, Extra>
    for AssertPinned<T, Extra, I>
{
    type Error = I::Error;

    fn metadata(&self) -> <T as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        // SAFETY: discharged to caller
        unsafe { self.init.init(dst, extra) }
    }
}
// SAFETY: By the safety contract of `Self`'s constructor TODO
unsafe impl<T: MetaSized, Extra, I: PinInit<T, Extra>> Init<T, Extra>
    for AssertPinned<T, Extra, I>
{
}
