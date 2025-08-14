use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

/// An initializer that will always succeed. This can be helpful to use an infallible initializer in a combinator with a fallible initializer.
///
/// FIXME: This is probably not
pub struct Succeed<T: MetaSized, I, Error> {
    result: PhantomData<fn() -> T>,
    err: PhantomData<fn() -> Error>,
    init: I,
}

impl<T: MetaSized, I: Clone, Error> Clone for Succeed<T, I, Error> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            err: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, I: Copy, Error> Copy for Succeed<T, I, Error> {}

impl<T: MetaSized, I, E> Succeed<T, I, E> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            err: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Error, Extra, E, I: PinInit<T, !, Extra>> PinInit<T, Error, Extra>
    for Succeed<T, I, E>
{
    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Error> {
        let result = unsafe { self.init.init(dst, extra) };
        result.map_err(|e| match e {})
    }
}

unsafe impl<T: MetaSized, Error, Extra, I: Init<T, !, Extra>> Init<T, Error, Extra>
    for Succeed<T, I, Error>
{
}
