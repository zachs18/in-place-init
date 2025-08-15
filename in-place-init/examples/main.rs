#![feature(ptr_metadata)]
#![allow(unused)]

use std::{fmt, marker::PhantomPinned, mem::MaybeUninit, pin::Pin, ptr::NonNull, rc::Weak};

use in_place_init::{Init, PinInit, VecExt};

struct SelfReferential {
    value: u32,
    ptr: *mut u32,
    _pinned: PhantomPinned,
}

impl SelfReferential {
    fn foo(self: Pin<&mut Self>) {
        println!("{}", self.value);
        unsafe {
            *self.ptr += 1;
        }
        println!("{}", self.value);
    }
}

struct MakeSelfReferential(u32);

unsafe impl<Error> PinInit<SelfReferential, Error> for MakeSelfReferential {
    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut SelfReferential, _: ()) -> Result<(), Error> {
        unsafe {
            (*dst).value = self.0;
            (*dst).ptr = &raw mut (*dst).value;
        }
        Ok(())
    }
}

mod sbo {
    use std::{
        fmt,
        marker::PhantomPinned,
        mem::{ManuallyDrop, MaybeUninit},
        pin::Pin,
        ptr::NonNull,
    };

    use in_place_init::{Init, PinInit};

    pub struct ShortBuffer<const SHORT_CAP: usize, T> {
        /// This ptr either points to a heap allocation, or to `buffer`.
        /// This is fine because we only expose pinned in-place initailizers for `ShortBuffer`.
        ptr: NonNull<T>,
        len: usize,
        cap: usize,
        buffer: [MaybeUninit<T>; SHORT_CAP],
        pinned: PhantomPinned,
    }

    impl<const N: usize, T: fmt::Debug> fmt::Debug for ShortBuffer<N, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.as_slice().fmt(f)
        }
    }

    impl<const N: usize, T> Drop for ShortBuffer<N, T> {
        fn drop(&mut self) {
            // SAFETY: all constructors of ShortBuffer initialize it into a pinned place.
            unsafe { std::ptr::drop_in_place(Pin::new_unchecked(&mut *self).as_mut_slice()) };
            self.len = 0;
            if !self.is_inline() {
                drop(unsafe { Vec::from_raw_parts(self.ptr.as_ptr(), 0, self.cap) });
            }
        }
    }

    pub struct MakeShortBuffer<I> {
        pub slice_initializer: I,
    }

    unsafe impl<T, const N: usize, Error, Extra, I: Init<[T], Error, Extra>>
        PinInit<ShortBuffer<N, T>, Error, Extra> for MakeShortBuffer<I>
    {
        fn metadata(&self) {}

        unsafe fn init(self, dst: *mut ShortBuffer<N, T>, extra: Extra) -> Result<(), Error> {
            let len = PinInit::<[T], Error, Extra>::metadata(&self.slice_initializer);
            if size_of::<T>() == 0 || len <= N {
                // in-place
                let buf = unsafe { &raw mut (*dst).buffer };
                let ptr = NonNull::new(buf).unwrap().cast();
                let buf: *mut [T] = core::ptr::slice_from_raw_parts_mut(buf.cast(), len);
                let cap = if size_of::<T>() == 0 { usize::MAX } else { N };
                unsafe {
                    (*dst).ptr = ptr;
                    (*dst).len = len;
                    (*dst).cap = N;
                    self.slice_initializer.init(buf, extra)?;
                }
                Ok(())
            } else {
                // on the heap
                let buf = in_place_init::try_new_boxed::<[T], _>(in_place_init::with_extra(
                    self.slice_initializer,
                    extra,
                ))?;
                debug_assert_eq!(buf.len(), len);
                let ptr = NonNull::new(Box::into_raw(buf).cast::<T>()).unwrap();
                unsafe {
                    (*dst).ptr = ptr;
                    (*dst).len = len;
                    (*dst).cap = len;
                }
                Ok(())
            }
        }
    }

    impl<const N: usize, T> ShortBuffer<N, T> {
        pub fn as_slice(&self) -> &[T] {
            unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
        pub fn as_mut_slice(self: Pin<&mut Self>) -> &mut [T] {
            unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
        }
        pub unsafe fn set_len(self: Pin<&mut Self>, new_len: usize) {
            let this = unsafe { Pin::get_unchecked_mut(self) };
            debug_assert!(new_len <= this.cap);
            this.len = new_len;
        }
        pub fn as_mut_ptr(self: Pin<&mut Self>) -> *mut T {
            self.ptr.as_ptr()
        }
        pub fn pop(mut self: Pin<&mut Self>) -> Option<T> {
            if self.len > 0 {
                let new_len = self.len - 1;
                unsafe {
                    self.as_mut().set_len(new_len);
                    Some(self.as_mut_ptr().add(new_len).read())
                }
            } else {
                None
            }
        }
        pub fn len(&self) -> usize {
            self.len
        }
        pub fn capacity(&self) -> usize {
            self.cap
        }
        pub fn reserve(mut self: Pin<&mut Self>, additional: usize) {
            let remaining = self.cap - self.len;
            if remaining >= additional {
                return;
            }
            if let Some(mut buf) = self.as_mut().take_heap() {
                // reallocate on the heap
                buf.reserve(additional);
                self.as_mut().overwrite_heap(buf);
                debug_assert!(self.cap - self.len >= additional);
            } else {
                // move to heap
                let cap = usize::max(N.saturating_mul(2), self.len.saturating_add(additional));
                if cap - self.len < additional {
                    panic!("capacity overflow")
                }
                let mut buf = Vec::with_capacity(cap);
                buf.extend(self.as_mut().take_in_place());
                self.as_mut().overwrite_heap(buf);
            }
        }
        pub fn is_inline(&self) -> bool {
            size_of::<T>() == 0 || std::ptr::addr_eq(self.ptr.as_ptr(), self.buffer.as_ptr())
        }
        pub fn take_in_place(
            self: Pin<&mut Self>,
        ) -> noop_allocator::owning_ref::OwningRef<'_, [T]> {
            let this = unsafe { Pin::get_unchecked_mut(self) };
            let len = this.len;
            this.len = 0;
            unsafe {
                noop_allocator::owning_ref::from_raw(core::ptr::slice_from_raw_parts_mut(
                    this.ptr.as_ptr(),
                    len,
                ))
            }
        }
        /// If `self` is not inline, takes ownership of the elements and heap allocation
        /// and makes `self` inline and empty.
        pub fn take_heap(self: Pin<&mut Self>) -> Option<Vec<T>> {
            if self.is_inline() {
                return None;
            }
            let this = unsafe { Pin::get_unchecked_mut(self) };
            let ptr = this.ptr.as_ptr();
            let len = this.len;
            let cap = this.cap;
            this.ptr = NonNull::new(&raw mut (*this).buffer).unwrap().cast();
            this.len = 0;
            this.cap = N; // T is not a ZST, or we'd always be inline and never reach here
            Some(unsafe { Vec::from_raw_parts(ptr, len, cap) })
        }
        /// Overwrite this empty inline buffer with a heap buffer.
        ///
        /// Leaks all current elements, and the current heap allocation, if any.
        ///
        /// If `size_of::<T>() == 0 || heap.capacity() == 0` , this should not be called
        fn overwrite_heap(self: Pin<&mut Self>, heap: Vec<T>) {
            debug_assert!(
                heap.capacity() > 0,
                "overwrite_heap should not be called with no-alloc heap buffers"
            );
            debug_assert!(
                size_of::<T>() > 0,
                "overwrite_heap should not be called for ZSTs (because they are always inline)"
            );
            debug_assert!(
                self.len == 0,
                "overwrite_heap should only be called on empty buffers"
            );
            debug_assert!(
                self.is_inline(),
                "overwrite_heap should only be called on inline buffers"
            );
            let mut heap = ManuallyDrop::new(heap);
            let this = unsafe { Pin::get_unchecked_mut(self) };
            this.len = heap.len();
            this.cap = heap.capacity();
            this.ptr = NonNull::new(heap.as_mut_ptr()).unwrap();
        }

        pub fn push(mut self: Pin<&mut Self>, value: T) {
            self.as_mut().reserve(1);
            let idx = self.len();
            unsafe {
                self.as_mut().as_mut_ptr().add(idx).write(value);
                self.set_len(idx + 1);
            }
        }
    }
}

fn main() {
    let mut vec: Vec<[usize; 100]> = vec![];
    vec.push_emplace(in_place_init::array_repeat(vec.len()));
    vec.push_emplace(in_place_init::array_repeat(vec.len()));
    assert_eq!(vec, [[0; 100], [1; 100]]);

    let bx = in_place_init::new_boxed::<str>("hello, world");
    dbg!(bx);

    let mut buffer = {
        static mut BUFFER: MaybeUninit<sbo::ShortBuffer<4, String>> = MaybeUninit::uninit();
        in_place_init::initialize_pinned_owned(
            unsafe { &mut *&raw mut BUFFER },
            sbo::MakeShortBuffer {
                slice_initializer: [String::from("hello"), String::from("world")],
            },
        )
    };

    println!("{buffer:?}");
    assert_eq!(buffer.as_mut().pop(), Some(String::from("world")));
    println!("{buffer:?}");
    buffer.as_mut().push("my".to_owned());
    buffer.as_mut().push("good".to_owned());
    buffer.as_mut().push("friend".to_owned());
    assert!(buffer.is_inline());
    println!("{buffer:?}");
    buffer.as_mut().push("Steve".to_owned());
    assert!(!buffer.is_inline());
    println!("{buffer:?}");
    drop(buffer.as_mut().take_heap().unwrap());
    buffer.as_mut().push("my".to_owned());
    buffer.as_mut().push("good".to_owned());
    buffer.as_mut().push("friend".to_owned());
    assert!(buffer.is_inline());
    println!("{buffer:?}");
    drop(buffer);

    #[derive(Debug)]
    #[allow(unused)]
    struct Foo {
        weak: Weak<Foo>,
    }

    let rc = in_place_init::rc_new_cyclic(in_place_init::with(|weak| Foo { weak }));
    dbg!(rc);

    #[derive(Debug)]
    #[allow(unused)]
    struct Bar {
        idx: usize,
        weak: Weak<[Bar]>,
    }

    let rc = in_place_init::rc_new_cyclic(in_place_init::slice_for_each(2, |idx| {
        in_place_init::with(move |weak| Bar { idx, weak })
    }));
    dbg!(rc);

    let rc = in_place_init::rc_new_cyclic(in_place_init::slice_for_each_with(2, |idx, weak| Bar {
        idx,
        weak,
    }));
    dbg!(rc);

    let rc =
        in_place_init::rc_new::<[String]>(in_place_init::slice_for_each(42, |x| format!("{x}")));
    println!("{rc:?}");

    _ = std::panic::catch_unwind(|| {
        let rc = in_place_init::try_rc_new::<[String], _>(in_place_init::slice_for_each(42, |x| {
            if x == 30 {
                panic!()
            }
            format!("{x}")
        }))
        .unwrap();
        println!("{rc:?}");
    });

    let mut sr = in_place_init::new_pinned::<SelfReferential>(MakeSelfReferential(42));
    sr.as_mut().foo();

    const N: usize = if cfg!(miri) { 4 } else { 64 };

    #[inline(never)]
    pub fn fooo() -> Box<[[[usize; N]; N]]> {
        in_place_init::new_boxed(in_place_init::array_for_each::<6, _>(|x| {
            in_place_init::array_for_each(move |y| {
                in_place_init::array_for_each(move |z| x * 100000000 + y * 10000 + z)
            })
        }))
    }
    let bx = fooo();

    println!("{bx:p}");
    drop(bx);

    #[cfg(feature = "bytemuck")]
    {
        #[inline(never)]
        pub fn baaar() -> Box<[[[usize; N]; N]]> {
            in_place_init::new_boxed(in_place_init::Zeroed::new_zeroable_slice(6))
        }
        let bx = baaar();

        println!("{bx:p}");
        drop(bx);
    }

    let bx: Box<str> = in_place_init::new_boxed(in_place_init::chain("hello", "world"));
    assert_eq!(&*bx, "helloworld");

    #[cfg(feature = "macros")]
    {
        use std::rc::Rc;

        let bx: Box<foo::Bar<42, str>> = in_place_init::new_boxed(foo::BarInit(
            in_place_init::array_for_each(|idx| idx as u8),
            "hello, world!",
        ));
        println!("{bx:?}");

        let rc1: Rc<foo::Baz<str>> = in_place_init::rc_new_cyclic(foo::BazInit(
            in_place_init::with(|weak| weak),
            in_place_init::ignore_extra("hello, world!"),
        ));
        let rc2 = rc1.this.upgrade().unwrap();
        println!("{rc1:?}");
        assert!(Rc::ptr_eq(&rc1, &rc2));

        let bx: Box<foo::Bar<20, dyn std::any::Any>> = in_place_init::new_boxed(foo::BarInit(
            in_place_init::array_for_each({
                let mut acc = 0;
                move |idx| {
                    acc += idx;
                    acc as u8
                }
            }),
            in_place_init::unsize(String::from("hello, world")),
        ));
        println!("{bx:?}");
        assert_eq!(
            bx.y.downcast_ref::<String>().map(String::as_str),
            Some("hello, world")
        );
    }
}

#[cfg(feature = "macros")]
mod foo {
    use in_place_init::Init;

    #[derive(Init)]
    struct Foo;

    #[derive(Debug, Init)]
    pub(crate) struct Bar<const N: usize, T: ?Sized> {
        pub x: [u8; N],
        pub y: T,
    }

    #[derive(Debug, Init)]
    pub(crate) struct Baz<T: ?Sized> {
        pub this: std::rc::Weak<Self>,
        pub tail: T,
    }

    pub trait Trait {
        type Assoc<U: ?Sized>;
    }
    impl<T: ?Sized> Trait for T {
        type Assoc<U: ?Sized> = u32;
    }

    #[derive(Debug, Init)]
    pub(crate) struct Quux<T: ?Sized> {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }

    #[derive(Debug, Init)]
    pub(crate) struct Phooey<T>
    where
        T: ?Sized,
    {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }

    #[derive(Debug, Init)]
    pub(crate) struct Phenomenal<T>
    where
        T: ?Sized,
    {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }
}
