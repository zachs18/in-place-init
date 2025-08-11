use core::{marker::MetaSized, pin::Pin};

use alloc::alloc::{Allocator, Global};
pub(crate) use alloc::rc::{Rc, Weak};

use crate::{Init, PinInit};

pub(crate) trait MaybeWeakExtra<T: MetaSized, A: Allocator, InputExtra = ()>: Sized {
    type OutputExtra: Sized;
    fn make(weak: &Weak<T, A>, input: InputExtra) -> Self::OutputExtra;
}

pub(crate) struct WeakExtra;

impl<T: MetaSized, A: Allocator + Clone> MaybeWeakExtra<T, A> for WeakExtra {
    type OutputExtra = Weak<T, A>;

    fn make(weak: &Weak<T, A>, _: ()) -> Weak<T, A> {
        weak.clone()
    }
}

pub(crate) struct NonWeakExtra;

impl<T: MetaSized, A: Allocator, Extra> MaybeWeakExtra<T, A, Extra> for NonWeakExtra {
    type OutputExtra = Extra;

    fn make(_weak: &Weak<T, A>, input: Extra) -> Extra {
        input
    }
}

pub(crate) struct WithWeakExtra;

impl<T: MetaSized, A: Allocator + Clone, Extra> MaybeWeakExtra<T, A, Extra> for WithWeakExtra {
    type OutputExtra = (Weak<T, A>, Extra);
    fn make(weak: &Weak<T, A>, extra: Extra) -> (Weak<T, A>, Extra) {
        (weak.clone(), extra)
    }
}

/// # Safety
///
/// Either `init` implements `Init`, or the returned `Rc` and `rc::Weak`s passed as extras (if any)
/// are treated as pinned.
///
/// Also, this assumes the layout of `Rc`'s heap allocation, which is not stable.
pub(crate) unsafe fn rc_new_base_impl<
    T: MetaSized,
    E,
    A: Allocator,
    InputExtra,
    WeakExtra: MaybeWeakExtra<T, A, InputExtra>,
>(
    init: impl PinInit<T, WeakExtra::OutputExtra, Error = E>,
    alloc: A,
    extra: InputExtra,
) -> Result<Rc<T, A>, E> {
    // NOTE: this is unsound; it relies on the unstable layout of Rc's heap allocation
    use core::alloc::Layout;
    use core::cell::Cell;
    let metadata = init.metadata();
    let value_layout = unsafe {
        Layout::for_value_raw::<T>(core::ptr::from_raw_parts(core::ptr::null::<()>(), metadata))
    };

    #[repr(C)]
    struct RcCounts {
        strong: Cell<usize>,
        weak: Cell<usize>,
    }

    let (layout, offset) = Layout::new::<RcCounts>().extend(value_layout).unwrap();

    let base_ptr = if layout.size() == 0 {
        layout.dangling()
    } else {
        match alloc.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(_) => alloc::alloc::handle_alloc_error(layout),
        }
    };

    unsafe {
        base_ptr.cast::<RcCounts>().write(RcCounts {
            strong: 0.into(),
            weak: 1.into(),
        });
    }

    let value_ptr =
        core::ptr::from_raw_parts_mut::<T>(unsafe { base_ptr.byte_add(offset).as_ptr() }, metadata);

    let weak = unsafe { Weak::from_raw_in(value_ptr, alloc) };

    let extra = WeakExtra::make(&weak, extra);

    match unsafe { init.init(value_ptr, extra) } {
        Ok(()) => Ok(unsafe {
            let (_, alloc) = Weak::into_raw_with_allocator(weak);
            base_ptr.cast::<RcCounts>().as_ref().strong.set(1);

            Rc::from_raw_in(value_ptr, alloc)
        }),
        // dropping `weak` in this branch deallocates
        Err(err) => Err(err),
    }
}

pub fn try_rc_new<T: MetaSized, E>(init: impl Init<T, Error = E>) -> Result<Rc<T>, E> {
    // Safety: `init` implements `Init<T>`
    unsafe { rc_new_base_impl::<T, E, Global, (), NonWeakExtra>(init, Global, ()) }
}
pub fn rc_new<T: MetaSized>(init: impl Init<T, Error = !>) -> Rc<T> {
    try_rc_new(init).unwrap_or_else(|e| match e {})
}
pub fn try_rc_new_pinned<T: MetaSized, E>(
    init: impl PinInit<T, Error = E>,
) -> Result<Pin<Rc<T>>, E> {
    // Safety: the `Rc` is immediately pinned
    let rc = unsafe { rc_new_base_impl::<T, E, Global, (), NonWeakExtra>(init, Global, ()) }?;
    // SAFETY: No other code has had access to this `Rc`.
    Ok(unsafe { Pin::new_unchecked(rc) })
}
pub fn rc_new_pinned<T: MetaSized>(init: impl PinInit<T, Error = !>) -> Pin<Rc<T>> {
    try_rc_new_pinned(init).unwrap_or_else(|e| match e {})
}

/// Create a new `Rc<T>` while giving you a `Weak<T>` to the allocation.
pub fn try_rc_new_cyclic<T: MetaSized, E>(
    init: impl Init<T, Weak<T>, Error = E>,
) -> Result<Rc<T>, E> {
    // Safety: `init` implements `Init<T>`
    unsafe { rc_new_base_impl::<T, E, Global, (), WeakExtra>(init, Global, ()) }
}

/// Create a new `Rc<T>` while giving you a `Weak<T>` to the allocation.
pub fn rc_new_cyclic<T: MetaSized>(init: impl Init<T, Weak<T>, Error = !>) -> Rc<T> {
    try_rc_new_cyclic(init).unwrap_or_else(|e| match e {})
}

/// Create a new pinned `Rc<T>` while giving you a `Weak<T>` to the allocation.
///
/// # Safety
///
/// `init` must treat the `Weak`s passed to it as pinned.
pub unsafe fn try_rc_new_cyclic_pinned<T: MetaSized, E>(
    init: impl PinInit<T, Weak<T>, Error = E>,
) -> Result<Pin<Rc<T>>, E> {
    // Safety: the `Rc` is immediately pinned
    let rc = unsafe { rc_new_base_impl::<T, E, Global, (), WeakExtra>(init, Global, ()) }?;
    // SAFETY: The only code that has had access to this Rc has had access as `Weak<T>`,
    // which the caller must ensure are treated as pinned.
    Ok(unsafe { Pin::new_unchecked(rc) })
}

/// Create a new pinned `Rc<T>` while giving you a `Weak<T>` to the allocation.
///
/// # Safety
///
/// `init` must treat the `Weak`s passed to it as pinned.
pub unsafe fn rc_new_cyclic_pinned<T: MetaSized>(
    init: impl PinInit<T, Weak<T>, Error = !>,
) -> Pin<Rc<T>> {
    // SAFETY: discharged to caller
    unsafe { try_rc_new_cyclic_pinned(init).unwrap_or_else(|e| match e {}) }
}
