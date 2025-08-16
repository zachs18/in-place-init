#![allow(unused)]
use std::mem::MaybeUninit;

mod sbo {
    use std::{
        fmt,
        marker::PhantomPinned,
        mem::{ManuallyDrop, MaybeUninit},
        pin::Pin,
        ptr::NonNull,
    };

    use in_place_init::{Init, PinInit};

    pub struct ShortBuffer<const N: usize, T> {
        /// This buffer can be dangling, inline, or heap.
        ///
        /// * Dangling: `self.ptr` is an arbitrary non-null aligned pointer.
        /// * Inline: `self.ptr` is a pointer to `self.buffer` (requires `self` is pinned).
        /// * Heap: `self.ptr` is a pointer to a heap allocation, following the requirements of `Vec<T>`.
        ///
        /// Invariants:
        ///
        /// * If `self.cap == 0 || size_of::<T>() == 0`, this is a dangling buffer.
        /// * If `self.cap > 0 && size_of::<T>() == 0`, this is an inline buffer if and only if `self.ptr == &raw self.buffer`, and a heap buffer otherwise.
        /// * If this is an inline buffer, then `self.cap == N`.
        /// * If this is an inline buffer, then `self` is pinned.
        ptr: NonNull<T>,
        len: usize,
        cap: usize,
        buffer: [MaybeUninit<T>; N],
        _pinned: PhantomPinned,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum BufferKind {
        Inline,
        Heap,
        Dangling,
    }

    impl<const N: usize, T: fmt::Debug> fmt::Debug for ShortBuffer<N, T> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            self.as_slice().fmt(f)
        }
    }

    impl<const N: usize, T> Drop for ShortBuffer<N, T> {
        fn drop(&mut self) {
            // drop elements
            if core::mem::needs_drop::<T>() {
                let to_drop = core::ptr::slice_from_raw_parts_mut(self.ptr.as_ptr(), self.len);
                unsafe {
                    core::ptr::drop_in_place(to_drop);
                }
            }
            // deallocate if heap
            if let BufferKind::Heap = self.buffer_kind() {
                let buffer = core::ptr::slice_from_raw_parts_mut(
                    self.ptr.as_ptr().cast::<MaybeUninit<T>>(),
                    self.cap,
                );
                drop(unsafe { Box::from_raw(buffer) });
            }
        }
    }

    impl<const N: usize, T> ShortBuffer<N, T> {
        pub const fn new() -> Self {
            Self::from_heap(Vec::new())
        }

        pub const fn from_heap(vec: Vec<T>) -> Self {
            // Note: MaybeUnint instead of ManuallyDrop, since there's no safe way to get `&mut T` from `ManuallyDrop<T>` in `const`.
            let mut vec = MaybeUninit::new(vec);
            // SAFETY: `vec` is initialized
            let vec = unsafe { MaybeUninit::assume_init_mut(&mut vec) };
            let len = vec.len();
            let cap = vec.capacity();
            let ptr = NonNull::new(vec.as_mut_ptr()).unwrap();
            Self {
                ptr,
                len,
                cap,
                buffer: [const { MaybeUninit::uninit() }; N],
                _pinned: PhantomPinned,
            }
        }

        pub fn as_slice(&self) -> &[T] {
            unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
        }
        pub fn as_mut_slice(self: Pin<&mut Self>) -> &mut [T] {
            unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
        }
        pub fn as_mut_slice_unpinned(&mut self) -> &mut [T] {
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
            } else if size_of::<T>() == 0 {
                // We can only reach this point if `self.len + additional > usize::MAX`,
                // which is a request we can never satisfy
                panic!("capacity overflow");
            } else if self.cap == 0 {
                // We can only reach this point if the buffer is dangling
                // and the element is non-zero-sized, which implies the buffer is empty,
                // so we don't need to move any elements.
                if additional > N {
                    // allocate on heap
                    self.as_mut().overwrite_heap(Vec::with_capacity(additional));
                } else {
                    // make inline
                    let mut this = unsafe { Pin::get_unchecked_mut(self.as_mut()) };
                    this.ptr = NonNull::new(&raw mut this.buffer).unwrap().cast();
                    this.cap = N;
                }
            } else if let Some(mut heap) = self.as_mut().take_vec() {
                // reallocate on the heap
                heap.reserve(additional);
                self.as_mut().overwrite_heap(heap);
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
            debug_assert!(self.cap - self.len >= additional);
        }

        #[inline]
        fn buffer_kind(&self) -> BufferKind {
            if size_of::<T>() == 0 || self.cap == 0 {
                BufferKind::Dangling
            } else if std::ptr::addr_eq(self.ptr.as_ptr(), self.buffer.as_ptr()) {
                BufferKind::Inline
            } else {
                BufferKind::Heap
            }
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
        /// If `self` is a heap or dangling buffer, takes ownership of the elements and heap allocation
        /// and makes `self` dangling and empty. Does nothing and returns `None` if `self` is an inline buffer.
        pub fn take_vec(self: Pin<&mut Self>) -> Option<Vec<T>> {
            match self.buffer_kind() {
                BufferKind::Inline => None,
                BufferKind::Dangling => {
                    let mut vec = Vec::new();
                    let len = self.len;
                    if len > 0 {
                        debug_assert!(
                            vec.capacity() >= len,
                            "Vec of ZSTs should have usize::MAX capacity"
                        );
                        debug_assert!(
                            size_of::<T>() > 0,
                            "BufferKind::Dangling should not occur for non-empty buffers of non-zero-sized elements"
                        );
                        // SAFETY: T is a ZST,
                        // because `BufferKind::Dangling` only occurs when `self.cap == 0 || size_of::<T>() == 0`,
                        // and `0 < self.len <= self.cap`,
                        unsafe {
                            self.set_len(0);
                            vec.set_len(len);
                        }
                    }
                    Some(vec)
                }
                BufferKind::Heap => {
                    let this = unsafe { Pin::get_unchecked_mut(self) };
                    let ptr = this.ptr.as_ptr();
                    let len = this.len;
                    let cap = this.cap;
                    this.ptr = NonNull::dangling();
                    this.len = 0;
                    this.cap = 0;
                    Some(unsafe { Vec::from_raw_parts(ptr, len, cap) })
                }
            }
        }
        /// Overwrite this empty non-heap buffer with a heap or dangling buffer.
        ///
        /// If this buffer is non-empty or a heap buffer, this will either panic
        /// or leak all current elements, and the current heap allocation, if any.
        fn overwrite_heap(self: Pin<&mut Self>, heap: Vec<T>) {
            debug_assert!(
                self.len == 0,
                "overwrite_heap should only be called on empty buffers"
            );
            debug_assert!(
                !matches!(self.buffer_kind(), BufferKind::Heap),
                "overwrite_heap should only be called on non-heap buffers"
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

        pub fn extend_copy(mut self: Pin<&mut Self>, value: &[T])
        where
            T: Copy,
        {
            self.as_mut().reserve(value.len());
            let idx = self.len();
            unsafe {
                self.as_mut()
                    .as_mut_ptr()
                    .add(idx)
                    .copy_from_nonoverlapping(value.as_ptr(), value.len());
                self.set_len(idx + value.len());
            }
        }

        pub fn extend<I: IntoIterator<Item = T>>(mut self: Pin<&mut Self>, iter: I) {
            let mut iter = iter.into_iter();
            let (additional, _) = iter.size_hint();
            self.as_mut().reserve(additional);
            let idx = self.len();
            for _ in 0..additional {
                let Some(value) = iter.next() else { return };
                unsafe {
                    self.as_mut().as_mut_ptr().add(idx).write(value);
                    self.as_mut().set_len(idx + 1);
                }
            }
            for value in iter {
                self.as_mut().push(value);
            }
        }
    }

    pub struct MakeEmptyShortBuffer;

    unsafe impl<T, const N: usize, Error> PinInit<ShortBuffer<N, T>, Error> for MakeEmptyShortBuffer {
        fn metadata(&self) {}

        unsafe fn init(self, dst: *mut ShortBuffer<N, T>, _: ()) -> Result<(), Error> {
            unsafe {
                (*dst).ptr = NonNull::dangling();
                (*dst).len = 0;
                (*dst).cap = const { if size_of::<T>() == 0 { usize::MAX } else { 0 } };
            }
            Ok(())
        }
    }
    // SAFETY: `MakeEmptyShortBuffer` always makes a dangling buffer, which is safe to be not pinned.
    unsafe impl<T, const N: usize, Error> Init<ShortBuffer<N, T>, Error> for MakeEmptyShortBuffer {}

    pub struct MakeShortBuffer<I> {
        pub slice_initializer: I,
    }

    unsafe impl<T, const N: usize, Error, Extra, I: Init<[T], Error, Extra>>
        PinInit<ShortBuffer<N, T>, Error, Extra> for MakeShortBuffer<I>
    {
        fn metadata(&self) {}

        unsafe fn init(self, dst: *mut ShortBuffer<N, T>, extra: Extra) -> Result<(), Error> {
            let len = PinInit::<[T], Error, Extra>::metadata(&self.slice_initializer);
            if size_of::<T>() == 0 || len == 0 {
                // dangling
                let ptr = NonNull::dangling();
                let buf: *mut [T] = core::ptr::slice_from_raw_parts_mut(ptr.as_ptr(), len);
                unsafe {
                    (*dst).ptr = ptr;
                    (*dst).len = len;
                    (*dst).cap = const { if size_of::<T>() == 0 { usize::MAX } else { 0 } };
                    self.slice_initializer.init(buf, extra)?;
                }
                Ok(())
            } else if len <= N {
                // in-place
                let buf = unsafe { &raw mut (*dst).buffer };
                let ptr = NonNull::new(buf).unwrap().cast();
                let buf: *mut [T] = core::ptr::slice_from_raw_parts_mut(buf.cast(), len);
                unsafe {
                    (*dst).ptr = ptr;
                    (*dst).len = len;
                    (*dst).cap = N;
                    self.slice_initializer.init(buf, extra)?;
                }
                Ok(())
            } else {
                // on the heap
                let buf = in_place_init::try_new_boxed::<[T], Error>(in_place_init::with_extra(
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
}

mod sso {
    use core::fmt;
    use std::pin::Pin;

    #[pin_project::pin_project]
    #[repr(transparent)]
    pub struct ShortString<const N: usize>(#[pin] crate::sbo::ShortBuffer<N, u8>);

    impl<const N: usize> ShortString<N> {
        pub fn as_str(&self) -> &str {
            let utf8 = self.0.as_slice();
            // SAFETY: a ShortString always contains valid UTF-8 data.
            unsafe { core::str::from_utf8_unchecked(utf8) }
        }
        pub fn as_mut_str(self: Pin<&mut Self>) -> &str {
            let utf8 = self.project().0.as_mut_slice();
            // SAFETY: a ShortString always contains valid UTF-8 data.
            unsafe { core::str::from_utf8_unchecked_mut(utf8) }
        }
        pub fn push_str(self: Pin<&mut Self>, s: &str) {
            self.project().0.extend_copy(s.as_bytes());
        }
    }

    impl<const N: usize> fmt::Write for Pin<&mut ShortString<N>> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.as_mut().push_str(s);
            Ok(())
        }
    }

    impl<const N: usize> fmt::Write for Pin<Box<ShortString<N>>> {
        fn write_str(&mut self, s: &str) -> std::fmt::Result {
            self.as_mut().push_str(s);
            Ok(())
        }
    }

    impl<const N: usize> fmt::Display for ShortString<N> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.as_str().fmt(f)
        }
    }

    impl<const N: usize> fmt::Debug for ShortString<N> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.as_str().fmt(f)
        }
    }
}
fn main() {
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
    println!("{buffer:?}");
    buffer.as_mut().push("Steve".to_owned());
    println!("{buffer:?}");
    drop(buffer.as_mut().take_vec().unwrap());
    buffer.as_mut().push("my".to_owned());
    buffer.as_mut().push("good".to_owned());
    buffer.as_mut().push("friend".to_owned());
    println!("{buffer:?}");
    drop(buffer);
}
