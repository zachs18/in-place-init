use crate::{Init, PinInit};

/// Intialize a slice in two pieces.
#[derive(Clone, Copy)]
pub struct Chain<I1, I2> {
    init1: I1,
    init2: I2,
}

impl<I1, I2> Chain<I1, I2> {
    pub fn new(init1: I1, init2: I2) -> Self {
        Self { init1, init2 }
    }
}

unsafe impl<T, Error, Extra: Clone, I1: PinInit<[T], Error, Extra>, I2: PinInit<[T], Error, Extra>>
    PinInit<[T], Error, Extra> for Chain<I1, I2>
{
    fn metadata(&self) -> usize {
        self.init1
            .metadata()
            .checked_add(self.init2.metadata())
            .expect("slice length overflow")
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Error> {
        let len1 = self.init1.metadata();
        let len2 = self.init2.metadata();
        debug_assert_eq!(dst.len(), len1 + len2);
        let dst = dst.cast::<T>();

        let dst1 = core::ptr::slice_from_raw_parts_mut(dst, len1);
        unsafe { self.init1.init(dst1, extra.clone()) }?;

        // Safety: The elements were just initialized.
        // Drop the first slice's elements if the second initializer panics or returns an error.
        let guard = unsafe { noop_allocator::owning_ref::from_raw(dst1) };

        let dst2 = core::ptr::slice_from_raw_parts_mut(unsafe { dst.add(len1) }, len2);
        unsafe { self.init2.init(dst2, extra) }?;

        // Safety: Defuse the panic guard
        core::mem::forget(guard);
        Ok(())
    }
}
unsafe impl<T, Error, Extra: Clone, I1: Init<[T], Error, Extra>, I2: Init<[T], Error, Extra>>
    Init<[T], Error, Extra> for Chain<I1, I2>
{
}

unsafe impl<Error, Extra: Clone, I1: PinInit<str, Error, Extra>, I2: PinInit<str, Error, Extra>>
    PinInit<str, Error, Extra> for Chain<I1, I2>
{
    fn metadata(&self) -> usize {
        self.init1
            .metadata()
            .checked_add(self.init2.metadata())
            .expect("slice length overflow")
    }

    unsafe fn init(self, dst: *mut str, extra: Extra) -> Result<(), Error> {
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
unsafe impl<Error, Extra: Clone, I1: Init<str, Error, Extra>, I2: Init<str, Error, Extra>>
    Init<str, Error, Extra> for Chain<I1, I2>
{
}
