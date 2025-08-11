use core::{
    marker::{MetaSized, PhantomData},
    pin::Pin,
};

use crate::{Init, PinInit};

pub struct Then<T: MetaSized, I, F> {
    result: PhantomData<fn() -> T>,
    init: I,
    func: F,
}

impl<T: MetaSized, I: Clone, F: Clone> Clone for Then<T, I, F> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            init: self.init.clone(),
            func: self.func.clone(),
        }
    }
}

impl<T: MetaSized, I, F> Then<T, I, F> {
    pub fn new(init: I, func: F) -> Self {
        Self {
            result: PhantomData,
            init,
            func,
        }
    }
}

unsafe impl<
    T: MetaSized,
    Extra,
    E,
    I: Init<T, Extra, Error = E>,
    F: FnOnce(&mut T) -> Result<(), E>,
> PinInit<T, Extra> for Then<T, I, F>
{
    type Error = E;

    fn metadata(&self) -> <T as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        // SAFETY: discharged to caller
        unsafe { self.init.init(dst, extra) }?;

        // SAFETY: we just initialized `*dst`
        let mut dst = unsafe { noop_allocator::owning_ref::from_raw(dst) };
        (self.func)(&mut *dst)?;
        core::mem::forget(dst);
        Ok(())
    }
}
unsafe impl<
    T: MetaSized,
    Extra,
    E,
    I: Init<T, Extra, Error = E>,
    F: FnOnce(&mut T) -> Result<(), E>,
> Init<T, Extra> for Then<T, I, F>
{
}
