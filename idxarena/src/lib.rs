//! Arena allocation and interning where elements are referred to by integer indexes.
//!
//! By avoiding lifetimes, this tends to be more convenient than allocators like [`bumpalo::Bump`] and [`typed_arena::Arena`].
//!
//! It is similar to the functionality provided by [`thunderdome::Arena`], [`slotmap::SlotMap`], [`slab::Slab`] and [`la_arena::Arena`], with the following notable differences:
//! - Indexes are plain integers without a version counter.
//!   This is contrast to "generational" arenas like `thunderdome` and `slotmap`.
//!   This makes integers indexes smaller but provides no protection against the [ABA problem].
//!   This is less of an issue as [`IndexedArena`] doesn't currently support deleting individual elements.
//! - Indexes are not stable across deletions, unlike `slotmap`, `slab` and `thunderdome`.
//!   This is not actually a problem right now, as [`IndexedArena`] doesn't currently support deletion.
//! - Allocation in the arena doesn't require a mutable reference, unlike `thunderdome`, `slotmap`, `slab`, and `la_arena`.
//!   This allows borrowed references to live for the entire lifetime of the arena, just like with [`bumpalo::Bump`].
//!   - It follows that the arena is `!Sync`, as it uses thread-unsafe interior mutability.
//!
//! [`bumpalo::Bump`]: https://docs.rs/bumpalo/3/bumpalo/struct.Bump.html
//! [`typed_arena::Arena`]: https://docs.rs/typed-arena/2/typed_arena/struct.Arena.html
//! [`typed_generational_arena::Arena`]: https://docs.rs/typed-generational-arena/0.2/typed_generational_arena/struct.Arena.html
//! [`thunderdome::Arena`]: https://docs.rs/thunderdome/latest/thunderdome/struct.Arena.html
//! [`slab::Slab`]: https://docs.rs/slab/0.4/slab/struct.Slab.html
//! [`slotmap::SlotMap`]: https://docs.rs/slotmap/1/slotmap/struct.SlotMap.html
//! [`la_arena::Arena`]: https://docs.rs/la-arena/0.3/la_arena/struct.Arena.html
//! [ABA problem]: https://en.wikipedia.org/wiki/ABA_problem

#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

pub use arena::IndexedArena;
#[cfg(feature = "intern")]
pub use intern::InternedArena;

mod arena;
mod frozen;
#[cfg(feature = "intern")]
pub mod intern;
mod polyfill;
mod sealed;

pub(crate) type RawArena = bumpalo::Bump;

/// A type that can be arena allocated given a specified source type `U`.
///
/// This trait is currently sealed, preventing users from implementing it themselves.
pub trait ArenaAllocFrom<U>: sealed::AllocFromImpl<U> {}
impl<T> ArenaAllocFrom<T> for T {}
impl<T: Clone> ArenaAllocFrom<&T> for T {}
impl<T: Copy> ArenaAllocFrom<&[T]> for [T] {}
impl<T> ArenaAllocFrom<Box<[T]>> for [T] {}
impl<T> ArenaAllocFrom<Vec<T>> for [T] {}
impl ArenaAllocFrom<&str> for str {}

/// Implements [`ArenaAllocFrom`] on a reference to a slice,
/// by cloning each element.
///
/// This is less efficient than the default impl for `&[T]` which uses `T: Copy`.
#[derive(Copy, Clone, Debug)]
pub struct CloneSlice<'a, T>(pub &'a [T]);
impl<T: Clone> ArenaAllocFrom<CloneSlice<'_, T>> for [T] {}
