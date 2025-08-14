use core::marker::{MetaSized, PhantomData};

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
    Error,
    I: Init<T, Error, Extra>,
    F: FnOnce(&mut T) -> Result<(), Error>,
> PinInit<T, Error, Extra> for Then<T, I, F>
{
    fn metadata(&self) -> <T as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Error> {
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
    Error,
    I: Init<T, Error, Extra>,
    F: FnOnce(&mut T) -> Result<(), Error>,
> Init<T, Error, Extra> for Then<T, I, F>
{
}
