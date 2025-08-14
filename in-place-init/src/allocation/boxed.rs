use core::{marker::MetaSized, pin::Pin, ptr::NonNull};

use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
};

use crate::{Init, PinInit};

/// # Safety
///
/// Either `init` implements `Init<T, Extra>`, or the returned `Box` is immediately pinned.
pub(super) unsafe fn new_impl<T: MetaSized, Error, A: Allocator, Extra>(
    init: impl PinInit<T, Error, Extra>,
    alloc: A,
    extra: Extra,
) -> Result<Box<T, A>, Error> {
    use core::alloc::Layout;
    let metadata = init.metadata();
    // SAFETY: this is unsound, size could overflow
    // FIXME: should use checked_layout_for_meta if/when that's a thing
    let layout = unsafe {
        Layout::for_value_raw::<T>(core::ptr::from_raw_parts(core::ptr::null::<()>(), metadata))
    };

    let ptr = if layout.size() == 0 {
        layout.dangling()
    } else {
        match alloc.allocate(layout) {
            Ok(ptr) => ptr.cast(),
            Err(_) => alloc::alloc::handle_alloc_error(layout),
        }
    };

    struct DeallocOnDrop<'a, A: Allocator> {
        ptr: NonNull<u8>,
        layout: Layout,
        alloc: &'a A,
    }
    impl<'a, A: Allocator> Drop for DeallocOnDrop<'a, A> {
        fn drop(&mut self) {
            unsafe {
                self.alloc.deallocate(self.ptr, self.layout);
            }
        }
    }
    let guard = DeallocOnDrop {
        ptr,
        layout,
        alloc: &alloc,
    };

    let ptr = core::ptr::from_raw_parts_mut(ptr.as_ptr(), metadata);

    // TODO: deallocate on panic
    match unsafe { init.init(ptr, extra) } {
        Ok(()) => {
            core::mem::forget(guard);
            Ok(unsafe { Box::from_raw_in(ptr, alloc) })
        }
        Err(err) => {
            drop(guard);
            Err(err)
        }
    }
}

pub fn try_new_boxed_in<T: MetaSized, Error, A: Allocator>(
    init: impl Init<T, Error>,
    alloc: A,
) -> Result<Box<T, A>, Error> {
    // Safety: `init` implements `Init<T>`
    unsafe { new_impl(init, alloc, ()) }
}
pub fn try_new_pinned_in<T: MetaSized, Error, A: Allocator + 'static>(
    init: impl PinInit<T, Error>,
    alloc: A,
) -> Result<Pin<Box<T, A>>, Error> {
    // Safety: the box is immediately pinned
    unsafe { new_impl(init, alloc, ()).map(Box::into_pin) }
}
pub fn new_boxed_in<T: MetaSized, A: Allocator>(init: impl Init<T>, alloc: A) -> Box<T, A> {
    try_new_boxed_in(init, alloc).unwrap_or_else(|e| match e {})
}
pub fn new_pinned_in<T: MetaSized, A: Allocator + 'static>(
    init: impl PinInit<T>,
    alloc: A,
) -> Pin<Box<T, A>> {
    try_new_pinned_in(init, alloc).unwrap_or_else(|e| match e {})
}

pub fn try_new_boxed<T: MetaSized, Error>(init: impl Init<T, Error>) -> Result<Box<T>, Error> {
    try_new_boxed_in(init, Global)
}
pub fn try_new_pinned<T: MetaSized, Error>(
    init: impl PinInit<T, Error>,
) -> Result<Pin<Box<T>>, Error> {
    try_new_pinned_in(init, Global)
}
pub fn new_boxed<T: MetaSized>(init: impl Init<T>) -> Box<T> {
    try_new_boxed_in(init, Global).unwrap_or_else(|e| match e {})
}
pub fn new_pinned<T: MetaSized>(init: impl PinInit<T>) -> Pin<Box<T>> {
    try_new_pinned_in(init, Global).unwrap_or_else(|e| match e {})
}
