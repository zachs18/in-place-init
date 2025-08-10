use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct WithExtra<T: ?Sized + MetaSized, I, Extra> {
    result: PhantomData<fn() -> T>,
    extra: Extra,
    init: I,
}
impl<T: ?Sized + MetaSized, I, Extra> WithExtra<T, I, Extra> {
    pub fn new(extra: Extra, init: I) -> Self {
        Self {
            result: PhantomData,
            extra,
            init,
        }
    }
}

unsafe impl<T: ?Sized + MetaSized, Extra, E, I: PinInit<T, Extra, Error = E>> PinInit<T>
    for WithExtra<T, I, Extra>
{
    type Error = E;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), Self::Error> {
        unsafe { self.init.init(dst, self.extra) }
    }
}

unsafe impl<T: ?Sized + MetaSized, Extra, E, I: Init<T, Extra, Error = E>> Init<T>
    for WithExtra<T, I, Extra>
{
}
