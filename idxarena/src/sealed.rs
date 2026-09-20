//! Sealed internal traits.
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::alloc::Layout;
use core::ptr::NonNull;

use super::{CloneSlice, RawArena};

#[cfg(feature = "intern")]
pub trait SealedInternable {}

/// Allows allocation in an [`crate::IndexedArena`] given a specific source type `T`.
///
/// # Safety
/// The `alloc` function must only allocate from the specified arena.
/// If `MAY_RECURSE` is false, the alloc function must not call user-controlled code.
pub unsafe trait AllocFromImpl<T> {
    /// Determines if calling [`Self::alloc_from`] could potentially recurse back into the allocator.
    const ALLOC_MAY_RECURSE: bool;
    fn from_ref(val: &T) -> &Self;
    fn alloc_from(arena: &RawArena, val: T) -> NonNull<Self>;
}
// SAFETY: Allocates properly
unsafe impl<T> AllocFromImpl<T> for T {
    const ALLOC_MAY_RECURSE: bool = false;

    #[inline]
    fn from_ref(val: &T) -> &Self {
        val
    }
    #[inline] // always marked inline to avoid overhead of extra move
    fn alloc_from(arena: &RawArena, src: T) -> NonNull<Self> {
        NonNull::from(arena.alloc(src))
    }
}
// SAFETY: Allocates properly
unsafe impl<T: Clone> AllocFromImpl<&T> for T {
    const ALLOC_MAY_RECURSE: bool = true; // Clone can run arbitrary user code

    #[inline]
    fn from_ref<'a>(val: &'a &T) -> &'a Self {
        val
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, src: &T) -> NonNull<Self> {
        NonNull::from(arena.alloc_with(|| src.clone()))
    }
}
// SAFETY: Allocates properly
unsafe impl<T: Copy> AllocFromImpl<&[T]> for [T] {
    const ALLOC_MAY_RECURSE: bool = false; // a Copy never calls user code, and we never drop anything either
    #[inline]
    fn from_ref<'a>(val: &'a &[T]) -> &'a Self {
        val
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, val: &[T]) -> NonNull<Self> {
        NonNull::from(arena.alloc_slice_copy(val))
    }
}
// SAFETY: Allocates properly
unsafe impl<T> AllocFromImpl<Box<[T]>> for [T] {
    // same reason as why this is false for AllocFromImpl for Vec
    const ALLOC_MAY_RECURSE: bool = false;

    #[inline]
    fn from_ref(val: &Box<[T]>) -> &Self {
        val
    }
    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, val: Box<[T]>) -> NonNull<Self> {
        Self::alloc_from(arena, val.into_vec())
    }
}
// SAFETY: Allocates properly
unsafe impl<T> AllocFromImpl<Vec<T>> for [T] {
    // drop impl cannot recurse because we call set_len(0) first
    const ALLOC_MAY_RECURSE: bool = false;

    #[inline]
    fn from_ref(val: &Vec<T>) -> &Self {
        val
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, mut val: Vec<T>) -> NonNull<Self> {
        let len = val.len();
        let layout = Layout::array::<T>(val.len()).expect("alloc overflow");
        let dest = arena.alloc_layout(layout).cast::<T>();
        let src = NonNull::from(val.as_slice()).cast::<T>();
        // SAFETY: Valid to copy since the result is just a pointer
        unsafe {
            dest.as_ptr().copy_from_nonoverlapping(src.as_ptr(), val.len());
        }
        // Officially Take ownership of elements and drop the memory
        // SAFETY: Always okay to forget elements
        unsafe {
            val.set_len(0);
        }
        drop(val);
        NonNull::slice_from_raw_parts(dest, len)
    }
}
// SAFETY: Allocates properly
unsafe impl AllocFromImpl<&str> for str {
    const ALLOC_MAY_RECURSE: bool = false; // will never recurse because we just Copy and str has no Drop

    #[inline]
    fn from_ref<'a>(val: &'a &str) -> &'a Self {
        val
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, val: &str) -> NonNull<Self> {
        NonNull::from(arena.alloc_str(val))
    }
}
// SAFETY: Allocates properly
unsafe impl<T: Clone> AllocFromImpl<CloneSlice<'_, T>> for [T] {
    const ALLOC_MAY_RECURSE: bool = true; // Clone is user-controlled

    #[inline]
    fn from_ref<'a>(val: &'a CloneSlice<T>) -> &'a Self {
        val.0
    }

    #[cfg_attr(feature = "inline-more", inline)]
    fn alloc_from(arena: &RawArena, val: CloneSlice<T>) -> NonNull<Self> {
        NonNull::from(arena.alloc_slice_clone(val.0))
    }
}
