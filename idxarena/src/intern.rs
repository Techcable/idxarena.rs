// SPDX-FileCopyrightText: Copyright 2026 Techcable <https://techcable.net>
//
// SPDX-License-Identifier: LicenseRef-DuckLogic-All-Rights-Reserved

//! Defines the [`InternedArena`] type.

use core::cell::RefCell;
use core::hash::{BuildHasher, Hash};

use equivalent::Equivalent;
use hashbrown::HashTable;
use hashbrown::hash_table::Entry;
use intid::IntegerId;

use crate::{IndexedArena, sealed};

/// A type that supports use in an [`InternedArena`].
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait Internable: Eq + Hash + sealed::SealedInternable {}
impl<T: ?Sized + Eq + Hash> Internable for T {}
impl<T: ?Sized + Eq + Hash> sealed::SealedInternable for T {}

/// The default hasher for an [`InternedArena`],
///
/// This currently matches [`hashbrown::DefaultHasher`],
/// but this may change in the future.
///
/// [`hashbrown::DefaultHasher`]: https://docs.rs/hashbrown/0.17/hashbrown/struct.DefaultHasher.html
pub type DefaultHasher = foldhash::fast::RandomState;

/// Indicates that a type can be interned from a value `U`.
///
/// For now this trait is effectively sealed,
/// as it depends on the sealed traits [`crate::ArenaAllocFrom`] and [`Internable`].
pub trait InternFrom<U>: Internable + crate::ArenaAllocFrom<U> {}

/// An [`IndexedArena`] that avoids duplicate allocations of the same elements.
///
/// This saves memory and allows cheap comparisons by indexes,
/// but means that items can't sensibly be mutated after they are allocated.
pub struct InternedArena<K: IntegerId, V: ?Sized + Internable, H: BuildHasher = DefaultHasher> {
    arena: IndexedArena<K, V>,
    map: RefCell<HashTable<K>>,
    hasher: H,
}
impl<K: IntegerId, V: ?Sized + Internable, H: BuildHasher + Default> Default for InternedArena<K, V, H> {
    fn default() -> Self {
        Self::with_hasher(H::default())
    }
}

impl<K: IntegerId, V: ?Sized + Internable> InternedArena<K, V> {
    /// Create a new arena with no elements.
    #[inline]
    pub fn new() -> Self {
        Self::with_hasher(DefaultHasher::default())
    }
}
impl<K: IntegerId, V: ?Sized + Internable, H: BuildHasher> InternedArena<K, V, H> {
    /// Create a new arena using the specified hasher.
    #[inline]
    pub fn with_hasher(hasher: H) -> Self {
        InternedArena {
            arena: crate::IndexedArena::new(),
            map: RefCell::new(HashTable::new()),
            hasher,
        }
    }

    /// Get the value associated with the specified key.
    ///
    /// # Panics
    /// Panics if the key is invalid/out of bounds.
    #[inline]
    #[track_caller]
    pub fn resolve(&self, key: K) -> &V {
        self.arena.resolve(key)
    }

    /// Get the value associated with the specified key,
    /// returning `None` if the key is invalid or out of bounds.
    #[inline]
    pub fn get(&self, key: K) -> Option<&V> {
        self.arena.get(key)
    }

    /// Allocate the specified value into the arena if necessary,
    /// while reusing an existing key if the value is already present.
    ///
    /// # Panics
    /// If the `Eq` or `Hash` implementations of the invoked type
    /// attempt to recursively invoke this method,
    /// it will cause a panic.
    /// This is a corner case unlikely to actually occur with well-behaved code.
    #[track_caller]
    #[cfg_attr(feature = "inline-more", inline)]
    pub fn intern<U>(&self, source: U) -> K
    where
        V: InternFrom<U>,
        U: Equivalent<V>,
        U: Hash,
    {
        // The U: Equivalent<V> trait requires hashes to behave identically for U & V
        let hash = self.hasher.hash_one(&source);
        // The use of a RefCell here implicitly guards against recursion
        // It should be dwarfed by the cost of hashmap lookup
        let Ok(mut map) = self.map.try_borrow_mut() else {
            panic!("recursive intern")
        };
        match map.entry(
            hash,
            |&other_key| source.equivalent(self.resolve(other_key)),
            |&other_key| self.hasher.hash_one(self.resolve(other_key)),
        ) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                #[allow(clippy::incompatible_msrv)] // we use a polyfill
                crate::polyfill::hint::cold_path();
                let new_key = self.arena.alloc(source);
                entry.insert(new_key);
                new_key
            }
        }
    }

    /// Remove all items present in this arena,
    /// while keeping the allocated memory for reuse.
    pub fn reset(&mut self) {
        self.arena.reset();
        self.map.get_mut().clear();
    }
}
static_assertions::assert_impl_all!(IndexedArena<u32, str>: Send);
static_assertions::assert_not_impl_any!(IndexedArena<u32, str>: Sync);
