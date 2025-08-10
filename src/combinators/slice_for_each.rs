use crate::{Init, PinInit, PinInitExt, VecExt};

/// Initialize a slice by creating an initializer for each element.
///
/// ```rust
/// # use in_place_init::{Init, PinInit};
///
/// let bx: Box<[usize]> = in_place_init::new_boxed(
///     in_place_init::slice_for_each(3, |idx| idx * 2 + 1)
/// );
/// assert_eq!(*bx, [1, 3, 5]);
/// ```
#[derive(Clone)]
pub struct SliceForEach<F> {
    count: usize,
    func: F,
}

impl<F> SliceForEach<F> {
    pub fn new(count: usize, func: F) -> Self {
        Self { count, func }
    }
}

unsafe impl<T, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I> PinInit<[T], Extra>
    for SliceForEach<F>
{
    type Error = I::Error;

    fn metadata(&self) -> usize {
        self.count
    }

    unsafe fn init(mut self, dst: *mut [T], extra: Extra) -> Result<(), Self::Error> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = self.count;
        debug_assert_eq!(buf.capacity(), self.count);
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
unsafe impl<T, Extra: Clone, I: Init<T, Extra>, F: FnMut(usize) -> I> Init<[T], Extra>
    for SliceForEach<F>
{
}
