use std::hash::{BuildHasher, Hash};

use hashbrown::HashTable;
use quick_impl::quick_impl_all;
use rustc_hash::FxBuildHasher;
use slotmap::SlotMap;

pub use slotmap::{DefaultKey, Key, new_key_type};

mod entry;
pub use entry::{Entry, OccupiedEntry, VacantEntry};

mod with_id;
pub use with_id::WithId;

/// Bidirectional `key <-> id` store.
#[derive(Debug, Default)]
pub struct SlotBimap<K, V, I: Key = DefaultKey, S = FxBuildHasher> {
    data: SlotMap<I, Record<K, V>>,
    index: HashTable<I>,
    hasher: S,
}

/// The value actually stored in `data`: the interned key, its value and its id.
#[derive(Debug)]
#[quick_impl_all(pub const get = "{}")]
struct Record<K, V> {
    key: K,
    #[quick_impl(replace)]
    value: V,
}

impl<K, V, I, S> SlotBimap<K, V, I, S>
where
    K: Hash + Eq,
    I: Key,
    S: BuildHasher,
{
    pub fn new() -> Self
    where
        S: Default,
    {
        Self {
            data: SlotMap::<I, _>::with_key(),
            index: HashTable::new(),
            hasher: S::default(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> WithId<Option<V>, I> {
        let hash = self.hasher.hash_one(&key);

        let existing = self
            .index
            .find(hash, |&id| self.data[id].key == key)
            .copied();

        if let Some(id) = existing {
            let old = self.data[id].replace_value(value);
            WithId::new(id, Some(old))
        } else {
            let id = self.data.insert(Record { key, value });
            self.index
                .insert_unique(hash, id, |&id| self.hasher.hash_one(&self.data[id].key));
            WithId::new(id, None)
        }
    }

    pub fn remove(&mut self, id: I) -> Option<V> {
        let record = self.data.remove(id)?;
        let hash = self.hasher.hash_one(&record.key);
        if let Ok(entry) = self.index.find_entry(hash, |&e| e == id) {
            entry.remove();
        }
        Some(record.value)
    }

    pub fn get(&self, id: I) -> Option<&V> {
        self.data.get(id).map(|record| &record.value)
    }

    pub fn get_mut(&mut self, id: I) -> Option<&mut V> {
        self.data.get_mut(id).map(|record| &mut record.value)
    }

    pub fn get_by_key(&self, key: &K) -> Option<WithId<&V, I>> {
        let hash = self.hasher.hash_one(key);
        let &id = self.index.find(hash, |&id| &self.data[id].key == key)?;
        Some(WithId::new(id, &self.data[id].value))
    }

    pub fn get_mut_by_key(&mut self, key: &K) -> Option<WithId<&mut V, I>> {
        let hash = self.hasher.hash_one(key);
        let &id = self.index.find(hash, |&id| &self.data[id].key == key)?;
        Some(WithId::new(id, &mut self.data[id].value))
    }

    /// Gets the entry for `key`, computing the hash and lookup once so the
    /// result can be inspected or filled without a second probe.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, V, I, S> {
        let hash = self.hasher.hash_one(&key);
        match self
            .index
            .find(hash, |&id| self.data[id].key == key)
            .copied()
        {
            Some(id) => Entry::occupied(self, id),
            None => Entry::vacant(self, hash, key),
        }
    }

    /// Returns the value associated with `key`, inserting the result of `f` if
    /// it is not already present. `f` is only called on insertion.
    pub fn get_or_insert_with(&mut self, key: K, f: impl FnOnce() -> V) -> WithId<&mut V, I> {
        self.entry(key).or_insert_with(f)
    }

    /// Like [`get_or_insert_with`](Self::get_or_insert_with), but `f` receives a
    /// reference to the key being inserted.
    pub fn get_or_insert_with_key(&mut self, key: K, f: impl FnOnce(&K) -> V) -> WithId<&mut V, I> {
        self.entry(key).or_insert_with_key(f)
    }

    /// Returns the value associated with `key`, inserting `value` if it is not
    /// already present.
    pub fn get_or_insert(&mut self, key: K, value: V) -> WithId<&mut V, I> {
        self.entry(key).or_insert(value)
    }

    /// Returns the value associated with `key`, inserting `V::default()` if it
    /// is not already present.
    pub fn get_or_insert_default(&mut self, key: K) -> WithId<&mut V, I>
    where
        V: Default,
    {
        self.entry(key).or_default()
    }

    /// Returns the id associated with `key`, inserting the result of `f` if it
    /// is not already present.
    pub fn get_or_insert_id_with(&mut self, key: K, f: impl FnOnce() -> V) -> I {
        self.entry(key).or_insert_with(f).id()
    }

    /// Returns the value associated with `key`, inserting the result of `f` if
    /// it is not already present. `f` is only called on insertion, and nothing
    /// is inserted if it returns `Err`.
    pub fn get_or_try_insert_with<E>(
        &mut self,
        key: K,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<WithId<&mut V, I>, E> {
        self.entry(key).or_try_insert_with(f)
    }

    /// Like [`get_or_try_insert_with`](Self::get_or_try_insert_with), but `f`
    /// receives a reference to the key being inserted.
    pub fn get_or_try_insert_with_key<E>(
        &mut self,
        key: K,
        f: impl FnOnce(&K) -> Result<V, E>,
    ) -> Result<WithId<&mut V, I>, E> {
        self.entry(key).or_try_insert_with_key(f)
    }

    /// Returns the id associated with `key`, inserting the result of `f` if it
    /// is not already present. Nothing is inserted if `f` returns `Err`.
    pub fn get_or_try_insert_id_with<E>(
        &mut self,
        key: K,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<I, E> {
        Ok(self.entry(key).or_try_insert_with(f)?.id())
    }
}
