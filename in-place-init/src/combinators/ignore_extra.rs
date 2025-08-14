use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct IgnoreExtra<T: MetaSized, I> {
    result: PhantomData<fn() -> T>,
    init: I,
}

impl<T: MetaSized, I: Clone> Clone for IgnoreExtra<T, I> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, I: Copy> Copy for IgnoreExtra<T, I> {}

impl<T: MetaSized, I> IgnoreExtra<T, I> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: PinInit<T, Error>> PinInit<T, Error, Extra>
    for IgnoreExtra<T, I>
{
    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, _: Extra) -> Result<(), Error> {
        unsafe { self.init.init(dst, ()) }
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: Init<T, Error>> Init<T, Error, Extra>
    for IgnoreExtra<T, I>
{
}
