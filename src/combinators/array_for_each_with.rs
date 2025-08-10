use crate::{Init, PinInit, VecExt};

/// Initialize an array or slice by creating an initializer for each element with extra information provided by the caller.
///
/// For example, [`rc_new_cyclic`][crate::rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// This is similar to using [`With`](crate::With) within [`ArrayForEach`](crate::ArrayForEach),
/// but has better interactions with borrowing in some cases. See [`SliceForEachWith`](crate::SliceForEachWith)
/// for information about those cases.
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// #[derive(Debug)]
/// struct Foo {
///     idx: usize,
///     weak: Weak<[Foo; 3]>,
/// }
///
/// let rc: Rc<[Foo; 3]> = in_place_init::rc_new_cyclic(
///     in_place_init::array_for_each_with::<_, 3>(|idx, weak| Foo { idx, weak })
/// );
/// let rc2 = rc[0].weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
///
/// let rc: Rc<[Foo; 3]> = in_place_init::rc_new_cyclic(
///     in_place_init::array_for_each::<_, 3>(|idx| in_place_init::with(move |weak| Foo { idx, weak }))
/// );
/// let rc2 = rc[0].weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
/// ```
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// # type Token = usize;
/// #[derive(Debug)]
/// struct Foo {
///     idx: usize,
///     weak: Weak<[Foo]>,
///     id: Token,
/// }
///
/// # let mut counter = 0;
/// # struct NotCopy(u32);
/// # let not_copy = NotCopy(0);
/// let mut next_id: Box<dyn FnMut() -> Token> = Box::new(|| {
/// // ...
/// # let _a = &not_copy;
/// # let token = counter;
/// # counter += 1;
/// # token
/// });
///
/// let rc: Rc<[Foo]> = in_place_init::rc_new_cyclic(
///     in_place_init::slice_for_each_with(3, move |idx, weak| Foo { idx, weak, id: (next_id)() })
/// );
/// let rc2 = rc[0].weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
/// ```
pub struct ArrayForEachWith<F, const N: usize> {
    func: F,
}

impl<F, const N: usize> ArrayForEachWith<F, N> {
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

unsafe impl<T, const N: usize, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I>
    PinInit<[T], Extra> for ArrayForEachWith<F, N>
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
            let init = (self.func)(buf.len(), extra.clone());
            // SAFETY: there is excess capacity
            unsafe { buf.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I>
    Init<[T], Extra> for ArrayForEachWith<F, N>
{
}

unsafe impl<T, const N: usize, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I>
    PinInit<[T; N], Extra> for ArrayForEachWith<F, N>
{
    type Error = I::Error;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; N], extra: Extra) -> Result<(), Self::Error> {
        unsafe { <Self as PinInit<[T], Extra>>::init(self, dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I>
    Init<[T; N], Extra> for ArrayForEachWith<F, N>
{
}
