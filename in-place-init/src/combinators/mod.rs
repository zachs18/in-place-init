pub(crate) mod try_with;
pub(crate) mod with;

pub(crate) mod for_each;
pub(crate) mod for_each_with;
pub(crate) mod repeat;

pub(crate) mod fail;

pub(crate) mod map_err;

pub(crate) mod ignore_extra;
pub(crate) mod map_extra;
pub(crate) mod with_extra;

pub(crate) mod chain;

pub(crate) mod uninit;
pub(crate) mod zeroed;

pub(crate) mod for_slice;
pub(crate) mod for_type;

pub(crate) mod assert_pinned;

pub(crate) mod then;
pub(crate) mod then_pinned;

pub(crate) mod as_bytes;
pub(crate) mod as_utf8;

pub(crate) mod flatten;
