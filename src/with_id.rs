use quick_impl::quick_impl;
use slotmap::DefaultKey;

/// A value paired with the id it is associated with in a
/// [`SlotBimap`](crate::SlotBimap).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[quick_impl(pub const new)]
pub struct WithId<V, I = DefaultKey> {
    #[quick_impl(pub get_clone = "{}")]
    id: I,
    #[quick_impl(pub const get = "{}", pub const get_mut = "{}_mut", pub replace, pub into, impl Deref, impl DerefMut)]
    value: V,
}
