#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(layout_for_ptr)]
#![feature(ptr_metadata, sized_hierarchy, never_type, clone_to_uninit)]
#![no_std]

extern crate alloc;
use alloc::alloc::Allocator;
use alloc::boxed::Box;
use alloc::vec::Vec;

use core::alloc::Layout;
use core::clone::CloneToUninit;
use core::marker::MetaSized;
use core::mem::MaybeUninit;
use core::ptr::{NonNull, Pointee};

mod combinators;

mod allocation;

/// A trait for pinned in-place initializers.
///
/// # Safety
///
/// See the documentation for [`metadata`][PinInit::metadata] and [`init`][PinInit::init].
pub unsafe trait PinInit<Dst: MetaSized, Extra = ()>: Sized {
    type Error;

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
    /// * This method must return the same value (or diverge) each time it is called, if there are not intermediate modifications to `self`.
    /// * If `Dst` is `Sized`, this function must not diverge or have any observable side-effects, i.e. if `Dst: Sized`, it is allowed for callers to assume the return value of this method is `()` without calling it.
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
    unsafe fn init(self, dst: *mut Dst, extra: Extra) -> Result<(), Self::Error>;
}

pub trait PinInitExt<Dst: MetaSized, Extra = ()>: Sized + PinInit<Dst, Extra> {
    fn map_err<E, F: FnOnce(Self::Error) -> E>(self, func: F) -> MapErr<Dst, F, Self> {
        MapErr::new(func, self)
    }

    fn map_extra<E, F: FnOnce(E) -> Extra>(self, func: F) -> MapExtra<Dst, F, Self> {
        MapExtra::new(func, self)
    }

    fn ignore_extra(self) -> IgnoreExtra<Dst, Self> {
        IgnoreExtra::new(self)
    }

    fn with_extra(self, extra: Extra) -> WithExtra<Dst, Self, Extra> {
        WithExtra::new(extra, self)
    }

    fn chain<I2: PinInit<Dst, Extra, Error = Self::Error>>(self, init2: I2) -> Chain<Self, I2> {
        Chain::new(self, init2)
    }
}
impl<Dst: MetaSized, Extra, I: PinInit<Dst, Extra>> PinInitExt<Dst, Extra> for I {}

/// A trait for non-pinned in-place initializers.
///
/// # Safety
///
/// See [`PinInit`].
///
/// [`PinInit::init`]'s caller requirements are relaxed to not necessarily treat `*dst` as pinned.
pub unsafe trait Init<Dst: MetaSized, Extra = ()>: PinInit<Dst, Extra> {}

// Simple initializers

/// Initialize a place by writing an existing value.
unsafe impl<T> PinInit<T> for T {
    type Error = !;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), !> {
        unsafe {
            dst.write(self);
        }
        Ok(())
    }
}
unsafe impl<T> Init<T> for T {}

/// Initialize a place by writing an existing value.
unsafe impl<T, E> PinInit<T> for Result<T, E> {
    type Error = E;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), E> {
        let this = self?;
        unsafe {
            dst.write(this);
        }
        Ok(())
    }
}
unsafe impl<T, E> Init<T> for Result<T, E> {}

/// Initialize a slice with an array of a given length.
unsafe impl<T, const N: usize> PinInit<[T]> for [T; N] {
    type Error = !;
    fn metadata(&self) -> usize {
        N
    }
    unsafe fn init(self, dst: *mut [T], _: ()) -> Result<(), !> {
        debug_assert_eq!(dst.len(), N);
        unsafe {
            dst.cast::<Self>().write(self);
        }
        Ok(())
    }
}
unsafe impl<T, const N: usize> Init<[T]> for [T; N] {}

/// Initialize a place by cloning an existing value.
unsafe impl<T: ?Sized + MetaSized + CloneToUninit> PinInit<T> for &T {
    type Error = !;
    fn metadata(&self) -> <T as Pointee>::Metadata {
        core::ptr::metadata::<T>(*self)
    }
    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), !> {
        unsafe {
            T::clone_to_uninit(self, dst.cast());
        }
        Ok(())
    }
}
unsafe impl<T: ?Sized + MetaSized + CloneToUninit> Init<T> for &T {}

/// Initialize a place by moving an existing value from a `Box`.
unsafe impl<T: ?Sized + MetaSized, A: Allocator> PinInit<T> for Box<T, A> {
    type Error = !;
    fn metadata(&self) -> <T as Pointee>::Metadata {
        core::ptr::metadata::<T>(&**self)
    }
    unsafe fn init(self, dst: *mut T, _: ()) -> Result<(), !> {
        let layout = Layout::for_value::<T>(&*self);
        let (src, alloc) = Box::into_raw_with_allocator(self);
        unsafe {
            core::ptr::copy_nonoverlapping(src.cast::<u8>(), dst.cast::<u8>(), layout.size());

            // Drop the pointee if deallocating/dropping the allocator panics.
            let guard = Box::from_raw_in(dst, noop_allocator::NoopAllocator::new());

            if layout.size() > 0 {
                alloc.deallocate(NonNull::new(src.cast()).unwrap(), layout);
            }
            drop(alloc);
            core::mem::forget(guard);
        }
        Ok(())
    }
}
unsafe impl<T: ?Sized + MetaSized, A: Allocator> Init<T> for Box<T, A> {}

/// Initialize a slice by moving elements from a `Vec`.
unsafe impl<T, A: Allocator> PinInit<[T]> for Vec<T, A> {
    type Error = !;

    fn metadata(&self) -> usize {
        self.len()
    }

    unsafe fn init(mut self, dst: *mut [T], _: ()) -> Result<(), Self::Error> {
        let count = self.len();
        unsafe {
            self.set_len(0);
            core::ptr::copy_nonoverlapping::<T>(self.as_ptr(), dst.cast(), count);

            // Drop the pointee if deallocating/dropping the empty vec panics.
            let guard = Box::from_raw_in(dst, noop_allocator::NoopAllocator::new());
            drop(self);
            core::mem::forget(guard);
        }
        Ok(())
    }
}
unsafe impl<T, A: Allocator> Init<[T]> for Vec<T, A> {}

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

pub use combinators::succeed::Succeed;
pub fn succeed<T, I, E>(init: I) -> Succeed<T, I, E> {
    Succeed::new(init)
}

pub use combinators::map_err::MapErr;
pub fn map_err<T: ?Sized + MetaSized, F, I>(func: F, init: I) -> MapErr<T, F, I> {
    MapErr::new(func, init)
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

unsafe impl<T, I: ExactSizeIterator<Item = T>, Extra> PinInit<[T], Extra> for FromIter<I> {
    type Error = InitFromIterError;

    fn metadata(&self) -> usize {
        self.iter.len()
    }

    unsafe fn init(mut self, dst: *mut [T], _: Extra) -> Result<(), Self::Error> {
        let mut buf = unsafe { noop_allocator::owning_slice::empty_from_raw(dst) };
        let count = self.iter.len();
        while buf.len() < count {
            let Some(item) = self.iter.next() else {
                return Err(InitFromIterError::TooShort);
            };
            buf.push(item);
        }
        core::mem::forget(buf);
        Ok(())
    }
}
unsafe impl<T, I: ExactSizeIterator<Item = T>, Extra> Init<[T], Extra> for FromIter<I> {}

pub use combinators::array_for_each::ArrayForEach;
pub fn array_for_each<F, const N: usize>(func: F) -> ArrayForEach<F, N> {
    ArrayForEach::new(func)
}

pub use combinators::slice_for_each::SliceForEach;
pub fn slice_for_each<F>(count: usize, func: F) -> SliceForEach<F> {
    SliceForEach::new(count, func)
}

pub use combinators::array_for_each_with::ArrayForEachWith;
pub fn array_for_each_with<F, const N: usize>(func: F) -> ArrayForEachWith<F, N> {
    ArrayForEachWith::new(func)
}

pub use combinators::slice_for_each_with::SliceForEachWith;
pub fn slice_for_each_with<F>(count: usize, func: F) -> SliceForEachWith<F> {
    SliceForEachWith::new(count, func)
}

pub use combinators::chain::Chain;
pub fn chain<I1, I2>(init1: I1, init2: I2) -> Chain<I1, I2> {
    Chain::new(init1, init2)
}

pub use combinators::ignore_extra::IgnoreExtra;
pub fn ignore_extra<T: ?Sized + MetaSized, I>(init: I) -> IgnoreExtra<T, I> {
    IgnoreExtra::new(init)
}

pub use combinators::map_extra::MapExtra;
pub fn map_extra<T: ?Sized + MetaSized, F, I>(func: F, init: I) -> MapExtra<T, F, I> {
    MapExtra::new(func, init)
}

pub use combinators::with_extra::WithExtra;
pub fn with_extra<T: ?Sized + MetaSized, I, Extra>(
    extra: Extra,
    init: I,
) -> WithExtra<T, I, Extra> {
    WithExtra::new(extra, init)
}

// Allocation and initialization

pub use allocation::Builder;

pub use allocation::boxed::{new_boxed, new_pinned, try_new_boxed, try_new_pinned};
pub use allocation::boxed::{new_boxed_in, new_pinned_in, try_new_boxed_in, try_new_pinned_in};

pub use allocation::vec::{VecExt, new_vec, try_new_vec};

pub use allocation::string::{StringExt, new_string, try_new_string};

pub use allocation::rc::{rc_new, rc_new_pinned, try_rc_new, try_rc_new_pinned};
pub use allocation::rc::{
    rc_new_cyclic, rc_new_cyclic_pinned, try_rc_new_cyclic, try_rc_new_cyclic_pinned,
};

fn try_initialize_with<T, Extra, E>(
    slot: &mut MaybeUninit<T>,
    init: impl Init<T, Extra, Error = E>,
    extra: Extra,
) -> Result<&mut T, E> {
    unsafe {
        init.init(slot.as_mut_ptr(), extra)?;
        Ok(slot.assume_init_mut())
    }
}

fn initialize_with<T, Extra>(
    slot: &mut MaybeUninit<T>,
    init: impl Init<T, Extra, Error = !>,
    extra: Extra,
) -> &mut T {
    try_initialize_with(slot, init, extra).unwrap_or_else(|e| match e {})
}

fn try_initialize<T, E>(
    slot: &mut MaybeUninit<T>,
    init: impl Init<T, Error = E>,
) -> Result<&mut T, E> {
    try_initialize_with(slot, init, ())
}

fn initialize<T>(slot: &mut MaybeUninit<T>, init: impl Init<T, Error = !>) -> &mut T {
    initialize_with(slot, init, ())
}
