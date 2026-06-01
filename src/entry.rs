use std::hash::{BuildHasher, Hash};

use slotmap::Key;

use crate::{Record, SlotBimap, WithId};

/// A view into a single entry of a [`SlotBimap`], obtained from [`SlotBimap::entry`].
#[derive(Debug)]
pub enum Entry<'a, K, V, I: Key, S> {
    Occupied(OccupiedEntry<'a, K, V, I, S>),
    Vacant(VacantEntry<'a, K, V, I, S>),
}

/// An entry for a key that is already present in the store.
#[derive(Debug)]
pub struct OccupiedEntry<'a, K, V, I: Key, S> {
    store: &'a mut SlotBimap<K, V, I, S>,
    id: I,
}

/// An entry for a key that is not yet present in the store.
#[derive(Debug)]
pub struct VacantEntry<'a, K, V, I: Key, S> {
    store: &'a mut SlotBimap<K, V, I, S>,
    hash: u64,
    key: K,
}

impl<'a, K, V, I: Key, S> OccupiedEntry<'a, K, V, I, S> {
    /// Returns the id of this entry.
    pub fn id(&self) -> I {
        self.id
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        &self.store.data[self.id].key
    }

    /// Returns a reference to this entry's value.
    pub fn get(&self) -> &V {
        &self.store.data[self.id].value
    }

    /// Returns a mutable reference to this entry's value.
    pub fn get_mut(&mut self) -> &mut V {
        &mut self.store.data[self.id].value
    }

    /// Converts the entry into a mutable reference to its value, bound to the
    /// store's lifetime.
    pub fn into_mut(self) -> &'a mut V {
        &mut self.store.data[self.id].value
    }

    /// Replaces this entry's value, returning the old one.
    pub fn insert(&mut self, value: V) -> V {
        self.store.data[self.id].replace_value(value)
    }
}

impl<'a, K, V, I, S> OccupiedEntry<'a, K, V, I, S>
where
    K: Hash + Eq,
    I: Key,
    S: BuildHasher,
{
    /// Removes the entry from the store, returning its value.
    pub fn remove(self) -> V {
        self.store
            .remove(self.id)
            .expect("occupied entry id is always present")
    }
}

impl<'a, K, V, I, S> VacantEntry<'a, K, V, I, S>
where
    K: Hash + Eq,
    I: Key,
    S: BuildHasher,
{
    /// Returns a reference to the key that would be inserted.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Takes ownership of the key, consuming the entry.
    pub fn into_key(self) -> K {
        self.key
    }

    /// Inserts `value` for the entry's key, returning a mutable reference to it
    /// alongside the freshly assigned id.
    pub fn insert(self, value: V) -> WithId<&'a mut V, I> {
        let VacantEntry { store, hash, key } = self;
        let id = store.data.insert(Record { key, value });
        store
            .index
            .insert_unique(hash, id, |&id| store.hasher.hash_one(&store.data[id].key));
        WithId::new(id, &mut store.data[id].value)
    }
}

impl<'a, K, V, I, S> Entry<'a, K, V, I, S>
where
    K: Hash + Eq,
    I: Key,
    S: BuildHasher,
{
    pub(crate) fn occupied(store: &'a mut SlotBimap<K, V, I, S>, id: I) -> Self {
        Self::Occupied(OccupiedEntry { store, id })
    }

    pub(crate) fn vacant(store: &'a mut SlotBimap<K, V, I, S>, hash: u64, key: K) -> Self {
        Self::Vacant(VacantEntry { store, hash, key })
    }

    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &K {
        match self {
            Entry::Occupied(e) => e.key(),
            Entry::Vacant(e) => e.key(),
        }
    }

    /// Returns the id of this entry, or `None` if it is vacant.
    pub fn id(&self) -> Option<I> {
        match self {
            Entry::Occupied(e) => Some(e.id()),
            Entry::Vacant(_) => None,
        }
    }

    /// Runs `f` against the value if the entry is occupied, leaving a vacant
    /// entry untouched.
    pub fn and_modify(mut self, f: impl FnOnce(&mut V)) -> Self {
        if let Entry::Occupied(e) = &mut self {
            f(e.get_mut());
        }
        self
    }

    /// Ensures the entry is occupied, inserting `default` if it is vacant.
    pub fn or_insert(self, default: V) -> WithId<&'a mut V, I> {
        self.or_insert_with(|| default)
    }

    /// Ensures the entry is occupied, inserting the result of `f` if it is
    /// vacant.
    pub fn or_insert_with(self, f: impl FnOnce() -> V) -> WithId<&'a mut V, I> {
        self.or_insert_with_key(|_| f())
    }

    /// Like [`or_insert_with`](Self::or_insert_with), but `f` receives a
    /// reference to the key being inserted.
    pub fn or_insert_with_key(self, f: impl FnOnce(&K) -> V) -> WithId<&'a mut V, I> {
        match self {
            Entry::Occupied(e) => {
                let id = e.id();
                WithId::new(id, e.into_mut())
            }
            Entry::Vacant(e) => {
                let value = f(&e.key);
                e.insert(value)
            }
        }
    }

    /// Ensures the entry is occupied, inserting `V::default()` if it is vacant.
    pub fn or_default(self) -> WithId<&'a mut V, I>
    where
        V: Default,
    {
        self.or_insert_with(V::default)
    }

    /// Ensures the entry is occupied, inserting the result of `f` if it is
    /// vacant. Nothing is inserted if `f` returns `Err`.
    pub fn or_try_insert_with<E>(
        self,
        f: impl FnOnce() -> Result<V, E>,
    ) -> Result<WithId<&'a mut V, I>, E> {
        self.or_try_insert_with_key(|_| f())
    }

    /// Like [`or_try_insert_with`](Self::or_try_insert_with), but `f` receives a
    /// reference to the key being inserted.
    pub fn or_try_insert_with_key<E>(
        self,
        f: impl FnOnce(&K) -> Result<V, E>,
    ) -> Result<WithId<&'a mut V, I>, E> {
        match self {
            Entry::Occupied(e) => {
                let id = e.id();
                Ok(WithId::new(id, e.into_mut()))
            }
            Entry::Vacant(e) => {
                let value = f(&e.key)?;
                Ok(e.insert(value))
            }
        }
    }
}
