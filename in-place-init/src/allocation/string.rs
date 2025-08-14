use core::mem::MaybeUninit;

use alloc::string::String;

use crate::Init;

pub fn try_new_string<Error>(init: impl Init<str, Error>) -> Result<String, Error> {
    crate::try_new_boxed(init).map(String::from)
}
pub fn new_string(init: impl Init<str, !>) -> String {
    String::from(crate::new_boxed(init))
}

pub trait StringExt {
    fn try_append_emplace<Error>(&mut self, init: impl Init<str, Error>) -> Result<(), Error>;
    fn append_emplace(&mut self, init: impl Init<str>);
}

impl StringExt for String {
    fn try_append_emplace<Error>(&mut self, init: impl Init<str, Error>) -> Result<(), Error> {
        let additional = init.metadata();
        self.reserve(additional);
        let len = self.len();
        unsafe {
            let this = self.as_mut_vec();
            init.init(
                &mut this.spare_capacity_mut()[..additional] as *mut [MaybeUninit<u8>] as *mut str,
                (),
            )?;
            this.set_len(len + additional);
        };
        Ok(())
    }

    fn append_emplace(&mut self, init: impl Init<str>) {
        self.try_append_emplace(init).unwrap_or_else(|e| match e {});
    }
}
