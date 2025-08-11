use crate::{
    Init, PinInit, VecExt,
    util::{ConstLength, Length, RuntimeLength},
};

/// Initialize an array or slice by creating an initializer for each element.
///
/// ```rust
/// # use in_place_init::{Init, PinInit};
///
/// let bx: Box<[usize; 3]> = in_place_init::new_boxed(
///     in_place_init::array_for_each(|idx| idx * 2 + 1)
/// );
/// assert_eq!(*bx, [1, 3, 5]);
///
/// let bx: Box<[usize]> = in_place_init::new_boxed(
///     in_place_init::array_for_each::<3, _>(|idx| idx * 2 + 1)
/// );
/// assert_eq!(*bx, [1, 3, 5]);
/// ```
#[derive(Clone)]
pub struct ForEach<F, L: Length> {
    length: L,
    func: F,
}

impl<F> ForEach<F, RuntimeLength> {
    pub fn new_slice(length: usize, func: F) -> Self {
        Self {
            length: RuntimeLength { length },
            func,
        }
    }

    pub fn new_array<const N: usize>(func: F) -> ForEach<F, ConstLength<N>> {
        ForEach {
            length: ConstLength,
            func,
        }
    }
}

unsafe impl<T, L: Length, Extra: Clone, I: PinInit<T, Extra>, F: FnMut(usize) -> I>
    PinInit<[T], Extra> for ForEach<F, L>
{
    type Error = I::Error;

    fn metadata(&self) -> usize {
        self.length.length()
    }

    unsafe fn init(mut self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = self.length.length();
        debug_assert_eq!(buf.capacity(), count);
        for (idx, extra) in core::iter::repeat_n(extra, count).enumerate() {
            let init = (self.func)(idx);
            let init = crate::with_extra(init, extra);
            // SAFETY: either `init: Init`, or we treat the destination as pinned
            let init = unsafe { crate::assert_pinned(init) };
            // SAFETY: there is excess capacity
            unsafe { buf.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, L: Length, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I> Init<[T], Extra>
    for ForEach<F, L>
{
}

unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    PinInit<[T; N], Extra> for ForEach<F, ConstLength<N>>
{
    type Error = I::Error;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; N], extra: Extra) -> Result<(), Self::Error> {
        unsafe { <Self as PinInit<[T], Extra>>::init(self, dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I>
    Init<[T; N], Extra> for ForEach<F, ConstLength<N>>
{
}
