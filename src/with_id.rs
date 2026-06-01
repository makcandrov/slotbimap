use quick_impl::quick_impl_all;
use slotmap::DefaultKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[quick_impl_all(pub const get = "{}")]
pub struct WithId<V, I = DefaultKey> {
    id: I,
    #[quick_impl(pub const get_mut = "{}_mut", pub replace, pub into, impl Deref, impl DerefMut)]
    value: V,
}

impl<V, I> WithId<V, I> {
    pub fn new(id: I, value: V) -> Self {
        Self { id, value }
    }
}
