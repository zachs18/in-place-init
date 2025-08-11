use core::{
    marker::{MetaSized, PhantomData},
    pin::Pin,
};

use crate::{Init, PinInit};

pub struct ThenPinned<T: MetaSized, I, F> {
    result: PhantomData<fn() -> T>,
    init: I,
    func: F,
}

impl<T: MetaSized, I: Clone, F: Clone> Clone for ThenPinned<T, I, F> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            init: self.init.clone(),
            func: self.func.clone(),
        }
    }
}

impl<T: MetaSized, I, F> ThenPinned<T, I, F> {
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
    I: PinInit<T, Extra, Error = E>,
    F: FnOnce(Pin<&mut T>) -> Result<(), E>,
> PinInit<T, Extra> for ThenPinned<T, I, F>
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
        // SAFETY: `*dst` will be treated as pinned, or is `Unpin` in the `Init` impl
        let pinned = unsafe { Pin::new_unchecked(&mut *dst) };
        (self.func)(pinned)?;
        core::mem::forget(dst);
        Ok(())
    }
}

// SAFETY: `Dst: Unpin`, so `Pin::new_unchecked` is safe (as `Pin::new`)
unsafe impl<
    T: MetaSized + Unpin,
    Extra,
    E,
    I: PinInit<T, Extra, Error = E>,
    F: FnOnce(Pin<&mut T>) -> Result<(), E>,
> Init<T, Extra> for ThenPinned<T, I, F>
{
}
