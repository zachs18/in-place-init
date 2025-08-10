use core::{
    marker::{MetaSized, PhantomData},
    ptr::Pointee,
};

use crate::{Init, PinInit};

pub struct MapExtra<T: MetaSized, F, I> {
    result: PhantomData<fn() -> T>,
    func: F,
    init: I,
}

impl<T: MetaSized, F: Clone, I: Clone> Clone for MapExtra<T, F, I> {
    fn clone(&self) -> Self {
        Self {
            result: self.result.clone(),
            func: self.func.clone(),
            init: self.init.clone(),
        }
    }
}

impl<T: MetaSized, F, I> MapExtra<T, F, I> {
    pub fn new(func: F, init: I) -> Self {
        Self {
            result: PhantomData,
            func,
            init,
        }
    }
}

unsafe impl<
    T: MetaSized,
    Extra1,
    Extra2,
    E,
    F: FnOnce(Extra1) -> Result<Extra2, E>,
    I: PinInit<T, Extra2, Error = E>,
> PinInit<T, Extra1> for MapExtra<T, F, I>
{
    type Error = E;

    fn metadata(&self) -> <T as Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra1) -> Result<(), Self::Error> {
        let extra = (self.func)(extra)?;
        unsafe { self.init.init(dst, extra) }
    }
}

unsafe impl<
    T: MetaSized,
    Extra1,
    Extra2,
    E,
    F: FnOnce(Extra1) -> Result<Extra2, E>,
    I: Init<T, Extra2, Error = E>,
> Init<T, Extra1> for MapExtra<T, F, I>
{
}
