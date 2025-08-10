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

unsafe impl<T, const N: usize, Extra, I: PinInit<[T; N], Extra>> PinInit<[T], Extra>
    for ForSlice<I, N>
{
    type Error = I::Error;

    fn metadata(&self) -> usize {
        N
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        // SAFETY: discharged to caller, `dst` must have the length `N`
        unsafe { self.init.init(dst.cast(), extra) }
    }
}

unsafe impl<T, const N: usize, Extra, I: Init<[T; N], Extra>> Init<[T], Extra> for ForSlice<I, N> {}
