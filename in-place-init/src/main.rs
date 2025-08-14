#![feature(never_type, ptr_metadata)]

use std::{marker::PhantomPinned, pin::Pin, rc::Weak};

use in_place_init::{PinInit, VecExt};

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

unsafe impl PinInit<SelfReferential> for MakeSelfReferential {
    type Error = std::convert::Infallible;

    fn metadata(&self) {}

    unsafe fn init(self, dst: *mut SelfReferential, _: ()) -> Result<(), Self::Error> {
        unsafe {
            (*dst).value = self.0;
            (*dst).ptr = &raw mut (*dst).value;
        }
        Ok(())
    }
}

fn main() {
    let mut vec: Vec<[usize; 100]> = vec![];
    vec.push_emplace(in_place_init::array_repeat(vec.len()));
    vec.push_emplace(in_place_init::array_repeat(vec.len()));
    assert_eq!(vec, [[0; 100], [1; 100]]);

    let bx = in_place_init::new_boxed::<str>("hello, world");
    dbg!(bx);

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

    let mut sr =
        in_place_init::try_new_pinned::<SelfReferential, _>(MakeSelfReferential(42)).unwrap();
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

        let bx: Box<foo::Bar<str>> = in_place_init::new_boxed(foo::BarInit(
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
    }
}

#[cfg(feature = "macros")]
mod foo {
    #[derive(in_place_init_derive::Init)]
    struct Foo;

    #[derive(Debug, in_place_init_derive::Init)]
    pub(crate) struct Bar<T: ?Sized> {
        pub x: [u8; 1024],
        pub y: T,
    }

    #[derive(Debug, in_place_init_derive::Init)]
    pub(crate) struct Baz<T: ?Sized> {
        pub this: std::rc::Weak<Self>,
        pub tail: T,
    }

    trait Trait {
        type Assoc<U: ?Sized>;
    }
    impl<T: ?Sized> Trait for T {
        type Assoc<U: ?Sized> = u32;
    }

    #[derive(Debug, in_place_init_derive::Init)]
    pub(crate) struct Quux<T: ?Sized> {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }

    #[derive(Debug, in_place_init_derive::Init)]
    pub(crate) struct Phooey<T> where T: ?Sized, {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }

    #[derive(Debug, in_place_init_derive::Init)]
    pub(crate) struct Phenomenal<T> where T: ?Sized {
        pub foo: <Self as Trait>::Assoc<Self>,
        pub tail: T,
    }
}
