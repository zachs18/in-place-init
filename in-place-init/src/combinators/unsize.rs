use core::marker::Unsize as UnsizeTrait;
use core::marker::{MetaSized, PhantomData};
use core::ptr::Pointee;

use crate::{Init, PinInit};

/// Initialize an unsized place by writing a sized value.
pub struct Unsize<T, Dst: MetaSized, I> {
    // We need to mention `Dst`, otherwise `impl Init<Dst> for Unsize<U, I> where U: Unsize<Dst>`
    // overlaps with `impl Init<T> for T` where `T = Unsize<U, I>: Unsize<Dst>`
    result: PhantomData<fn() -> (T, Dst)>,
    init: I,
}

impl<T: UnsizeTrait<Dst>, Dst: MetaSized, I: Clone> Clone for Unsize<T, Dst, I> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            init: self.init.clone(),
        }
    }
}

impl<T: UnsizeTrait<Dst>, Dst: MetaSized, I: Copy> Copy for Unsize<T, Dst, I> {}

impl<T: UnsizeTrait<Dst>, Dst: MetaSized, I> Unsize<T, Dst, I> {
    pub fn new(init: I) -> Self {
        Self {
            result: PhantomData,
            init,
        }
    }
}

unsafe impl<Dst: MetaSized, T: UnsizeTrait<Dst>, Error, Extra, I: Init<T, Error, Extra>>
    PinInit<Dst, Error, Extra> for Unsize<T, Dst, I>
{
    fn metadata(&self) -> <Dst as Pointee>::Metadata {
        let ptr: *const Dst = core::ptr::null::<T>();
        core::ptr::metadata(ptr)
    }

    unsafe fn init(self, dst: *mut Dst, extra: Extra) -> Result<(), Error> {
        unsafe { self.init.init(dst.cast::<T>(), extra) }
    }
}
unsafe impl<Dst: MetaSized, T: UnsizeTrait<Dst>, Error, Extra, I: Init<T, Error, Extra>>
    Init<Dst, Error, Extra> for Unsize<T, Dst, I>
{
}
