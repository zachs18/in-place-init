use std::{marker::PhantomPinned, pin::Pin, rc::Weak};

use in_place_init::PinInit;

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

    #[inline(never)]
    pub fn fooo() -> Box<[[[usize; 1024]; 1024]]> {
        in_place_init::new_boxed(in_place_init::array_for_each::<128, _>(|x| {
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
        pub fn baaar() -> Box<[[[usize; 1024]; 1024]]> {
            in_place_init::new_boxed(in_place_init::Zeroed::new_zeroable_slice(128))
        }
        let bx = baaar();

        println!("{bx:p}");
        drop(bx);
    }
}
