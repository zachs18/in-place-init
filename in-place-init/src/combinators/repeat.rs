use crate::{
    Init, PinInit, PinInitExt, VecExt,
    util::{ConstLength, Length, RuntimeLength},
};

/// Initialize an array or slice by cloning an initializer for each element.
///
/// ```rust
/// # use in_place_init::{Init, PinInit};
///
/// let bx: Box<[usize; 3]> = in_place_init::new_boxed(
///     in_place_init::array_repeat(0)
/// );
/// assert_eq!(*bx, [0, 0, 0]);
///
/// let bx: Box<[usize]> = in_place_init::new_boxed(
///     in_place_init::array_repeat::<3, _>(1)
/// );
/// assert_eq!(*bx, [1, 1, 1]);
/// ```
#[derive(Clone, Copy)]
pub struct Repeat<I, L: Length> {
    length: L,
    init: I,
}

impl<I> Repeat<I, RuntimeLength> {
    pub fn new_slice(length: usize, init: I) -> Self {
        Self {
            length: RuntimeLength { length },
            init,
        }
    }

    pub fn new_array<const N: usize>(init: I) -> Repeat<I, ConstLength<N>> {
        Repeat {
            length: ConstLength,
            init,
        }
    }
}

unsafe impl<T, L: Length, Extra: Clone, I: Clone + PinInit<T, Extra>> PinInit<[T], Extra>
    for Repeat<I, L>
{
    type Error = I::Error;

    fn metadata(&self) -> usize {
        self.length.length()
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = self.length.length();
        debug_assert_eq!(buf.capacity(), count);
        while buf.len() < count {
            let init = self.init.clone().with_extra(extra.clone());
            // SAFETY: either `init: Init`, or we treat the destination as pinned
            let init = unsafe { init.assert_pinned() };
            // SAFETY: there is excess capacity
            unsafe { buf.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, L: Length, Extra: Clone, I: Clone + Init<T, Extra>> Init<[T], Extra>
    for Repeat<I, L>
{
}

unsafe impl<T, const N: usize, Extra: Clone, I: Clone + Init<T, Extra>> PinInit<[T; N], Extra>
    for Repeat<I, ConstLength<N>>
{
    type Error = I::Error;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; N], extra: Extra) -> Result<(), Self::Error> {
        unsafe { <Self as PinInit<[T], Extra>>::init(self, dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Clone + Init<T, Extra>> Init<[T; N], Extra>
    for Repeat<I, ConstLength<N>>
{
}
