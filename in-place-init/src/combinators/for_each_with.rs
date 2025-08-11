use crate::{
    Init, PinInit, VecExt,
    util::{ConstLength, Length, RuntimeLength},
};

/// Initialize an array or slice by creating an initializer for each element with extra information provided by the caller.
///
/// For example, [`rc_new_cyclic`][crate::rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// This is similar to using [`With`](crate::With) within [`ForEach`](crate::ForEach),
/// but has better interactions with borrowing in some cases.
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// #[derive(Debug)]
/// struct Foo {
///     idx: usize,
///     weak: Weak<[Foo]>,
/// }
///
/// let rc: Rc<[Foo]> = in_place_init::rc_new_cyclic(
///     in_place_init::slice_for_each_with(3, |idx, weak| Foo { idx, weak })
/// );
/// let rc2 = rc[0].weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
///
/// let rc: Rc<[Foo]> = in_place_init::rc_new_cyclic(
///     in_place_init::slice_for_each(3, |idx| in_place_init::with(move |weak| Foo { idx, weak }))
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
///
/// ```rust,compile_fail
/// # use std::rc::{Weak, Rc};
/// # type Token = usize;
/// # #[derive(Debug)]
/// # struct Foo {
/// #     idx: usize,
/// #     weak: Weak<[Foo]>,
/// #     id: Token,
/// # }
/// #
/// # let mut counter = 0;
/// # let mut next_id: Box<dyn FnMut() -> Token> = Box::new(|| {
/// #   let token = counter;
/// #   counter += 1;
/// #   token
/// # });
/// let rc: Rc<[Foo]> = in_place_init::rc_new_cyclic(
///     in_place_init::slice_for_each(
///         3,
///         |idx| in_place_init::with(
///             move |weak| Foo { idx, weak, id: next_id() }
///             // ^ With this `move`, `next_id` is moved into the inner closure,
///             // so the outer closure can only be called once.
///             // Without the `move`, the inner closure borrows `next_id`,
///             // so the outer closure gives a "captured variable cannot
///             // escape `FnMut` closure body" error
///         ),
///     )
/// );
/// let rc2 = rc[0].weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
/// ```
#[derive(Clone)]
pub struct ForEachWith<F, L: Length> {
    length: L,
    func: F,
}

impl<F> ForEachWith<F, RuntimeLength> {
    pub fn new_slice(length: usize, func: F) -> Self {
        Self {
            length: RuntimeLength { length },
            func,
        }
    }

    pub fn new_array<const N: usize>(func: F) -> ForEachWith<F, ConstLength<N>> {
        ForEachWith {
            length: ConstLength,
            func,
        }
    }
}

unsafe impl<T, L: Length, Extra: Clone, I: PinInit<T>, F: FnMut(usize, Extra) -> I>
    PinInit<[T], Extra> for ForEachWith<F, L>
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
            let init = (self.func)(idx, extra);
            // SAFETY: either `init: Init`, or we treat the destination as pinned
            let init = unsafe { crate::assert_pinned(init) };
            // SAFETY: there is excess capacity
            unsafe { buf.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, L: Length, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I> Init<[T], Extra>
    for ForEachWith<F, L>
{
}

unsafe impl<T, const N: usize, Extra: Clone, I: PinInit<T>, F: FnMut(usize, Extra) -> I>
    PinInit<[T; N], Extra> for ForEachWith<F, ConstLength<N>>
{
    type Error = I::Error;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; N], extra: Extra) -> Result<(), Self::Error> {
        unsafe { <Self as PinInit<[T], Extra>>::init(self, dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I>
    Init<[T; N], Extra> for ForEachWith<F, ConstLength<N>>
{
}
