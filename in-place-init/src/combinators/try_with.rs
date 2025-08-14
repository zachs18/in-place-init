use core::marker::PhantomData;

use crate::{Init, PinInit};

/// Initialize a sized place by fallibly creating an initializer with extra information provided by the caller.
///
/// The error type of the returned initializer must be the same as the error type of the callback.
/// If necessary, you can change the error type with [`map_err`][crate::map_err] or [`succeed`][crate::succeed].
///
/// For example, [`try_rc_new_cyclic`][crate::try_rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// # use in_place_init::{Init};
/// #[derive(Debug)]
/// struct Foo {
///     weak: Weak<Foo>,
/// }
///
/// fn fallible_initializer(weak: Weak<Foo>) -> impl Init<Foo, Error = u32> {
/// # in_place_init::succeed(Foo { weak })
/// # /*
///     ...
/// # */
/// }
///
/// let rc = in_place_init::try_rc_new_cyclic(in_place_init::try_with(|weak| {
///     Ok(fallible_initializer(weak))
/// })).unwrap();
/// let rc2 = rc.weak.upgrade().unwrap();
/// assert!(Rc::ptr_eq(&rc, &rc2));
/// ```
pub struct TryWith<T, F> {
    /// We need to mention `T` so the compiler knows this can't overlap with `impl Init<T> for T`.
    variance: PhantomData<fn() -> T>,
    func: F,
}

impl<T, F: Clone> Clone for TryWith<T, F> {
    fn clone(&self) -> Self {
        Self {
            variance: PhantomData,
            func: self.func.clone(),
        }
    }
}

impl<T, F> TryWith<T, F> {
    pub fn new(func: F) -> Self {
        Self {
            variance: PhantomData,
            func,
        }
    }
}

unsafe impl<T, Error, Extra, I: PinInit<T, Error>, F: FnOnce(Extra) -> Result<I, Error>>
    PinInit<T, Error, Extra> for TryWith<T, F>
{
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Error> {
        let init = (self.func)(extra)?;
        unsafe { init.init(dst, ()) }
    }
}
unsafe impl<T, Error, Extra, I: Init<T, Error>, F: FnOnce(Extra) -> Result<I, Error>>
    Init<T, Error, Extra> for TryWith<T, F>
{
}
