use core::marker::{MetaSized, PhantomData};

use crate::{Init, PinInit};

/// Initialize a slice by initializing a slice of arrays.
pub struct Flatten<Src: MetaSized, Dst: MetaSized, I> {
    result: PhantomData<fn(Src) -> Dst>,
    init: I,
}

impl<Src: MetaSized, Dst: MetaSized, I: Clone> Clone for Flatten<Src, Dst, I> {
    fn clone(&self) -> Self {
        Self {
            result: PhantomData,
            init: self.init.clone(),
        }
    }
}
impl<Src: MetaSized, Dst: MetaSized, I: Copy> Copy for Flatten<Src, Dst, I> {}

impl<T, const N: usize, const M: usize, const P: usize, I> Flatten<[[T; N]; M], [T; P], I> {
    pub const fn try_new(init: I) -> Result<Self, I> {
        if let Some(size) = N.checked_mul(M)
            && size == P
        {
            Ok(Self {
                result: PhantomData,
                init,
            })
        } else {
            Err(init)
        }
    }

    pub const fn new(init: I) -> Self {
        if usize::strict_mul(N, M) != P {
            panic!("array length mismatch")
        };
        Self {
            result: PhantomData,
            init,
        }
    }

    pub fn for_slice(self) -> Flatten<[[T; N]], [T], crate::ForSlice<I, M>> {
        Flatten {
            result: PhantomData,
            init: crate::for_slice(self.init),
        }
    }
}

impl<T, const N: usize, I> Flatten<[[T; N]], [T], I> {
    pub const fn new_slice(init: I) -> Self {
        Self {
            result: PhantomData,
            init,
        }
    }
}

/// Initialize an array with an array of arrays with the same total element length.
unsafe impl<
    T,
    const N: usize,
    const M: usize,
    const P: usize,
    Error,
    Extra,
    I: PinInit<[[T; N]; M], Error, Extra>,
> PinInit<[T], Error, Extra> for Flatten<[[T; N]; M], [T; P], I>
{
    fn metadata(&self) -> usize {
        P
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Error> {
        if cfg!(debug_assertions) {
            assert_eq!(usize::strict_mul(N, M), P);
            assert_eq!(dst.len(), P);
        };
        let dst = dst.cast::<[[T; N]; M]>();
        // SAFETY: discharged to caller, and
        // an array of `T` has the same layout as an array of arrays of `T` with the same total element length.
        unsafe { self.init.init(dst, extra) }
    }
}
unsafe impl<
    T,
    const N: usize,
    const M: usize,
    const P: usize,
    Error,
    Extra,
    I: Init<[[T; N]; M], Error, Extra>,
> Init<[T], Error, Extra> for Flatten<[[T; N]; M], [T; P], I>
{
}

/// Initialize a slice with an array of arrays with the same total element length.
unsafe impl<
    T,
    const N: usize,
    const M: usize,
    const P: usize,
    Error,
    Extra,
    I: PinInit<[[T; N]; M], Error, Extra>,
> PinInit<[T; P], Error, Extra> for Flatten<[[T; N]; M], [T; P], I>
{
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut [T; P], extra: Extra) -> Result<(), Error> {
        if cfg!(debug_assertions) {
            assert_eq!(usize::strict_mul(N, M), P);
        };
        let dst = dst.cast::<[[T; N]; M]>();
        // SAFETY: discharged to caller, and
        // an array of `T` has the same layout as an array of arrays of `T` with the same total element length.
        unsafe { self.init.init(dst, extra) }
    }
}
unsafe impl<
    T,
    const N: usize,
    const M: usize,
    const P: usize,
    Error,
    Extra,
    I: Init<[[T; N]; M], Error, Extra>,
> Init<[T; P], Error, Extra> for Flatten<[[T; N]; M], [T; P], I>
{
}

/// Initialize a slice with a slice of arrays with the same total element length.
unsafe impl<T, const N: usize, Error, Extra, I: PinInit<[[T; N]], Error, Extra>>
    PinInit<[T], Error, Extra> for Flatten<[[T; N]], [T], I>
{
    fn metadata(&self) -> usize {
        self.init
            .metadata()
            .checked_mul(N)
            .expect("slice length overflow")
    }

    unsafe fn init(self, dst: *mut [T], extra: Extra) -> Result<(), Error> {
        if cfg!(debug_assertions) {
            let total_len = <Self as PinInit<[T], Error, _>>::metadata(&self);
            assert_eq!(dst.len(), total_len);
        };
        let chunk_count: usize = self.init.metadata();
        let dst = core::ptr::slice_from_raw_parts_mut(dst.cast::<[T; N]>(), chunk_count);
        // SAFETY: discharged to caller, and
        // a slice of `T` has the same layout of a slice of arrays of `T` with the same total element length.
        unsafe { self.init.init(dst, extra) }
    }
}
unsafe impl<T, const N: usize, Extra, I: Init<[[T; N]], Extra>> Init<[T], Extra>
    for Flatten<[[T; N]], [T], I>
{
}
