use crate::{Init, PinInit};

/// An initailizer for slices of a specific length, that delegates to an initializer for arrays of that length.
#[derive(Clone, Copy)]
pub struct ForSlice<I, const N: usize> {
    init: I,
}

impl<I, const N: usize> ForSlice<I, N> {
    pub fn new(init: I) -> Self {
        Self { init }
    }
}

unsafe impl<T, const N: usize, Error, Extra, I: PinInit<[T; N], Error, Extra>>
    PinInit<[T], Error, Extra> for ForSlice<I, N>
{
    fn metadata(&self) -> usize {
        N
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Error> {
        // SAFETY: discharged to caller, `dst` must have the length `N`
        unsafe { self.init.init(dst.cast(), extra) }
    }
}

unsafe impl<T, const N: usize, Error, Extra, I: Init<[T; N], Error, Extra>> Init<[T], Error, Extra>
    for ForSlice<I, N>
{
}
