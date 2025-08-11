use core::{marker::MetaSized, pin::Pin};

use alloc::{
    alloc::{Allocator, Global},
    boxed::Box,
    rc::Rc,
    vec::Vec,
};

use crate::{Init, PinInit};

pub(crate) mod boxed;
pub(crate) mod rc;
pub(crate) mod string;
pub(crate) mod vec;

pub struct Builder<I, A: Allocator, Extra> {
    init: I,
    alloc: A,
    extra: Extra,
}

impl<I> Builder<I, Global, ()> {
    pub fn new(init: I) -> Self {
        Self {
            init,
            alloc: Global,
            extra: (),
        }
    }
}

impl<I, A: Allocator> Builder<I, A, ()> {
    pub fn new_in(init: I, alloc: A) -> Self {
        Self {
            init,
            alloc,
            extra: (),
        }
    }
}

impl<I, A: Allocator, Extra> Builder<I, A, Extra> {
    pub fn with_alloc<A2: Allocator>(self, alloc: A2) -> Builder<I, A2, Extra> {
        Builder {
            init: self.init,
            alloc,
            extra: self.extra,
        }
    }

    pub fn with_extra<Extra2>(self, extra: Extra2) -> Builder<I, A, Extra2> {
        Builder {
            init: self.init,
            alloc: self.alloc,
            extra,
        }
    }

    pub fn try_build_box<T: MetaSized>(self) -> Result<Box<T, A>, I::Error>
    where
        I: Init<T, Extra>,
    {
        // SAFETY: `I` implements `Init<T, Extra>`
        unsafe { boxed::new_impl(self.init, self.alloc, self.extra) }
    }

    pub fn try_build_pinned_box<T: MetaSized>(self) -> Result<Pin<Box<T, A>>, I::Error>
    where
        I: PinInit<T, Extra>,
        A: 'static,
    {
        // Safety: the box is immediately pinned
        unsafe { boxed::new_impl(self.init, self.alloc, self.extra) }.map(Box::into_pin)
    }

    pub fn build_box<T: MetaSized>(self) -> Box<T, A>
    where
        I: Init<T, Extra, Error = !>,
    {
        self.try_build_box().unwrap_or_else(|e| match e {})
    }

    pub fn build_pinned_box<T: MetaSized>(self) -> Pin<Box<T, A>>
    where
        I: PinInit<T, Extra, Error = !>,
        A: 'static,
    {
        self.try_build_pinned_box().unwrap_or_else(|e| match e {})
    }

    pub fn try_build_vec<T>(self) -> Result<Vec<T, A>, I::Error>
    where
        I: Init<[T], Extra>,
    {
        self.try_build_box().map(Vec::from)
    }

    pub fn build_vec<T>(self) -> Vec<T, A>
    where
        I: Init<[T], Extra, Error = !>,
    {
        Vec::from(self.try_build_box().unwrap_or_else(|e| match e {}))
    }

    pub fn try_build_rc<T: MetaSized>(self) -> Result<Rc<T, A>, I::Error>
    where
        I: Init<T, Extra>,
    {
        // SAFETY: `I` implements `Init<T, Extra>`
        unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, Extra, rc::NonWeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }
    }

    pub fn try_build_pinned_rc<T: MetaSized>(self) -> Result<Pin<Rc<T, A>>, I::Error>
    where
        I: PinInit<T, Extra>,
        A: 'static,
    {
        // Safety: the rc is immediately pinned
        let rc = unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, Extra, rc::NonWeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }?;
        Ok(unsafe { Pin::new_unchecked(rc) })
    }

    pub fn build_rc<T: MetaSized>(self) -> Rc<T, A>
    where
        I: Init<T, Extra, Error = !>,
    {
        self.try_build_rc().unwrap_or_else(|e| match e {})
    }

    pub fn build_pinned_rc<T: MetaSized>(self) -> Pin<Rc<T, A>>
    where
        I: PinInit<T, Extra, Error = !>,
        A: 'static,
    {
        self.try_build_pinned_rc().unwrap_or_else(|e| match e {})
    }

    pub fn try_build_cyclic_rc_with<T: MetaSized>(self) -> Result<Rc<T, A>, I::Error>
    where
        I: Init<T, (rc::Weak<T, A>, Extra)>,
        A: Clone,
    {
        // SAFETY: `I` implements `Init<T, _>`
        unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, Extra, rc::WithWeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }
    }

    /// # Safety
    ///
    /// The `rc::Weak<T, A>`s passed to `init` must be treated as pinned.
    pub unsafe fn try_build_pinned_cyclic_rc_with<T: MetaSized>(
        self,
    ) -> Result<Pin<Rc<T, A>>, I::Error>
    where
        I: PinInit<T, (rc::Weak<T, A>, Extra)>,
        A: Clone + 'static,
    {
        // Safety: the rc is immediately pinned, the `Weak` requirement is discharged to the caller
        let rc = unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, Extra, rc::WithWeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }?;
        Ok(unsafe { Pin::new_unchecked(rc) })
    }

    pub fn build_cyclic_rc_with<T: MetaSized>(self) -> Rc<T, A>
    where
        I: Init<T, (rc::Weak<T, A>, Extra), Error = !>,
        A: Clone,
    {
        self.try_build_cyclic_rc_with()
            .unwrap_or_else(|e| match e {})
    }

    /// # Safety
    ///
    /// The `rc::Weak<T, A>`s passed to `init` must be treated as pinned.
    pub unsafe fn build_pinned_cyclic_rc_with<T: MetaSized>(self) -> Pin<Rc<T, A>>
    where
        I: PinInit<T, (rc::Weak<T, A>, Extra), Error = !>,
        A: Clone + 'static,
    {
        // SAFETY: discharged to caller
        unsafe {
            self.try_build_pinned_cyclic_rc_with()
                .unwrap_or_else(|e| match e {})
        }
    }
}

#[allow(clippy::unit_arg, reason = "symmetry")]
impl<I, A: Allocator> Builder<I, A, ()> {
    pub fn try_build_cyclic_rc<T: MetaSized>(self) -> Result<Rc<T, A>, I::Error>
    where
        I: Init<T, rc::Weak<T, A>>,
        A: Clone,
    {
        // SAFETY: `I` implements `Init<T, ()>`
        unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, (), rc::WeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }
    }

    /// # Safety
    ///
    /// The `rc::Weak<T, A>`s passed to `init` must be treated as pinned.
    pub unsafe fn try_build_pinned_cyclic_rc<T: MetaSized>(self) -> Result<Pin<Rc<T, A>>, I::Error>
    where
        I: PinInit<T, rc::Weak<T, A>>,
        A: Clone + 'static,
    {
        // Safety: the rc is immediately pinned, the `Weak` requirement is discharged to the caller
        let rc = unsafe {
            rc::rc_new_base_impl::<T, I::Error, A, (), rc::WeakExtra>(
                self.init, self.alloc, self.extra,
            )
        }?;
        Ok(unsafe { Pin::new_unchecked(rc) })
    }

    pub fn build_cyclic_rc<T: MetaSized>(self) -> Rc<T, A>
    where
        I: Init<T, rc::Weak<T, A>, Error = !>,
        A: Clone,
    {
        self.try_build_cyclic_rc().unwrap_or_else(|e| match e {})
    }

    /// # Safety
    ///
    /// The `rc::Weak<T, A>`s passed to `init` must be treated as pinned.
    pub unsafe fn build_pinned_cyclic_rc<T: MetaSized>(self) -> Pin<Rc<T, A>>
    where
        I: PinInit<T, rc::Weak<T, A>, Error = !>,
        A: Clone + 'static,
    {
        // SAFETY: discharged to caller
        unsafe {
            self.try_build_pinned_cyclic_rc()
                .unwrap_or_else(|e| match e {})
        }
    }
}
