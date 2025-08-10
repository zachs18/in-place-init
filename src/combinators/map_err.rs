use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct MapErr<T: ?Sized + MetaSized, F, I> {
    result: PhantomData<fn() -> T>,
    func: F,
    init: I,
}

impl<T: ?Sized + MetaSized, F, I> MapErr<T, F, I> {
    pub fn new(func: F, init: I) -> Self {
        Self {
            result: PhantomData,
            func,
            init,
        }
    }
}

unsafe impl<
    T: ?Sized + MetaSized,
    Extra,
    E1,
    E2,
    F: FnOnce(E1) -> E2,
    I: PinInit<T, Extra, Error = E1>,
> PinInit<T, Extra> for MapErr<T, F, I>
{
    type Error = E2;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        let result = unsafe { self.init.init(dst, extra) };
        result.map_err(self.func)
    }
}

unsafe impl<
    T: ?Sized + MetaSized,
    Extra,
    E1,
    E2,
    F: FnOnce(E1) -> E2,
    I: Init<T, Extra, Error = E1>,
> Init<T, Extra> for MapErr<T, F, I>
{
}
