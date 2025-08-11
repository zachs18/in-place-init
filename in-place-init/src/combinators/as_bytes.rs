use crate::{Init, PinInit};

/// Initialize a `[u8]` as a `str`.
#[derive(Clone, Copy)]
pub struct AsBytes<I> {
    init: I,
}

impl<I> AsBytes<I> {
    pub fn new(init: I) -> Self {
        Self { init }
    }
}

unsafe impl<Extra, E, I: PinInit<str, Extra, Error = E>> PinInit<[u8], Extra> for AsBytes<I> {
    type Error = E;

    fn metadata(&self) -> <[u8] as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut [u8], extra: Extra) -> Result<(), Self::Error> {
        unsafe { self.init.init(dst as *mut str, extra) }
    }
}
unsafe impl<Extra, E, I: Init<str, Extra, Error = E>> Init<[u8], Extra> for AsBytes<I> {}
