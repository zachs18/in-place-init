use core::marker::PhantomData;

use crate::{Init, PinInit};

/// Initialize a sized place by creating an initializer with extra information provided by the caller.
///
/// For example, [`rc_new_cyclic`][crate::rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// #[derive(Debug)]
/// struct Foo {
///     weak: Weak<Foo>,
/// }
///
/// let rc = in_place_init::rc_new_cyclic(in_place_init::with(|weak| Foo { weak }));
/// let rc2 = rc.weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
/// ```
pub struct With<T, F> {
    /// We need to mention `T` so the compiler knows this can't overlap with `impl Init<T> for T`.
    variance: PhantomData<fn() -> T>,
    func: F,
}

impl<T, F: Clone> Clone for With<T, F> {
    fn clone(&self) -> Self {
        Self {
            variance: PhantomData,
            func: self.func.clone(),
        }
    }
}

impl<T, F> With<T, F> {
    pub fn new(func: F) -> Self {
        Self {
            variance: PhantomData,
            func,
        }
    }
}

unsafe impl<T, Error, Extra, I: PinInit<T, Error>, F: FnOnce(Extra) -> I> PinInit<T, Error, Extra>
    for With<T, F>
{
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Error> {
        let init = (self.func)(extra);
        unsafe { init.init(dst, ()) }
    }
}
unsafe impl<T, Error, Extra, I: Init<T, Error>, F: FnOnce(Extra) -> I> Init<T, Error, Extra>
    for With<T, F>
{
}
