use core::mem::MaybeUninit;

use alloc::{alloc::Allocator, vec::Vec};

use crate::Init;

pub fn try_new_vec<T, E>(init: impl Init<[T], Error = E>) -> Result<Vec<T>, E> {
    crate::try_new_boxed(init).map(Vec::from)
}
pub fn new_vec<T>(init: impl Init<[T], Error = !>) -> Vec<T> {
    Vec::from(crate::new_boxed(init))
}

/// # Safety
///
/// See method documentation.
pub unsafe trait VecExt {
    type Item;

    /// # Safety
    ///
    /// `self` must have excess capacity.
    unsafe fn try_push_emplace_within_capacity_unchecked<E, I: Init<Self::Item, Error = E>>(
        &mut self,
        init: I,
    ) -> Result<(), E>;
    /// # Safety
    ///
    /// `self` must have excess capacity.
    unsafe fn push_emplace_within_capacity_unchecked<I: Init<Self::Item, Error = !>>(
        &mut self,
        init: I,
    );

    fn try_push_emplace_within_capacity<E, I: Init<Self::Item, Error = E>>(
        &mut self,
        init: I,
    ) -> Result<Result<(), E>, I>;
    fn push_emplace_within_capacity<I: Init<Self::Item, Error = !>>(
        &mut self,
        init: I,
    ) -> Result<(), I>;

    fn try_push_emplace<E>(&mut self, init: impl Init<Self::Item, Error = E>) -> Result<(), E>;
    fn push_emplace(&mut self, init: impl Init<Self::Item, Error = !>);

    fn try_append_emplace<E>(&mut self, init: impl Init<[Self::Item], Error = E>) -> Result<(), E>;
    fn append_emplace(&mut self, init: impl Init<[Self::Item], Error = !>);

    fn try_extend_emplace<E>(
        &mut self,
        iter: impl IntoIterator<Item: Init<Self::Item, Error = E>>,
    ) -> Result<(), E>;

    fn extend_emplace(&mut self, iter: impl IntoIterator<Item: Init<Self::Item, Error = !>>);
}

unsafe impl<T, A: Allocator> VecExt for Vec<T, A> {
    type Item = T;

    unsafe fn try_push_emplace_within_capacity_unchecked<E, I: Init<Self::Item, Error = E>>(
        &mut self,
        init: I,
    ) -> Result<(), E> {
        let len = self.len();
        // SAFETY: caller ensures there is excess capacity
        let slot = unsafe { self.spare_capacity_mut().get_unchecked_mut(0) };
        crate::try_initialize(slot, init)?;
        unsafe {
            self.set_len(len + 1);
        };
        Ok(())
    }

    unsafe fn push_emplace_within_capacity_unchecked<I: Init<Self::Item, Error = !>>(
        &mut self,
        init: I,
    ) {
        // SAFETY: caller ensures there is excess capacity
        unsafe {
            self.try_push_emplace_within_capacity_unchecked(init)
                .unwrap_or_else(|e| match e {})
        }
    }

    fn try_push_emplace_within_capacity<E, I: Init<Self::Item, Error = E>>(
        &mut self,
        init: I,
    ) -> Result<Result<(), E>, I> {
        if self.len() < self.capacity() {
            // SAFETY: there is excess capacity
            Ok(unsafe { self.try_push_emplace_within_capacity_unchecked(init) })
        } else {
            Err(init)
        }
    }

    fn push_emplace_within_capacity<I: Init<Self::Item, Error = !>>(
        &mut self,
        init: I,
    ) -> Result<(), I> {
        self.try_push_emplace_within_capacity(init)
            .map(|r| r.unwrap_or_else(|e| match e {}))
    }

    fn try_push_emplace<E>(&mut self, init: impl Init<T, Error = E>) -> Result<(), E> {
        self.reserve(1);
        let len = self.len();
        crate::try_initialize(&mut self.spare_capacity_mut()[0], init)?;
        unsafe {
            self.set_len(len + 1);
        };
        Ok(())
    }

    fn push_emplace(&mut self, init: impl Init<T, Error = !>) {
        self.try_push_emplace(init).unwrap_or_else(|e| match e {});
    }

    fn try_append_emplace<E>(&mut self, init: impl Init<[T], Error = E>) -> Result<(), E> {
        let additional = init.metadata();
        self.reserve(additional);
        let len = self.len();
        unsafe {
            init.init(
                &mut self.spare_capacity_mut()[..additional] as *mut [MaybeUninit<T>] as *mut [T],
                (),
            )?;
            self.set_len(len + additional);
        };
        Ok(())
    }

    fn append_emplace(&mut self, init: impl Init<[T], Error = !>) {
        self.try_append_emplace(init).unwrap_or_else(|e| match e {});
    }

    fn try_extend_emplace<E>(
        &mut self,
        iter: impl IntoIterator<Item: Init<T, Error = E>>,
    ) -> Result<(), E> {
        let mut iter = iter.into_iter();
        let min = iter.size_hint().0;
        self.reserve(min);
        for init in iter.by_ref().take(min) {
            // SAFETY: we reserved `min` slots, and are emplacing at most `min` elements in this loop.
            unsafe { self.try_push_emplace_within_capacity_unchecked(init) }?;
        }
        for init in iter {
            self.try_push_emplace(init)?;
        }
        Ok(())
    }

    fn extend_emplace(&mut self, iter: impl IntoIterator<Item: Init<T, Error = !>>) {
        self.try_extend_emplace(iter).unwrap_or_else(|e| match e {});
    }
}
