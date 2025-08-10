use crate::{Init, PinInit, VecExt};

/// Initialize a slice by creating an initializer for each element with extra information provided by the caller.
///
/// For example, [`rc_new_cyclic`][crate::rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// This is similar to using [`With`](crate::With) within [`SliceForEach`](crate::SliceForEach),
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
pub struct SliceForEachWith<F> {
    count: usize,
    func: F,
}

impl<F> SliceForEachWith<F> {
    pub fn new(count: usize, func: F) -> Self {
        Self { count, func }
    }
}

unsafe impl<T, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I> PinInit<[T], Extra>
    for SliceForEachWith<F>
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
            let init = (self.func)(buf.len(), extra.clone());
            // SAFETY: there is excess capacity
            unsafe { buf.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, Extra: Clone, I: Init<T>, F: FnMut(usize, Extra) -> I> Init<[T], Extra>
    for SliceForEachWith<F>
{
}
