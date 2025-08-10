use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

/// An initializer that will always succeed. This can be helpful to use an infallible initializer in a combinator with a fallible initializer.
pub struct Succeed<T: MetaSized, I, E> {
    result: PhantomData<fn() -> T>,
    err: PhantomData<fn() -> E>,
    init: I,
}

impl<T: MetaSized, I: Clone, E: Clone> Clone for Succeed<T, I, E> {
    fn clone(&self) -> Self {
        Self {
            result: self.result.clone(),
            err: self.err.clone(),
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, I, E> Succeed<T, I, E> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            err: PhantomData,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Extra, E, I: PinInit<T, Extra, Error = !>> PinInit<T, Extra>
    for Succeed<T, I, E>
{
    type Error = E;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        let result = unsafe { self.init.init(dst, extra) };
        result.map_err(|e| match e {})
    }
}

unsafe impl<T: MetaSized, Extra, E, I: Init<T, Extra, Error = !>> Init<T, Extra>
    for Succeed<T, I, E>
{
}
