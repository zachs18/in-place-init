use core::{marker::MetaSized, pin::Pin};

use alloc::boxed::Box;

use crate::{Init, PinInit};

/// # Safety
///
/// Either `init` implements `Init<T>`, or the returned `Box` is immediately pinned.
unsafe fn new_impl<T: ?Sized + MetaSized, E>(
    init: impl PinInit<T, Error = E>,
) -> Result<Box<T>, E> {
    use core::alloc::Layout;
    let metadata = init.metadata();
    let layout = unsafe {
        Layout::for_value_raw::<T>(core::ptr::from_raw_parts(core::ptr::null::<()>(), metadata))
    };

    let ptr = if layout.size() == 0 {
        layout.dangling().as_ptr()
    } else {
        unsafe { alloc::alloc::alloc(layout) }
    };
    if ptr.is_null() {
        alloc::alloc::handle_alloc_error(layout);
    }

    struct DeallocOnDrop {
        ptr: *mut u8,
        layout: Layout,
    }
    impl Drop for DeallocOnDrop {
        fn drop(&mut self) {
            unsafe {
                alloc::alloc::dealloc(self.ptr, self.layout);
            }
        }
    }
    let guard = DeallocOnDrop { ptr, layout };

    let ptr = core::ptr::from_raw_parts_mut(ptr, metadata);

    // TODO: deallocate on panic
    match unsafe { init.init(ptr, ()) } {
        Ok(()) => {
            core::mem::forget(guard);
            Ok(unsafe { Box::from_raw(ptr) })
        }
        Err(err) => {
            drop(guard);
            Err(err)
        }
    }
}

pub fn try_new_boxed<T: ?Sized + MetaSized, E>(init: impl Init<T, Error = E>) -> Result<Box<T>, E> {
    // Safety: `init` implements `Init<T>`
    unsafe { new_impl(init) }
}
pub fn try_new_pinned<T: ?Sized + MetaSized, E>(
    init: impl PinInit<T, Error = E>,
) -> Result<Pin<Box<T>>, E> {
    // Safety: the box is immediately pinned
    unsafe { new_impl(init).map(Box::into_pin) }
}
pub fn new_boxed<T: ?Sized + MetaSized>(init: impl Init<T, Error = !>) -> Box<T> {
    try_new_boxed(init).unwrap()
}
pub fn new_pinned<T: ?Sized + MetaSized>(init: impl PinInit<T, Error = !>) -> Pin<Box<T>> {
    try_new_pinned(init).unwrap()
}
