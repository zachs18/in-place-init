use core::{marker::MetaSized, pin::Pin};

use alloc::rc::{self, Rc};

use crate::{Init, PinInit};

trait MaybeWeakExtra<T: ?Sized + MetaSized> {
    fn make(weak: &rc::Weak<T>) -> Self;
}

impl<T: ?Sized + MetaSized> MaybeWeakExtra<T> for rc::Weak<T> {
    fn make(weak: &rc::Weak<T>) -> Self {
        weak.clone()
    }
}

impl<T: ?Sized + MetaSized> MaybeWeakExtra<T> for () {
    fn make(_weak: &rc::Weak<T>) -> Self {}
}

/// # Safety
///
/// Either `init` implements `Init`, or the returned `Rc` is immediately pinned.
unsafe fn rc_new_base_impl<T: ?Sized + MetaSized, E, CyclicWeak: MaybeWeakExtra<T>>(
    init: impl PinInit<T, CyclicWeak, Error = E>,
) -> Result<Rc<T>, E> {
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
        layout.dangling().as_ptr()
    } else {
        unsafe { alloc::alloc::alloc(layout) }
    };
    if base_ptr.is_null() {
        alloc::alloc::handle_alloc_error(layout);
    }

    unsafe {
        base_ptr.cast::<RcCounts>().write(RcCounts {
            strong: 0.into(),
            weak: 1.into(),
        });
    }

    let value_ptr =
        core::ptr::from_raw_parts_mut::<T>(unsafe { base_ptr.byte_add(offset) }, metadata);

    let weak = unsafe { rc::Weak::from_raw(value_ptr) };

    match unsafe { init.init(value_ptr, MaybeWeakExtra::make(&weak)) } {
        Ok(()) => Ok(unsafe {
            core::mem::forget(weak);
            base_ptr.cast::<RcCounts>().as_ref().unwrap().strong.set(1);

            Rc::from_raw(value_ptr)
        }),
        // dropping `weak` in this branch deallocates
        Err(err) => Err(err),
    }
}

pub fn try_rc_new<T: ?Sized + MetaSized, E>(init: impl Init<T, Error = E>) -> Result<Rc<T>, E> {
    // Safety: `init` implements `Init<T>`
    unsafe { rc_new_base_impl::<T, E, ()>(init) }
}
pub fn rc_new<T: ?Sized + MetaSized>(init: impl Init<T, Error = !>) -> Rc<T> {
    try_rc_new(init).unwrap_or_else(|e| match e {})
}
pub fn try_rc_new_pinned<T: ?Sized + MetaSized, E>(
    init: impl PinInit<T, Error = E>,
) -> Result<Pin<Rc<T>>, E> {
    // Safety: the `Rc` is immediately pinned
    let rc = unsafe { rc_new_base_impl::<T, E, ()>(init) }?;
    // SAFETY: No other code has had access to this `Rc`.
    Ok(unsafe { Pin::new_unchecked(rc) })
}
pub fn rc_new_pinned<T: ?Sized + MetaSized>(init: impl PinInit<T, Error = !>) -> Pin<Rc<T>> {
    try_rc_new_pinned(init).unwrap_or_else(|e| match e {})
}

/// Create a new `Rc<T>` while giving you a `Weak<T>` to the allocation.
pub fn try_rc_new_cyclic<T: ?Sized + MetaSized, E>(
    init: impl Init<T, rc::Weak<T>, Error = E>,
) -> Result<Rc<T>, E> {
    // Safety: `init` implements `Init<T>`
    unsafe { rc_new_base_impl::<T, E, rc::Weak<T>>(init) }
}

/// Create a new `Rc<T>` while giving you a `Weak<T>` to the allocation.
pub fn rc_new_cyclic<T: ?Sized + MetaSized>(init: impl Init<T, rc::Weak<T>, Error = !>) -> Rc<T> {
    try_rc_new_cyclic(init).unwrap_or_else(|e| match e {})
}

/// Create a new pinned `Rc<T>` while giving you a `Weak<T>` to the allocation.
///
/// # Safety
///
/// `init` must treat the `Weak`s passed to it as pinned.
pub unsafe fn try_rc_new_cyclic_pinned<T: ?Sized + MetaSized, E>(
    init: impl PinInit<T, rc::Weak<T>, Error = E>,
) -> Result<Pin<Rc<T>>, E> {
    // Safety: the `Rc` is immediately pinned
    let rc = unsafe { rc_new_base_impl::<T, E, rc::Weak<T>>(init) }?;
    // SAFETY: The only code that has had access to this Rc has had access as `Weak<T>`,
    // which the caller must ensure are treated as pinned.
    Ok(unsafe { Pin::new_unchecked(rc) })
}

/// Create a new pinned `Rc<T>` while giving you a `Weak<T>` to the allocation.
///
/// # Safety
///
/// `init` must treat the `Weak`s passed to it as pinned.
pub unsafe fn rc_new_cyclic_pinned<T: ?Sized + MetaSized>(
    init: impl PinInit<T, rc::Weak<T>, Error = !>,
) -> Pin<Rc<T>> {
    // SAFETY: discharged to caller
    unsafe { try_rc_new_cyclic_pinned(init).unwrap_or_else(|e| match e {}) }
}
