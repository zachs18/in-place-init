use crate::{Init, PinInit};

pub struct Chain<I1, I2> {
    init1: I1,
    init2: I2,
}

impl<I1, I2> Chain<I1, I2> {
    pub fn new(init1: I1, init2: I2) -> Self {
        Self { init1, init2 }
    }
}

unsafe impl<
    T,
    Extra: Clone,
    E,
    I1: PinInit<[T], Extra, Error = E>,
    I2: PinInit<[T], Extra, Error = E>,
> PinInit<[T], Extra> for Chain<I1, I2>
{
    type Error = E;

    fn metadata(&self) -> usize {
        self.init1
            .metadata()
            .checked_add(self.init2.metadata())
            .expect("slice length overflow")
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        let len1 = self.init1.metadata();
        let len2 = self.init2.metadata();
        debug_assert_eq!(dst.len(), len1 + len2);
        let dst = dst.cast::<T>();

        let dst1 = core::ptr::slice_from_raw_parts_mut(dst, len1);
        unsafe { self.init1.init(dst1, extra.clone()) }?;

        // Safety: The elements were just initialized.
        // Drop the first slice's elements if the second initializer panics or returns an error.
        let guard = unsafe { noop_allocator::owning_slice::full_from_raw(dst1) };

        let dst2 = core::ptr::slice_from_raw_parts_mut(unsafe { dst.add(len1) }, len2);
        unsafe { self.init2.init(dst2, extra) }?;

        // Safety: Defuse the panic guard
        core::mem::forget(guard);
        Ok(())
    }
}
unsafe impl<T, Extra: Clone, E, I1: Init<[T], Extra, Error = E>, I2: Init<[T], Extra, Error = E>>
    Init<[T], Extra> for Chain<I1, I2>
{
}

unsafe impl<Extra: Clone, E, I1: PinInit<str, Extra, Error = E>, I2: PinInit<str, Extra, Error = E>>
    PinInit<str, Extra> for Chain<I1, I2>
{
    type Error = E;

    fn metadata(&self) -> usize {
        self.init1
            .metadata()
            .checked_add(self.init2.metadata())
            .expect("slice length overflow")
    }

    unsafe fn init(self, dst: *mut str, extra: Extra) -> Result<(), Self::Error> {
        let len1 = self.init1.metadata();
        let len2 = self.init2.metadata();
        debug_assert_eq!((dst as *mut [u8]).len(), len1 + len2);
        let dst = dst.cast::<u8>();

        let dst1 = core::ptr::slice_from_raw_parts_mut(dst, len1) as *mut str;
        unsafe { self.init1.init(dst1, extra.clone()) }?;

        let dst2 = core::ptr::slice_from_raw_parts_mut(unsafe { dst.add(len1) }, len2) as *mut str;
        unsafe { self.init2.init(dst2, extra) }?;

        Ok(())
    }
}
unsafe impl<Extra: Clone, E, I1: Init<str, Extra, Error = E>, I2: Init<str, Extra, Error = E>>
    Init<str, Extra> for Chain<I1, I2>
{
}
