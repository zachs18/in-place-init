use core::marker::PhantomData;

use crate::{Init, PinInit};

/// Initialize a sized place by fallibly creating an initializer with extra information provided by the caller.
///
/// The error type of the returned initializer must be the same as the error type of the callback.
///
/// For example, [`try_rc_new_cyclic`][crate::try_rc_new_cyclic] provides the `Weak` pointer as extra information
/// to the provided intializer.
///
/// ```rust
/// # use std::rc::{Weak, Rc};
/// # use in_place_init::{Init, PinInit};
/// #[derive(Debug)]
/// struct Foo {
///     weak: Weak<Foo>,
/// }
///
/// fn fallible_initializer(weak: Weak<Foo>) -> impl Init<Foo, Error = u32> {
/// # <Foo as PinInit<Foo>>::map_err(Foo { weak }, |e| match e {})
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

impl<T, F> TryWith<T, F> {
    pub fn new(func: F) -> Self {
        Self {
            variance: PhantomData,
            func,
        }
    }
}

unsafe impl<T, Extra, E, I: PinInit<T, Error = E>, F: FnOnce(Extra) -> Result<I, E>>
    PinInit<T, Extra> for TryWith<T, F>
{
    type Error = E;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, extra: Extra) -> Result<(), Self::Error> {
        let init = (self.func)(extra)?;
        let result = unsafe { init.init(dst, ()) };
        result.map_err(Into::into)
    }
}
unsafe impl<T, Extra, E, I: Init<T, Error = E>, F: FnOnce(Extra) -> Result<I, E>> Init<T, Extra>
    for TryWith<T, F>
{
}
