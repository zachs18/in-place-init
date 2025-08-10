use crate::{Init, PinInit, PinInitExt, VecExt};

/// Initialize an array or slice by creating an initializer for each element.
///
/// ```rust
/// # use in_place_init::{Init, PinInit};
///
/// let bx: Box<[usize; 3]> = in_place_init::new(
///     in_place_init::array_for_each(|idx| idx * 2 + 1)
/// );
/// assert_eq!(*bx, [1, 3, 5]);
///
/// let bx: Box<[usize]> = in_place_init::new(
///     in_place_init::array_for_each::<_, 3>(|idx| idx * 2 + 1)
/// );
/// assert_eq!(*bx, [1, 3, 5]);
/// ```
pub struct ArrayForEach<F, const N: usize> {
    func: F,
}

impl<F, const N: usize> ArrayForEach<F, N> {
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    PinInit<[T], Extra> for ArrayForEach<F, N>
{
    type Error = I::Error;

    fn metadata(&self) -> usize {
        N
    }

    unsafe fn init(mut self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = N;
        debug_assert_eq!(buf.capacity(), N);
        while buf.len() < count {
            let init = (self.func)(buf.len());
            // SAFETY: there is excess capacity
            unsafe {
                buf.try_push_emplace_within_capacity_unchecked(init.with_extra(extra.clone()))
            }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    Init<[T], Extra> for ArrayForEach<F, N>
{
}

unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    PinInit<[T; N], Extra> for ArrayForEach<F, N>
{
    type Error = I::Error;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; N], extra: Extra) -> Result<(), Self::Error> {
        unsafe { <Self as PinInit<[T], Extra>>::init(self, dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    Init<[T; N], Extra> for ArrayForEach<F, N>
{
}
