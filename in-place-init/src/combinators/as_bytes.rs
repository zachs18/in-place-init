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

unsafe impl<Error, Extra, I: PinInit<str, Error, Extra>> PinInit<[u8], Error, Extra>
    for AsBytes<I>
{
    fn metadata(&self) -> <[u8] as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut [u8], extra: Extra) -> Result<(), Error> {
        unsafe { self.init.init(dst as *mut str, extra) }
    }
}
unsafe impl<Error, Extra, I: Init<str, Error, Extra>> Init<[u8], Error, Extra> for AsBytes<I> {}
