/// # Safety:
///
/// ## Implementors
///
/// * `length` must return the same value, if there are no intermediate modifications to `self`
/// * `length` may panic or diverge
pub unsafe trait Length {
    fn length(&self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConstLength<const N: usize>;

unsafe impl<const N: usize> Length for ConstLength<N> {
    fn length(&self) -> usize {
        N
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct RuntimeLength {
    pub length: usize,
}

unsafe impl Length for RuntimeLength {
    fn length(&self) -> usize {
        self.length
    }
}
