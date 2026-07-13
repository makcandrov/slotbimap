#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]

use std::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};

use hashbrown::HashTable;
use slotmap::SlotMap;

pub use slotmap::{DefaultKey, Key, new_key_type};

mod entry;
pub use entry::{Entry, OccupiedEntry, VacantEntry};

mod with_id;
pub use with_id::WithId;

/// Bidirectional `key <-> id` store.
#[derive(Debug)]
pub struct SlotBimap<K, V, I: Key = DefaultKey, S = hashbrown::DefaultHashBuilder> {
    data: SlotMap<I, Record<K, V>>,
    index: HashTable<I>,
    hasher: S,
}

/// The value actually stored in `data`: the interned key, its value and its id.
#[derive(Debug)]
struct Record<K, V> {
    key: K,
    value: V,
}

impl<K, V> Record<K, V> {
    #[inline]
    fn replace_value(&mut self, value: V) -> V {
        std::mem::replace(&mut self.value, value)
    }
}

impl<K, V, I, S> Default for SlotBimap<K, V, I, S>
where
    I: Key,
    S: Default,
{
    #[inline]
    fn default() -> Self {
        Self {
            data: Default::default(),
            index: Default::default(),
            hasher: Default::default(),
        }
    }
}

impl<K, V, I, S> SlotBimap<K, V, I, S>
where
    K: Hash + Eq,
    I: Key,
    S: BuildHasher,
{
    /// Creates an empty [`SlotBimap`] with a default hasher.
    #[inline]
    #[must_use]
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            data: SlotMap::with_key(),
            index: HashTable::new(),
            hasher: S::default(),
        }
    }

    /// Returns the id associated with `key`, if any.
    #[inline]
    #[must_use]
    pub fn get_id<Q>(&self, key: &Q) -> Option<I>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = self.hasher.hash_one(key);
        self.get_id_hashed(key, hash)
    }

    /// Returns the key associated with `id`, if any.
    #[inline]
    #[must_use]
    pub fn get_key(&self, id: I) -> Option<&K> {
        self.data.get(id).map(|record| &record.key)
    }

    /// Returns the id associated with `key` given its precomputed `hash`.
    #[inline]
    #[must_use]
    fn get_id_hashed<Q>(&self, key: &Q, hash: u64) -> Option<I>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.index
            .find(hash, |&id| self.data[id].key.borrow() == key)
            .copied()
    }

    /// Inserts a `key`-`value` pair into the map and returns its id.
    ///
    /// If `key` is already present, its value is replaced and the previous
    /// value is returned alongside the existing id. Otherwise a new entry is
    /// created and `None` is returned alongside the freshly allocated id.
    #[inline]
    pub fn insert(&mut self, key: K, value: V) -> WithId<Option<V>, I> {
        let hash = self.hasher.hash_one(&key);

        if let Some(id) = self.get_id_hashed(&key, hash) {
            let old = self.data[id].replace_value(value);
            WithId::new(id, Some(old))
        } else {
            let id = self.data.insert(Record { key, value });
            self.index
                .insert_unique(hash, id, |&id| self.hasher.hash_one(&self.data[id].key));
            WithId::new(id, None)
        }
    }

    /// Removes the entry identified by `id`, returning its value if present.
    #[inline]
    pub fn remove(&mut self, id: I) -> Option<V> {
        let record = self.data.remove(id)?;
        let hash = self.hasher.hash_one(&record.key);
        if let Ok(entry) = self.index.find_entry(hash, |&e| e == id) {
            entry.remove();
        }
        Some(record.value)
    }

    /// Removes the entry associated with `key`, returning its value if present.
    #[inline]
    pub fn remove_by_key<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let hash = self.hasher.hash_one(key);
        let entry = self
            .index
            .find_entry(hash, |&id| self.data[id].key.borrow() == key)
            .ok()?;
        let (id, _) = entry.remove();
        let record = self.data.remove(id).expect("indexed id is always present");
        Some(record.value)
    }

    /// Returns a reference to the value identified by `id`, if present.
    #[inline]
    #[must_use]
    pub fn get(&self, id: I) -> Option<&V> {
        self.data.get(id).map(|record| &record.value)
    }

    /// Returns a mutable reference to the value identified by `id`, if present.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, id: I) -> Option<&mut V> {
        self.data.get_mut(id).map(|record| &mut record.value)
    }

    /// Returns the value associated with `key` along with its id, if present.
    #[inline]
    #[must_use]
    pub fn get_by_key<Q>(&self, key: &Q) -> Option<WithId<&V, I>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let id = self.get_id(key)?;
        Some(WithId::new(id, &self.data[id].value))
    }

    /// Returns a mutable reference to the value associated with `key` along with
    /// its id, if present.
    #[inline]
    #[must_use]
    pub fn get_mut_by_key<Q>(&mut self, key: &Q) -> Option<WithId<&mut V, I>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let id = self.get_id(key)?;
        Some(WithId::new(id, &mut self.data[id].value))
    }

    /// Returns `true` if the map contains a value for the given `id`.
    #[inline]
    #[must_use]
    pub fn contains(&self, id: I) -> bool {
        self.data.contains_key(id)
    }

    /// Returns `true` if the map contains a value for the given `key`.
    #[inline]
    #[must_use]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_id(key).is_some()
    }

    /// Returns the number of entries in the map.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns `true` if the map contains no entries.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Gets the entry for `key`, computing the hash and lookup once so the
    /// result can be inspected or filled without a second probe.
    #[inline]
    #[must_use]
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, I, S> {
        let hash = self.hasher.hash_one(&key);
        match self.get_id_hashed(&key, hash) {
            Some(id) => Entry::occupied(self, id),
            None => Entry::vacant(self, hash, key),
        }
    }

    /// Returns the value associated with `key`, inserting the result of `f` if
    /// it is not already present. `f` is only called on insertion.
    #[inline]
    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> WithId<&mut V, I> {
        self.entry(key).or_insert_with(f)
    }

    /// Like [`get_or_insert_with`](Self::get_or_insert_with), but `f` receives a
    /// reference to the key being inserted.
    #[inline]
    pub fn get_or_insert_with_key(&mut self, key: K, f: impl FnOnce(&K) -> V) -> WithId<&mut V, I> {
        self.entry(key).or_insert_with_key(f)
    }

    /// Returns the value associated with `key`, inserting `value` if it is not
    /// already present.
    #[inline]
    pub fn get_or_insert(&mut self, key: K, value: V) -> WithId<&mut V, I> {
        self.entry(key).or_insert(value)
    }

    /// Returns the value associated with `key`, inserting `V::default()` if it
    /// is not already present.
    #[inline]
    pub fn get_or_insert_default(&mut self, key: K) -> WithId<&mut V, I>
    where
        V: Default,
    {
        self.entry(key).or_default()
    }

    /// Returns the id associated with `key`, inserting the result of `f` if it
    /// is not already present.
    #[inline]
    pub fn get_or_insert_id_with(&mut self, key: K, f: impl FnOnce() -> V) -> I {
        self.entry(key).or_insert_with(f).id()
    }

    /// Returns the value associated with `key`, inserting the result of `f` if
    /// it is not already present. `f` is only called on insertion, and nothing
    /// is inserted if it returns `Err`.
    #[inline]
    pub fn get_or_try_insert_with<E>(
        &mut self,
        key: K,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<WithId<&mut V, I>, E> {
        self.entry(key).or_try_insert_with(f)
    }

    /// Like [`get_or_try_insert_with`](Self::get_or_try_insert_with), but `f`
    /// receives a reference to the key being inserted.
    #[inline]
    pub fn get_or_try_insert_with_key<E>(
        &mut self,
        key: K,
        f: impl FnOnce(&K) -> Result<V, E>,
    ) -> Result<WithId<&mut V, I>, E> {
        self.entry(key).or_try_insert_with_key(f)
    }

    /// Returns the id associated with `key`, inserting the result of `f` if it
    /// is not already present. Nothing is inserted if `f` returns `Err`.
    #[inline]
    pub fn get_or_try_insert_id_with<E>(
        &mut self,
        key: K,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<I, E> {
        Ok(self.entry(key).or_try_insert_with(f)?.id())
    }
}
