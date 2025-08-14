use crate::{Init, PinInit};

/// Initialize a `str` as a `[u8]`.
#[derive(Clone, Copy)]
pub struct AsUtf8<I> {
    init: I,
}

impl<I> AsUtf8<I> {
    pub fn new(init: I) -> Self {
        Self { init }
    }
}

unsafe impl<Error: From<core::str::Utf8Error>, Extra, I: PinInit<[u8], Error, Extra>>
    PinInit<str, Error, Extra> for AsUtf8<I>
{
    fn metadata(&self) -> <[u8] as core::ptr::Pointee>::Metadata {
        self.init.metadata()
    }

    unsafe fn init(self, dst: *mut str, extra: Extra) -> Result<(), Error> {
        let dst = dst as *mut [u8];
        // SAFETY: discharged to caller, except for UTF-8 requirement which we check later
        unsafe { self.init.init(dst, extra) }?;
        // SAFETY: `*dst` is a fully initialized `[u8]`
        core::str::from_utf8(unsafe { &*dst })?;
        // SAFETY: `*dst` is valid UTF-8
        Ok(())
    }
}
unsafe impl<Error: From<core::str::Utf8Error>, Extra, I: Init<[u8], Error, Extra>>
    Init<str, Error, Extra> for AsUtf8<I>
{
}
