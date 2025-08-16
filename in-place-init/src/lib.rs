#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]
#![feature(sized_hierarchy)]
#![feature(never_type)]
#![feature(clone_to_uninit)]
#![feature(doc_auto_cfg)]
#![feature(unsize)]
#![no_std]

extern crate alloc;
use alloc::alloc::Allocator;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
pub use noop_allocator;
use noop_allocator::owning_ref::OwningRef;

use core::alloc::Layout;
use core::clone::CloneToUninit;
use core::marker::{MetaSized, Unsize as UnsizeTrait};
use core::mem::MaybeUninit;
use core::pin::Pin;
use core::ptr::{NonNull, Pointee};

mod combinators;

mod allocation;

mod util;

#[cfg(feature = "macros")]
pub use in_place_init_derive::Init;

/// A trait for pinned in-place initializers.
///
/// # Safety
///
/// See the documentation for [`metadata`][PinInit::metadata] and [`init`][PinInit::init].
pub unsafe trait PinInit<Dst: MetaSized, Error = !, Extra = ()>: Sized {
    /// The pointer metadata for the value that this initializer will create.
    ///
    /// # Safety
    ///
    /// ## Callers
    ///
    /// This function has no preconditions.
    ///
    /// This function may panic or otherwise diverge.
    ///
    /// ## Implementors
    ///
    /// * This method must return the same value (or diverge) each time it is called, if there are not intermediate modifications to `self`,
    ///   similar to [`DerefPure`](core::ops::DerefPure).
    /// * If `Self` implements `PinInit<Dst, Extra>` for additional `Extra`, this function must return the same value (or diverge) in all such implementations.
    ///     * Note the same is not required with respect to `Dst` for types which implement `PinInit<Dst, _>` for multiple `Dst`.
    /// * If `Self` implements `Clone` (or `Copy`), then any clones (or copies) must return the same value.
    /// * If `Dst: Sized`, this function must not diverge or have any observable side-effects.
    /// * If `Dst: Sized`, it is allowed for callers to assume the return value of this method is `()` without calling it.
    fn metadata(&self) -> <Dst as Pointee>::Metadata;

    /// Initialize a `Dst` value into the provided destination.
    ///
    /// `extra` allows for callers to pass in extra data that may only be available after knowing the metadata, e.g. in [`rc_new_cyclic`].
    ///
    /// # Safety
    ///
    /// ## Callers
    ///
    /// * The metadata of `dst` is be the value returned by `self.metadata()`, with no intermediate modifications to `self`.
    ///     * Except if `Dst: Sized`, where it is not required to call `self.metadata()`.
    /// * `dst` is well-aligned for its pointee, and is valid for writes for its pointee's size.
    ///
    /// This function may panic or return `Err(_)`, in which case `*dst` must be treated as uninitialized.
    ///
    /// If this function returns `Ok(())`, then `*dst` should be treated as a fully initialized `Dst`, and must be treated as pinned (unless `Self` additionally implements `Init<Dst, Extra>`).
    ///
    /// ## Implementors
    ///
    /// If this function returns `Ok(())`, then `*dst` must be a fully initialized `Dst`.
    ///
    /// If this function panics or returns `Err(_)`, then it should drop any partially-initialized parts of the destination.
    /// This is not a safety requirement, but failing to do so may cause resource leaks.
    unsafe fn init(self, dst: *mut Dst, extra: Extra) -> Result<(), Error>;
}

/// A trait for non-pinned in-place initializers.
///
/// # Safety
///
/// See [`PinInit`].
///
/// [`PinInit::init`]'s caller requirements are relaxed to not necessarily treat `*dst` as pinned.
pub unsafe trait Init<Dst: MetaSized, Error = !, Extra = ()>:
    PinInit<Dst, Error, Extra>
{
}

/// Helper methods to construct combinators from intializers.
///
/// The methods start with `init_` to help avoid name collisions.
pub trait PinInitExt<Dst: MetaSized, Error = !, Extra = ()>:
    Sized + PinInit<Dst, Error, Extra>
{
    fn init_map_err<E, F: FnOnce(Error) -> E>(self, func: F) -> MapErr<Dst, Error, F, Self> {
        MapErr::new(func, self)
    }

    fn init_map_extra<E, F: FnOnce(E) -> Result<Extra, Error>>(
        self,
        func: F,
    ) -> MapExtra<Dst, F, Self> {
        MapExtra::new(func, self)
    }

    fn init_ignore_extra(self) -> IgnoreExtra<Dst, Self> {
        IgnoreExtra::new(self)
    }

    fn init_with_extra(self, extra: Extra) -> WithExtra<Dst, Extra, Self> {
        WithExtra::new(self, extra)
    }

    fn init_chain<I2: PinInit<Dst, Error, Extra>>(self, init2: I2) -> Chain<Self, I2> {
        Chain::new(self, init2)
    }

    /// Assert that `self` will be used in a way that respects pinning,
    unsafe fn init_assert_pinned(self) -> AssertPinned<Dst, Error, Extra, Self> {
        unsafe { AssertPinned::new_unchecked(self) }
    }

    /// Allow using `self` as an `Init` safely, because `Dst: Unpin`
    fn init_assert_unpin(self) -> AssertPinned<Dst, Error, Extra, Self>
    where
        Dst: Unpin,
    {
        AssertPinned::new_unpin(self)
    }
}
impl<Dst: MetaSized, Extra, I: PinInit<Dst, Extra>> PinInitExt<Dst, Extra> for I {}

// Simple initializers

/// Initialize a place by writing an existing value.
unsafe impl<T, Error> PinInit<T, Error> for T {
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), Error> {
        unsafe {
            dst.write(self);
        }
        Ok(())
    }
}
unsafe impl<T> Init<T> for T {}

/// Initialize a place by writing an existing value.
unsafe impl<T, E> PinInit<T, E> for Result<T, E> {
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), E> {
        let this = self?;
        unsafe {
            dst.write(this);
        }
        Ok(())
    }
}
unsafe impl<T, E> Init<T, E> for Result<T, E> {}

/// Initialize a slice with an array of a given length.
unsafe impl<T, Error, const N: usize> PinInit<[T], Error> for [T; N] {
    fn metadata(&self) -> usize {
        N
    }
    unsafe fn init(self, dst: *mut [T], _: ()) -> Result<(), Error> {
        debug_assert_eq!(dst.len(), N);
        unsafe {
            dst.cast::<Self>().write(self);
        }
        Ok(())
    }
}
unsafe impl<T, Error, const N: usize> Init<[T], Error> for [T; N] {}

/// Initialize a slice by cloning from an array of a given length.
unsafe impl<T: Clone, Error, const N: usize> PinInit<[T], Error> for &[T; N] {
    fn metadata(&self) -> usize {
        N
    }
    unsafe fn init(self, dst: *mut [T], _: ()) -> Result<(), Error> {
        // SAFETY: discharged to caller
        unsafe { <&[T] as PinInit<[T], Error>>::init(self, dst, ()) }
    }
}
unsafe impl<T: Clone, Error, const N: usize> Init<[T], Error> for &[T; N] {}

/// Initialize a place by cloning an existing value.
unsafe impl<T: MetaSized + CloneToUninit, Error> PinInit<T, Error> for &T {
    fn metadata(&self) -> <T as Pointee>::Metadata {
        core::ptr::metadata::<T>(*self)
    }
    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), Error> {
        unsafe {
            T::clone_to_uninit(self, dst.cast());
        }
        Ok(())
    }
}
unsafe impl<T: MetaSized + CloneToUninit, Error> Init<T, Error> for &T {}

/// Initialize a place by moving an existing value from a `Box`.
unsafe impl<T: MetaSized, Error, A: Allocator> PinInit<T, Error> for Box<T, A> {
    fn metadata(&self) -> <T as Pointee>::Metadata {
        core::ptr::metadata::<T>(&**self)
    }
    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), Error> {
        let layout = Layout::for_value::<T>(&*self);
        let (src, alloc) = Box::into_raw_with_allocator(self);
        unsafe {
            core::ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), layout.size());

            // Drop the pointee if deallocating/dropping the allocator panics.
            let guard = noop_allocator::owning_ref::from_raw(dst);

            if layout.size() > 0 {
                alloc.deallocate(NonNull::new(src.cast()).unwrap(), layout);
            }
            drop(alloc);
            core::mem::forget(guard);
        }
        Ok(())
    }
}
unsafe impl<T: MetaSized, A: Allocator> Init<T> for Box<T, A> {}

/// Initialize a slice by moving elements from a `Vec`.
unsafe impl<T, Error, A: Allocator> PinInit<[T], Error> for Vec<T, A> {
    fn metadata(&self) -> usize {
        self.len()
    }

    unsafe fn init(mut self, dst: *mut [T], _: ()) -> Result<(), Error> {
        let count = self.len();
        unsafe {
            self.set_len(0);
            core::ptr::copy_nonoverlapping::<T>(self.as_ptr(), dst.cast(), count);

            // Drop the pointee if deallocating/dropping the empty vec panics.
            let guard = noop_allocator::owning_ref::from_raw(dst);
            drop(self);
            core::mem::forget(guard);
        }
        Ok(())
    }
}
unsafe impl<T, Error, A: Allocator> Init<[T], Error> for Vec<T, A> {}

/// Initialize a slice by cloning elements from a `Vec`.
unsafe impl<T: Clone, Error, A: Allocator> PinInit<[T], Error> for &Vec<T, A> {
    fn metadata(&self) -> usize {
        self.len()
    }

    unsafe fn init(self, dst: *mut [T], _: ()) -> Result<(), Error> {
        // SAFETY: discharged to caller
        unsafe { <&[T] as PinInit<[T], Error>>::init(&**self, dst, ()) }
    }
}
unsafe impl<T: Clone, Error, A: Allocator> Init<[T], Error> for &Vec<T, A> {}

/// Initialize a `str` slice by copying from a `String`.
unsafe impl<Error> PinInit<str, Error> for &String {
    fn metadata(&self) -> usize {
        self.len()
    }

    unsafe fn init(self, dst: *mut str, _: ()) -> Result<(), Error> {
        // SAFETY: discharged to caller
        unsafe { <&str as PinInit<str, Error>>::init(&*self, dst, ()) }
    }
}
unsafe impl<Error> Init<str, Error> for &String {}

// Initializer combinators

pub use combinators::with::With;
pub fn with<T, F>(func: F) -> With<T, F> {
    With::new(func)
}

pub use combinators::try_with::TryWith;
pub fn try_with<T, F>(func: F) -> TryWith<T, F> {
    TryWith::new(func)
}

pub use combinators::fail::Fail;
pub fn fail<T, E>(err: E) -> Fail<T, E>
where
    <T as Pointee>::Metadata: Default,
{
    Fail::new(err)
}

pub use combinators::map_err::MapErr;
pub fn map_err<T: MetaSized, E1, F, I>(func: F, init: I) -> MapErr<T, E1, F, I> {
    MapErr::new(func, init)
}

pub use combinators::assert_pinned::AssertPinned;
pub unsafe fn assert_pinned<T, Error, Extra, I: PinInit<T, Error, Extra>>(
    init: I,
) -> AssertPinned<T, Error, Extra, I> {
    // SAFETY: discharged to caller
    unsafe { AssertPinned::new_unchecked(init) }
}

pub struct FromIter<I: ExactSizeIterator> {
    iter: I,
}

#[derive(Debug)]
pub enum InitFromIterError {
    TooShort,
}

impl<I: ExactSizeIterator> FromIter<I> {
    pub fn new(iter: I) -> Self {
        Self { iter }
    }
}

unsafe impl<T, I: ExactSizeIterator<Item = T>, Extra> PinInit<[T], InitFromIterError, Extra>
    for FromIter<I>
{
    fn metadata(&self) -> usize {
        self.iter.len()
    }

    unsafe fn init(mut self, dst: *mut [T], _: Extra) -> Result<(), InitFromIterError> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = self.iter.len();
        debug_assert_eq!(dst.len(), count);
        while buf.len() < count {
            let Some(item) = self.iter.next() else {
                return Err(InitFromIterError::TooShort);
            };
            // SAFETY: there is excess capacity
            unsafe { buf.push_emplace_within_capacity_unchecked(item) };
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, I: ExactSizeIterator<Item = T>, Extra> Init<[T], InitFromIterError, Extra>
    for FromIter<I>
{
}

pub use combinators::repeat::Repeat;
pub fn array_repeat<const N: usize, I>(init: I) -> Repeat<I, ConstLength<N>> {
    Repeat::new_array(init)
}
pub fn slice_repeat<I>(length: usize, init: I) -> Repeat<I, RuntimeLength> {
    Repeat::new_slice(length, init)
}

pub use combinators::for_each::ForEach;
pub fn array_for_each<const N: usize, F>(func: F) -> ForEach<F, ConstLength<N>> {
    ForEach::new_array(func)
}
pub fn slice_for_each<F>(length: usize, func: F) -> ForEach<F, RuntimeLength> {
    ForEach::new_slice(length, func)
}

pub use combinators::for_each_with::ForEachWith;
pub fn array_for_each_with<const N: usize, F>(func: F) -> ForEachWith<F, ConstLength<N>> {
    ForEachWith::new_array(func)
}
pub fn slice_for_each_with<F>(length: usize, func: F) -> ForEachWith<F, RuntimeLength> {
    ForEachWith::new_slice(length, func)
}

pub use combinators::chain::Chain;
pub fn chain<I1, I2>(init1: I1, init2: I2) -> Chain<I1, I2> {
    Chain::new(init1, init2)
}

pub use combinators::ignore_extra::IgnoreExtra;
pub fn ignore_extra<T: MetaSized, I>(init: I) -> IgnoreExtra<T, I> {
    IgnoreExtra::new(init)
}

pub use combinators::map_extra::MapExtra;
pub fn map_extra<T: MetaSized, F, I>(func: F, init: I) -> MapExtra<T, F, I> {
    MapExtra::new(func, init)
}

pub use combinators::with_extra::WithExtra;
pub fn with_extra<T: MetaSized, I, Extra>(init: I, extra: Extra) -> WithExtra<T, Extra, I> {
    WithExtra::new(init, extra)
}

pub use combinators::uninit::Uninit;
pub fn uninit<T>() -> Uninit<MaybeUninit<T>> {
    Uninit::new()
}
pub use combinators::zeroed::Zeroed;
pub fn zeroed<T>() -> Zeroed<MaybeUninit<T>> {
    Zeroed::new()
}

pub use combinators::for_type::ForType;
pub fn for_type<T: MetaSized, I>(init: I) -> ForType<T, I> {
    ForType::new(init)
}

pub use combinators::for_slice::ForSlice;
pub fn for_slice<const N: usize, I>(init: I) -> ForSlice<I, N> {
    ForSlice::new(init)
}

pub use combinators::then::Then;
pub fn then<T, I, F>(init: I, func: F) -> Then<T, I, F> {
    Then::new(init, func)
}
pub use combinators::then_pinned::ThenPinned;
pub fn then_pinned<T, I, F>(init: I, func: F) -> ThenPinned<T, I, F> {
    ThenPinned::new(init, func)
}

pub use combinators::as_bytes::AsBytes;
pub fn as_bytes<I>(init: I) -> AsBytes<I> {
    AsBytes::new(init)
}
pub use combinators::as_utf8::AsUtf8;
pub fn as_utf8<I>(init: I) -> AsUtf8<I> {
    AsUtf8::new(init)
}

pub use combinators::flatten::Flatten;
pub fn flatten<T, const N: usize, const M: usize, const P: usize, I>(
    init: I,
) -> Flatten<[[T; N]; M], [T; P], I> {
    const {
        assert!(usize::strict_mul(N, M) == P, "array length mismatch");
    }
    Flatten::new(init)
}
pub fn try_flatten<T, const N: usize, const M: usize, const P: usize, I>(
    init: I,
) -> Result<Flatten<[[T; N]; M], [T; P], I>, I> {
    Flatten::try_new(init)
}
pub fn flatten_slice<T, const N: usize, I>(init: I) -> Flatten<[[T; N]], [T], I> {
    Flatten::new_slice(init)
}

pub use combinators::unsize::Unsize;
pub fn unsize<T: UnsizeTrait<Dst>, Dst: MetaSized, I>(init: I) -> Unsize<T, Dst, I> {
    Unsize::new(init)
}

// Allocation and initialization

pub use allocation::Builder;

pub use allocation::boxed::{new_boxed, new_pinned, try_new_boxed, try_new_pinned};
pub use allocation::boxed::{new_boxed_in, new_pinned_in, try_new_boxed_in, try_new_pinned_in};

pub use allocation::vec::{VecExt, new_vec, try_new_vec};

pub use allocation::string::{StringExt, new_string, try_new_string};

use crate::util::{ConstLength, RuntimeLength};
pub use allocation::rc::{rc_new, rc_new_pinned, try_rc_new, try_rc_new_pinned};
pub use allocation::rc::{
    rc_new_cyclic, rc_new_cyclic_pinned, try_rc_new_cyclic, try_rc_new_cyclic_pinned,
};

/// Initialize a `MaybeUninit<T>` and return a reference to the newly initialized slot.
///
/// Code that receives the mutable reference returned by this function needs to keep in mind
/// that the destructor is not run for the inner data if the MaybeUninit leaves scope without
/// a call to [`MaybeUninit::assume_init`], [`MaybeUninit::assume_init_drop`], or similar. See
/// the docs for [`MaybeUninit::write`] for more information.
pub fn try_initialize<T, Error>(
    slot: &mut MaybeUninit<T>,
    init: impl Init<T, Error>,
) -> Result<&mut T, Error> {
    // SAFETY: `slot` is uniquely borrowed, so it is valid for writes
    unsafe {
        init.init(slot.as_mut_ptr(), ())?;
    }
    // SAFETY: we just initialized `slot`
    unsafe { Ok(slot.assume_init_mut()) }
}

/// Initialize a `MaybeUninit<T>` and return a reference to the newly initialized slot.
///
/// Code that receives the mutable reference returned by this function needs to keep in mind
/// that the destructor is not run for the inner data if the MaybeUninit leaves scope without
/// a call to [`MaybeUninit::assume_init`], [`MaybeUninit::assume_init_drop`], or similar. See
/// the docs for [`MaybeUninit::write`] for more information.
pub fn initialize<T>(slot: &mut MaybeUninit<T>, init: impl Init<T>) -> &mut T {
    try_initialize(slot, init).unwrap_or_else(|e| match e {})
}

/// Initialize a `MaybeUninit<T>` and return a owning reference to the newly initialized slot.
pub fn try_initialize_owned<T, Error>(
    slot: &mut MaybeUninit<T>,
    init: impl Init<T, Error>,
) -> Result<OwningRef<'_, T>, Error> {
    // SAFETY: `slot` is uniquely borrowed, so it is valid for writes
    unsafe {
        init.init(slot.as_mut_ptr(), ())?;
    }
    // SAFETY: we just initialized `slot`
    unsafe { Ok(noop_allocator::owning_ref::from_maybeuninit(slot)) }
}

/// Initialize a `MaybeUninit<T>` and return a owning reference to the newly initialized slot.
pub fn initialize_owned<T>(slot: &mut MaybeUninit<T>, init: impl Init<T>) -> OwningRef<'_, T> {
    try_initialize_owned(slot, init).unwrap_or_else(|e| match e {})
}

/// Initialize a `MaybeUninit<T>` and return a reference to the newly initialized slot.
///
/// Code that receives the mutable reference returned by this function needs to keep in mind
/// that the destructor is not run for the inner data if the MaybeUninit leaves scope without
/// a call to [`MaybeUninit::assume_init`], [`MaybeUninit::assume_init_drop`], or similar. See
/// the docs for [`MaybeUninit::write`] for more information.
pub fn try_initialize_pinned<T, Error>(
    slot: &'static mut MaybeUninit<T>,
    init: impl PinInit<T, Error>,
) -> Result<Pin<&'static mut T>, Error> {
    // SAFETY: `slot` is uniquely borrowed, so it is valid for writes
    unsafe {
        init.init(slot.as_mut_ptr(), ())?;
    }
    // SAFETY: we just initialized `slot`
    Ok(Pin::static_mut(unsafe { slot.assume_init_mut() }))
}

/// Initialize a `MaybeUninit<T>` and return a reference to the newly initialized slot.
///
/// Code that receives the mutable reference returned by this function needs to keep in mind
/// that the destructor is not run for the inner data if the MaybeUninit leaves scope without
/// a call to [`MaybeUninit::assume_init`], [`MaybeUninit::assume_init_drop`], or similar. See
/// the docs for [`MaybeUninit::write`] for more information.
pub fn initialize_pinned<T>(
    slot: &'static mut MaybeUninit<T>,
    init: impl PinInit<T>,
) -> Pin<&'static mut T> {
    try_initialize_pinned(slot, init).unwrap_or_else(|e| match e {})
}

/// Initialize a `MaybeUninit<T>` and return a owning reference to the newly initialized slot.
pub fn try_initialize_pinned_owned<T, Error>(
    slot: &'static mut MaybeUninit<T>,
    init: impl PinInit<T, Error>,
) -> Result<Pin<OwningRef<'static, T>>, Error> {
    // SAFETY: `slot` is uniquely borrowed, so it is valid for writes
    unsafe {
        init.init(slot.as_mut_ptr(), ())?;
    }
    // SAFETY: we just initialized `slot`
    Ok(Box::into_pin(unsafe {
        noop_allocator::owning_ref::from_maybeuninit(slot)
    }))
}

/// Initialize a `MaybeUninit<T>` and return a owning reference to the newly initialized slot.
pub fn initialize_pinned_owned<T>(
    slot: &'static mut MaybeUninit<T>,
    init: impl PinInit<T>,
) -> Pin<OwningRef<'static, T>> {
    try_initialize_pinned_owned(slot, init).unwrap_or_else(|e| match e {})
}
