use alloc::vec::Vec;
use core::cell::UnsafeCell;

/// Lightweight reimplementation of [`elsa::FrozenVec`],
/// but with support for additional functionality like [`Vec::reserve`].
///
/// Always requires types to be [`Copy`] for access,
/// without offering support for the [`StableDeref`] bound.
///
/// [`elsa::FrozenVec`]: https://docs.rs/elsa/1/elsa/vec/struct.FrozenVec.html
/// [`StableDeref`]: https://docs.rs/stable_deref_trait/1/stable_deref_trait/trait.StableDeref.html
pub struct FrozenVec<T> {
    inner: UnsafeCell<Vec<T>>,
}
impl<T> FrozenVec<T> {
    #[inline]
    pub const fn new() -> Self {
        FrozenVec {
            inner: UnsafeCell::new(Vec::new()),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        // SAFETY: Careful to hold references to Vec only briefly
        unsafe { (*self.inner.get()).len() }
    }

    #[inline]
    pub fn reserve(&self, additional: usize) {
        // SAFETY: We hold references to Vec only briefly
        unsafe { (*self.inner.get()).reserve(additional) }
    }

    #[inline]
    pub fn push(&self, value: T) {
        // SAFETY: We hold references to the Vec only briefly
        unsafe { (*self.inner.get()).push(value) }
    }

    /// Get the element at the specified index,
    /// or `None` if the index is out of bounds
    ///
    /// # Safety
    /// This is safe because of the `T: Copy` bound
    #[inline]
    pub fn get(&self, index: usize) -> Option<T>
    where
        T: Copy,
    {
        // SAFETY: Returning a copy ensures that reference to Vec is held only briefly
        unsafe { (&*self.inner.get()).get(index).copied() }
    }

    /// Get mutable access to the underlying vector
    #[inline]
    pub fn get_mut(&mut self) -> &mut Vec<T> {
        self.inner.get_mut()
    }
}
