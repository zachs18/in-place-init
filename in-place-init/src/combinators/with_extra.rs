use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct WithExtra<T: MetaSized, Extra, I> {
    /// This type needs to mention `T`, otherwise the relevant implementation
    /// would overlap with `impl<T> PinInit<T> for T`.
    result: PhantomData<fn() -> T>,
    extra: Extra,
    init: I,
}

impl<T: MetaSized, Extra: Clone, I: Clone> Clone for WithExtra<T, Extra, I> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            extra: self.extra.clone(),
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, Extra: Copy, I: Copy> Copy for WithExtra<T, Extra, I> {}

impl<T: MetaSized, Extra, I> WithExtra<T, Extra, I> {
    pub fn new(init: I, extra: Extra) -> Self {
        Self {
            result: PhantomData,
            extra,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: PinInit<T, Error, Extra>> PinInit<T, Error>
    for WithExtra<T, Extra, I>
{
    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), Error> {
        unsafe { self.init.init(dst, self.extra) }
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: Init<T, Error, Extra>> Init<T, Error>
    for WithExtra<T, Extra, I>
{
}
