use std::ops::{Deref, DerefMut};

use slotmap::DefaultKey;

/// A value paired with the id it is associated with in a
/// [`SlotBimap`](crate::SlotBimap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WithId<V, I = DefaultKey> {
    id: I,
    value: V,
}

impl<V, I> WithId<V, I> {
    /// Pairs `value` with `id`.
    #[inline]
    #[must_use]
    pub const fn new(id: I, value: V) -> Self {
        Self { id, value }
    }

    /// Returns the id.
    #[inline]
    #[must_use]
    pub fn id(&self) -> I
    where
        I: Clone,
    {
        self.id.clone()
    }

    /// Returns a reference to the value.
    #[inline]
    #[must_use]
    pub const fn value(&self) -> &V {
        &self.value
    }

    /// Returns a mutable reference to the value.
    #[inline]
    #[must_use]
    pub const fn value_mut(&mut self) -> &mut V {
        &mut self.value
    }

    /// Replaces the value, returning the old one. The id is left unchanged.
    #[inline]
    pub fn replace_value(&mut self, value: V) -> V {
        std::mem::replace(&mut self.value, value)
    }

    /// Consumes the pair and returns the value, discarding the id.
    #[inline]
    #[must_use]
    pub fn into_value(self) -> V {
        self.value
    }
}

impl<V, I> Deref for WithId<V, I> {
    type Target = V;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<V, I> DerefMut for WithId<V, I> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
