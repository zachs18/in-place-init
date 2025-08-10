use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct IgnoreExtra<T: ?Sized + MetaSized, I> {
    result: PhantomData<fn() -> T>,
    init: I,
}

impl<T: ?Sized + MetaSized, I> IgnoreExtra<T, I> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            init,
        }
    }
}

unsafe impl<T: ?Sized + MetaSized, Extra, E, I: PinInit<T, Error = E>> PinInit<T, Extra>
    for IgnoreExtra<T, I>
{
    type Error = E;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, _: Extra) -> Result<(), Self::Error> {
        unsafe { self.init.init(dst, ()) }
    }
}

unsafe impl<T: ?Sized + MetaSized, Extra, E, I: Init<T, Error = E>> Init<T, Extra>
    for IgnoreExtra<T, I>
{
}
