use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct MapErr<T: MetaSized, ESrc, F, I> {
    result: PhantomData<fn() -> T>,
    map: PhantomData<fn(ESrc)>,
    func: F,
    init: I,
}

impl<T: MetaSized, E1, F: Clone, I: Clone> Clone for MapErr<T, E1, F, I> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            map: PhantomData,
            func: self.func.clone(),
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, E1, F, I> MapErr<T, E1, F, I> {
    pub fn new(func: F, init: I) -> Self {
        Self {
            result: PhantomData,
            map: PhantomData,
            func,
            init,
        }
    }
}

unsafe impl<T: MetaSized, Extra, E1, E2, F: FnOnce(E1) -> E2, I: PinInit<T, E1, Extra>>
    PinInit<T, E2, Extra> for MapErr<T, E1, F, I>
{
    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), E2> {
        let result = unsafe { self.init.init(dst, extra) };
        result.map_err(self.func)
    }
}

unsafe impl<T: MetaSized, Extra, E1, E2, F: FnOnce(E1) -> E2, I: Init<T, E1, Extra>>
    Init<T, E2, Extra> for MapErr<T, E1, F, I>
{
}
