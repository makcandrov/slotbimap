# slotbimap

A bidirectional `key <-> id` store: a [`slotmap`](https://docs.rs/slotmap) whose
entries can also be looked up by key, and a hash index whose entries can also be
looked up by their stable id.

Inserting a key hands back a cheap `Copy` id. Lookups by id skip hashing
entirely; lookups by key go through the hash index as usual.

```rust
use slotbimap::SlotBimap;

let mut map: SlotBimap<String, u32> = SlotBimap::new();

// `insert` returns the replaced value (if any) alongside the id.
let alice = map.insert("alice".into(), 30).id();

// Look up by id, or by key.
assert_eq!(map.get(alice), Some(&30));
assert_eq!(map.get_key(alice), Some(&"alice".to_string()));
assert_eq!(map.get_id(&"alice".to_string()), Some(alice));

// Ids stay valid across mutations.
*map.get_mut(alice).unwrap() += 1;
assert_eq!(map.get(alice), Some(&31));

// Entry API, keyed by key.
let bob = map.get_or_insert("bob".into(), 25).id();
assert_eq!(map.get_key(bob), Some(&"bob".to_string()));
assert_eq!(map.len(), 2);

assert_eq!(map.remove(alice), Some(31));
assert_eq!(map.get_id(&"alice".to_string()), None);
```

## Hashers

`SlotBimap` is generic over its `BuildHasher`, defaulting to hashbrown's
foldhash — fast, but not resistant to hash-flooding. If keys come from untrusted
input, swap in std's `RandomState`:

```rust
use std::hash::RandomState;
use slotbimap::{DefaultKey, SlotBimap};

let map: SlotBimap<String, u32, DefaultKey, RandomState> = SlotBimap::new();
```
