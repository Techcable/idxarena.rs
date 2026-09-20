// SPDX-FileCopyrightText: Copyright 2026 Techcable <https://techcable.net>
//
// SPDX-License-Identifier: LicenseRef-DuckLogic-All-Rights-Reserved

use core::fmt::{Debug, Formatter};
use core::marker::PhantomData;
use core::mem::ManuallyDrop;
use core::ops::{Index, IndexMut};
use core::ptr::NonNull;

use intid::IntegerId;

use crate::frozen::FrozenVec;
use crate::{ArenaAllocFrom, RawArena, sealed};

#[repr(transparent)]
struct ItemDropGuard<V: ?Sized>(NonNull<V>);
impl<V: ?Sized> ItemDropGuard<V> {
    #[inline]
    fn defuse(self) -> NonNull<V> {
        let this = ManuallyDrop::new(self);
        this.0
    }
}
impl<V: ?Sized> Drop for ItemDropGuard<V> {
    #[inline]
    fn drop(&mut self) {
        // SAFETY: We logically own the underlying value
        unsafe { core::ptr::drop_in_place(self.0.as_ptr()) }
    }
}

/// An arena allocator where elements are referred to by integer keys `K` rather than references.
pub struct IndexedArena<K: IntegerId, V: ?Sized> {
    items: FrozenVec<NonNull<V>>,
    arena: RawArena,
    marker: PhantomData<(K, V)>,
}
impl<K: IntegerId, V: ?Sized> Default for IndexedArena<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
impl<K: IntegerId, V: ?Sized> IndexedArena<K, V> {
    /// Create a new arena without any elements.
    #[inline]
    #[must_use]
    pub fn new() -> Self {
        IndexedArena {
            items: FrozenVec::new(),
            arena: RawArena::new(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn get_ptr(&self, key: K) -> Option<NonNull<V>> {
        let index = primint::to_usize_checked(K::to_int(key))?;
        self.items.get(index)
    }

    /// Get a reference to the element associated with the specified key,
    /// or `None` if the key is out of bounds/invalid.
    #[inline]
    #[must_use]
    pub fn get(&self, key: K) -> Option<&V> {
        unsafe {
            // SAFETY: We are careful not to invalidate allocated pointers
            Some(self.get_ptr(key)?.as_ref())
        }
    }

    /// Get a mutable reference to the element associated with the specified key,
    /// or `None` if the key is out of bounds/invalid.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, key: K) -> Option<&mut V> {
        unsafe {
            // SAFETY: We are careful not to invalidate allocated pointers
            // We have exclusive access to the arena, preventing aliasing
            Some(self.get_ptr(key)?.as_mut())
        }
    }

    /// Get the value associated with the specified index.
    ///
    /// # Panics
    /// Panics if the specified key is out of bounds for this arena.
    #[inline]
    #[track_caller]
    pub fn resolve(&self, key: K) -> &V {
        self.get(key).unwrap_or_else(|| missing())
    }

    /// Get a mutable reference to the value associated with the specified index.
    ///
    /// # Panics
    /// Panics if the specified key is out of bounds for this arena.
    #[inline]
    #[track_caller]
    pub fn resolve_mut(&mut self, key: K) -> &mut V {
        self.get_mut(key).unwrap_or_else(|| missing())
    }

    /// Allocate a new value in the arena,
    /// returning the key `K` pointing to the resulting item.
    ///
    /// By using the [`ArenaAllocFrom`] trait,
    /// allocations can be deferred until actually necessary.
    #[track_caller]
    #[must_use]
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn alloc<U>(&self, src: U) -> K
    where
        V: ArenaAllocFrom<U>,
    {
        let new_index = self.items.len();
        let new_key = primint::from_usize_checked(new_index)
            .and_then(|index| K::from_int_checked(index))
            .expect("indexes exhausted, overflowing key type");
        // ensure Vec has sufficient capacity before allocating in the underlying arena
        // this avoids leaking memory if Vec::push() fails
        self.items.reserve(1);
        let allocated = ItemDropGuard(<V as sealed::AllocFromImpl<U>>::alloc_from(&self.arena, src));
        let needs_len_check = V::ALLOC_MAY_RECURSE || cfg!(debug_assertions);
        // while this does not currently trigger any UB,
        // it would cause us to return an invalid index
        // it still violates the safety contract of AllocFromImpl
        if needs_len_check {
            assert_eq!(self.items.len(), new_index, "allocation recursed into itself");
        }
        // should not fail since we reserved the memory first.
        self.items.push(allocated.defuse());
        new_key
    }

    /// The number of elements that have been allocated in the arena.
    #[inline]
    #[must_use]
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Remove all items that have been allocated in this arena,
    /// while keeping the allocated memory for reuse.
    pub fn reset(&mut self) {
        {
            let items = self.items.get_mut();
            let items_ptr = items.as_ptr();
            let prev_len = items.len();
            // SAFETY: Always safe to forget all the elements
            unsafe {
                items.set_len(0);
            }
            // SAFETY: We haven't reset the underlying arena yet, so memory is still valid
            let prev_items = unsafe { core::slice::from_raw_parts(items_ptr, prev_len) };
            // SAFETY: We just forgot the elements from the vec to prevent a double free
            unsafe {
                drop_items(prev_items);
            }
        }
        self.arena.reset();
    }
}
static_assertions::assert_impl_all!(RawArena: Send);
static_assertions::assert_impl_all!(FrozenVec<u32>: Send);
// SAFETY: All our components are Send and we only use pointers for self-reference
// The `RawArena` guarantees that pointers are never deallocated until `reset()`
unsafe impl<K: IntegerId, V: ?Sized> Send for IndexedArena<K, V> where V: Send {}
static_assertions::assert_not_impl_any!(RawArena: Sync);
static_assertions::assert_not_impl_any!(FrozenVec<u32>: Sync);
// we cannot be Sync because RawArena and FrozenVec aren't Sync
static_assertions::assert_not_impl_any!(IndexedArena<u32, u32>: Sync);
impl<K: IntegerId, V: ?Sized + Debug> Debug for IndexedArena<K, V> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        // debug nothing for now
        f.debug_struct("IndexedArena").finish_non_exhaustive()
    }
}

#[cold]
#[inline(never)]
fn missing() -> ! {
    panic!("arena is missing value for key")
}

impl<K: IntegerId, V: ?Sized> Drop for IndexedArena<K, V> {
    fn drop(&mut self) {
        // SAFETY: We run item drop before we drop the actual arena
        unsafe { drop_items(self.items.get_mut().as_slice()) }
    }
}

#[cfg_attr(feature = "inline-more", inline)]
unsafe fn drop_items<T: ?Sized>(items: &[NonNull<T>]) {
    if core::mem::needs_drop::<T>() {
        for item in items {
            // SAFETY: Guaranteed by the caller to be safe
            unsafe { core::ptr::drop_in_place(item.as_ptr()) }
        }
    }
}
impl<K: IntegerId, V: ?Sized> Index<K> for IndexedArena<K, V> {
    type Output = V;

    #[inline]
    #[track_caller]
    fn index(&self, index: K) -> &Self::Output {
        self.resolve(index)
    }
}
impl<K: IntegerId, V: ?Sized> IndexMut<K> for IndexedArena<K, V> {
    #[inline]
    #[track_caller]
    fn index_mut(&mut self, index: K) -> &mut V {
        self.resolve_mut(index)
    }
}

#[cfg(test)]
mod test {
    use alloc::rc::{Rc, Weak};
    use core::cell::Cell;

    use crate::IndexedArena;

    struct CountDrops(Rc<Cell<u32>>);
    impl Drop for CountDrops {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    fn setup_drop_counts(expected_count: usize) -> (Rc<Cell<u32>>, IndexedArena<u32, CountDrops>) {
        let arena = IndexedArena::<u32, CountDrops>::new();
        let counter = Rc::new(Cell::new(0));
        for _ in 0..expected_count {
            let _ = arena.alloc(CountDrops(Rc::clone(&counter)));
        }
        assert_eq!(counter.get(), 0);
        (counter, arena)
    }

    #[test]
    fn count_drops() {
        let (counter, arena) = setup_drop_counts(5);
        drop(arena);
        assert_eq!(counter.get(), 5);
    }

    #[test]
    fn count_drops_reset() {
        let (counter, mut arena) = setup_drop_counts(5);
        arena.reset();
        assert_eq!(counter.get(), 5);
        drop(arena);
        assert_eq!(counter.get(), 5);
    }

    #[test]
    fn strings() {
        let mut arena = IndexedArena::<u32, str>::new();
        let first = arena.alloc("foo");
        let second = arena.alloc("bar");
        let third = arena.alloc("baz");
        assert_eq!(&arena[first], "foo");
        assert_eq!(&arena[second], "bar");
        assert_eq!(&arena[third], "baz");
        arena[third].make_ascii_uppercase();
        assert_eq!(&arena[third], "BAZ");
    }

    #[test]
    fn slices_owned() {
        type Slice = [Box<u32>];
        fn slice(items: &[u32]) -> Box<Slice> {
            items
                .iter()
                .copied()
                .map(Box::new)
                .collect::<Vec<_>>()
                .into_boxed_slice()
        }
        fn items(s: &Slice) -> Vec<u32> {
            s.iter().map(|x| **x).collect()
        }
        let mut arena = IndexedArena::<u32, Slice>::new();
        // consuming ownership avoids need for copy/clone
        let x = arena.alloc(slice(&[1, 2, 3]));
        // a Vec can achieve the same thing as a boxed slice
        let y = arena.alloc(slice(&[4, 5, 6, 7]).into_vec());
        let z_orig = slice(&[8, 9, 10]);
        let z_orig: &Slice = &z_orig;
        // we can explicitly request Clone from ref if the element type offers it
        let z = arena.alloc(crate::CloneSlice(z_orig));
        assert_eq!(items(&arena[x]), vec![1, 2, 3]);
        assert_eq!(items(&arena[y]), vec![4, 5, 6, 7]);
        assert_eq!(items(&arena[z]), vec![8, 9, 10]);
        arena[z].clone_from_slice(&slice(&[50, 60, 70]));
        assert_eq!(items(&arena[z]), vec![50, 60, 70]);
    }

    struct ReentrantAlloc {
        count: u32,
        // true if doing while clone, false if doing while drop
        is_clone: bool,
        arena: Weak<IndexedArena<u32, Self>>,
    }
    impl ReentrantAlloc {
        fn do_clone(&self, delta: i32) -> Self {
            ReentrantAlloc {
                count: self.count.checked_add_signed(delta).unwrap(),
                is_clone: self.is_clone,
                arena: self.arena.clone(),
            }
        }
        fn do_reentrant_alloc(&self, allow_failure: bool) {
            if self.count > 0 {
                let child = self.do_clone(-1);
                let Some(arena) = self.arena.upgrade() else {
                    if allow_failure {
                        return;
                    } else {
                        panic!("arena unexpectedly deallocated")
                    }
                };
                let _ = arena.alloc(child);
            }
        }
    }
    impl Clone for ReentrantAlloc {
        fn clone(&self) -> Self {
            if self.is_clone {
                self.do_reentrant_alloc(false /* cannot fail */);
            }
            self.do_clone(0)
        }
    }
    impl Drop for ReentrantAlloc {
        fn drop(&mut self) {
            if !self.is_clone {
                self.do_reentrant_alloc(true /* allow failure */);
            }
        }
    }
    #[test]
    #[should_panic = "allocation recursed"]
    fn alloc_reentrant_clone() {
        let arena = Rc::new(IndexedArena::new());
        for x in 1..=5 {
            let r: &ReentrantAlloc = &ReentrantAlloc {
                arena: Rc::downgrade(&arena),
                count: x,
                is_clone: true,
            };
            let _ = arena.alloc(r);
        }
        drop(arena);
    }

    #[test]
    fn alloc_reentrant_drop() {
        let arena = Rc::new(IndexedArena::new());
        for x in 1..=5 {
            let r: ReentrantAlloc = ReentrantAlloc {
                arena: Rc::downgrade(&arena),
                count: x,
                is_clone: false,
            };
            // reentrant drop shouldn't be an issue as we always move or fail allocation
            let _ = arena.alloc(r);
        }
        drop(arena);
    }
}
