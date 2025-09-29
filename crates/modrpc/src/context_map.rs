use core::any::TypeId;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use std::collections::HashMap;

use hashbrown::HashTable;

use crate::WorkerId;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ModrpcContextTag {
    Role(u64),
    Worker(WorkerId),
}

pub trait ContextClass {
    type Key: Eq + Hash;
    type Params;
    fn new(params: &Self::Params) -> Self;
}

/// A map for context of any type that implements `ContextClass`. Each context class can have a
/// distinct key type whose values uniquely identify items of that context class in the map.
/// `ContextMap` requires that its items are `Send` so that it itself can be `Send`.
pub struct ContextMap(ContextMapPrivate<ModrpcContextTag, IsSend>);
unsafe impl Send for ContextMap {}

/// A map for context of any type that implements `ContextClass`. Each context class can have a
/// distinct key type whose values uniquely identify items of that context class in the map.
/// `LocalContextMap` is `!Send` and thus does **not** require that its items implement `Send`.
pub struct LocalContextMap(ContextMapPrivate<ModrpcContextTag, NotSend>);

impl ContextMap {
    pub fn new() -> Self {
        Self(ContextMapPrivate::new_inner())
    }

    pub fn with<T, U>(
        &mut self,
        tag: ModrpcContextTag,
        key: T::Key,
        params: &T::Params,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        T: ContextClass + Send + 'static,
        T::Key: Send,
    {
        self.0.with_inner(tag, key, params, f)
    }

    pub fn with_fn<K, T, U>(
        &mut self,
        tag: ModrpcContextTag,
        key: K,
        constructor: impl FnOnce() -> T,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        K: Eq + Hash + Send + 'static,
        T: Send + 'static,
    {
        self.0.with_inner_fn(tag, key, constructor, f)
    }

    pub fn shutdown_tag(&mut self, tag: ModrpcContextTag) {
        self.0.shutdown_tag(tag)
    }
}

impl LocalContextMap {
    pub fn new() -> Self {
        Self(ContextMapPrivate::new_inner())
    }

    pub fn with<T, U>(
        &mut self,
        tag: ModrpcContextTag,
        key: T::Key,
        params: &T::Params,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        T: ContextClass + 'static,
    {
        self.0.with_inner(tag, key, params, f)
    }

    pub fn with_fn<K: Eq + Hash, T: 'static, U>(
        &mut self,
        tag: ModrpcContextTag,
        key: K,
        constructor: impl FnOnce() -> T,
        f: impl FnOnce(&mut T) -> U,
    ) -> U {
        self.0.with_inner_fn(tag, key, constructor, f)
    }

    pub fn shutdown_tag(&mut self, tag: ModrpcContextTag) {
        self.0.shutdown_tag(tag)
    }
}

trait SendTag {}
struct IsSend;
struct NotSend;
impl SendTag for IsSend {}
impl SendTag for NotSend {}

struct ContextMapPrivate<Tag, S: SendTag> {
    map: HashTable<Item>,
    // Map tag to the first Item of the item chain for the tag
    tag_chains: HashMap<Tag, ItemId>,
    _send: PhantomData<S>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
struct ItemId {
    hash: u64,
    ptr: *mut (),
}

struct Item {
    type_id: TypeId,
    hash: u64,
    ptr: *mut (),
    tag_next: ItemId,
    drop_fn: unsafe fn(*mut ()),
}

struct ItemTyped<K, V> {
    key: K,
    value: V,
}

impl ItemId {
    fn null() -> Self {
        Self {
            hash: 0,
            ptr: core::ptr::null_mut(),
        }
    }
}

impl<Tag: Hash + Eq, S: SendTag> ContextMapPrivate<Tag, S> {
    fn new_inner() -> Self {
        Self {
            map: HashTable::new(),
            tag_chains: HashMap::new(),
            _send: PhantomData,
        }
    }

    fn shutdown_tag(&mut self, tag: Tag) {
        let Some(mut tag_chain_next) = self.tag_chains.remove(&tag) else {
            return;
        };

        while tag_chain_next != ItemId::null() {
            let (removed_item, _) = self
                .find_item(tag_chain_next)
                .expect("ContextMap::shutdown_tag missing item")
                .remove();
            unsafe {
                (removed_item.drop_fn)(removed_item.ptr);
            }
            tag_chain_next = removed_item.tag_next;
        }
    }

    fn find_item(
        &'_ mut self,
        item_id: ItemId,
    ) -> Option<hashbrown::hash_table::OccupiedEntry<'_, Item>> {
        self.map
            .find_entry(item_id.hash, |item| item.ptr == item_id.ptr)
            .ok()
    }

    fn with_inner<T, U>(
        &mut self,
        tag: Tag,
        key: T::Key,
        params: &T::Params,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        T: ContextClass + 'static,
    {
        let item_ptr = self.get_ptr::<T>(tag, key, params);
        let item_ref = unsafe { &mut *(item_ptr as *mut ItemTyped<T::Key, T>) };

        f(&mut item_ref.value)
    }

    fn new_item<T: ContextClass + 'static>(key: T::Key, params: &T::Params) -> Item {
        unsafe fn drop_fn<K, T>(item_ptr: *mut ()) {
            let _ = unsafe { Box::from_raw(item_ptr as *mut ItemTyped<K, T>) };
        }

        let type_id = TypeId::of::<T>();
        Item {
            type_id,
            hash: hash(type_id, &key),
            ptr: Box::into_raw(Box::new(ItemTyped {
                key,
                value: T::new(params),
            })) as *mut (),
            tag_next: ItemId::null(),
            drop_fn: drop_fn::<T::Key, T>,
        }
    }

    fn get_ptr<T: ContextClass + 'static>(
        &mut self,
        tag: Tag,
        key: T::Key,
        params: &T::Params,
    ) -> *mut () {
        let type_id = TypeId::of::<T>();
        self.map
            .entry(
                hash(type_id, &key),
                |item| Self::item_compare::<T::Key, T>(type_id, &key, item),
                Self::item_hash::<T::Key, T>,
            )
            .or_insert_with(|| {
                let mut item = Self::new_item::<T>(key, params);

                // Link this item into its tag's chain.
                let prev_tag_chain_head = self.tag_chains.insert(
                    tag,
                    ItemId {
                        hash: item.hash,
                        ptr: item.ptr,
                    },
                );
                item.tag_next = prev_tag_chain_head.unwrap_or(ItemId::null());

                item
            })
            .get()
            .ptr
    }

    fn item_compare<K: Eq, T>(type_id: TypeId, key: &K, item: &Item) -> bool {
        if item.type_id != type_id {
            return false;
        }

        let item_ptr = item.ptr as *const ItemTyped<K, T>;
        let typed = unsafe { &*item_ptr };
        &typed.key == key
    }

    fn item_hash<K: Hash, T: 'static>(item: &Item) -> u64 {
        item.hash
    }

    fn with_inner_fn<K, T, U>(
        &mut self,
        tag: Tag,
        key: K,
        constructor: impl FnOnce() -> T,
        f: impl FnOnce(&mut T) -> U,
    ) -> U
    where
        K: Eq + Hash,
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let key_hash = hash(type_id, &key);

        let item_ptr = self.get_ptr_fn::<K, T>(type_id, tag, key, key_hash, move |key| {
            Self::new_item_fn::<K, T>(key, constructor)
        });
        let item_ref = unsafe { &mut *(item_ptr as *mut ItemTyped<K, T>) };

        f(&mut item_ref.value)
    }

    fn new_item_fn<K: Hash, T: 'static>(key: K, constructor: impl FnOnce() -> T) -> Item {
        unsafe fn drop_fn<K, T>(item_ptr: *mut ()) {
            let _ = unsafe { Box::from_raw(item_ptr as *mut ItemTyped<K, T>) };
        }

        let type_id = TypeId::of::<T>();
        Item {
            type_id,
            hash: hash(type_id, &key),
            ptr: Box::into_raw(Box::new(ItemTyped {
                key,
                value: constructor(),
            })) as *mut (),
            tag_next: ItemId::null(),
            drop_fn: drop_fn::<K, T>,
        }
    }

    fn get_ptr_fn<K: Eq + Hash, T: 'static>(
        &mut self,
        type_id: TypeId,
        tag: Tag,
        key: K,
        key_hash: u64,
        item_constructor: impl FnOnce(K) -> Item,
    ) -> *mut () {
        let item = self
            .map
            .entry(
                key_hash,
                |item| Self::item_compare::<K, T>(type_id, &key, item),
                Self::item_hash::<K, T>,
            )
            .or_insert_with(|| {
                let mut item = item_constructor(key);

                // Link this item into its tag's chain.
                let prev_tag_chain_head = self.tag_chains.insert(
                    tag,
                    ItemId {
                        hash: item.hash,
                        ptr: item.ptr,
                    },
                );
                item.tag_next = prev_tag_chain_head.unwrap_or(ItemId::null());

                item
            })
            .into_mut();
        item.ptr
    }
}

impl<Tag, S: SendTag> Drop for ContextMapPrivate<Tag, S> {
    fn drop(&mut self) {
        for item in self.map.iter() {
            unsafe {
                (item.drop_fn)(item.ptr);
            }
        }
    }
}

fn hash<T: Hash>(type_id: TypeId, t: &T) -> u64 {
    let mut s = siphasher::sip::SipHasher::new();
    type_id.hash(&mut s);
    t.hash(&mut s);
    s.finish()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_context_map() {
        struct Foo {
            x: u32,
        }
        impl ContextClass for Foo {
            type Key = &'static str;
            type Params = u32;
            fn new(params: &Self::Params) -> Self {
                Self { x: *params }
            }
        }

        struct Bar {
            x: u32,
        }
        impl ContextClass for Bar {
            type Key = u16;
            type Params = u32;
            fn new(params: &Self::Params) -> Self {
                Self { x: *params + 10 }
            }
        }

        let tag1 = ModrpcContextTag::Role(0);
        let tag2 = ModrpcContextTag::Role(1);
        let params = 10;
        let mut cx = ContextMap::new();
        cx.with::<Foo, _>(tag1, "foo", &params, |x| x.x += 1);
        cx.with::<Bar, _>(tag2, 2, &params, |x| x.x += 2);
        cx.with::<Foo, _>(tag2, "baz", &params, |x| x.x += 3);
        cx.with::<Foo, _>(tag1, "foo", &params, |x| x.x += 4);
        cx.with::<Bar, _>(tag2, 2, &params, |x| x.x += 5);
        cx.with::<Foo, _>(tag2, "baz", &params, |x| x.x += 6);

        cx.with::<Foo, _>(tag1, "foo", &params, |x| {
            assert_eq!(x.x, 15);
        });
        cx.with::<Bar, _>(tag2, 2, &params, |x| {
            assert_eq!(x.x, 27);
        });
        cx.with::<Foo, _>(tag2, "baz", &params, |x| {
            assert_eq!(x.x, 19);
        });

        cx.shutdown_tag(tag1);
        cx.with::<Foo, _>(tag1, "foo", &params, |x| {
            assert_eq!(x.x, 10);
        });

        cx.shutdown_tag(tag2);
        cx.with::<Foo, _>(tag2, "baz", &params, |x| {
            assert_eq!(x.x, 10);
        });
        cx.with::<Bar, _>(tag2, 2, &params, |x| {
            assert_eq!(x.x, 20);
        });
    }

    #[test]
    fn test_local_context_map() {
        use std::cell::Cell;
        use std::rc::Rc;

        struct Foo {
            x: Rc<Cell<u32>>,
        }
        impl ContextClass for Foo {
            type Key = &'static str;
            type Params = ();
            fn new(_: &Self::Params) -> Self {
                Self {
                    x: Rc::new(Cell::new(10)),
                }
            }
        }

        let tag1 = ModrpcContextTag::Role(0);
        let params = ();
        let mut cx = LocalContextMap::new();
        cx.with::<Foo, _>(tag1, "foo", &params, |x| x.x.set(x.x.get() + 1));
        cx.with::<Foo, _>(tag1, "baz", &params, |x| x.x.set(x.x.get() + 2));
        cx.with::<Foo, _>(tag1, "foo", &params, |x| x.x.set(x.x.get() + 3));
        cx.with::<Foo, _>(tag1, "baz", &params, |x| x.x.set(x.x.get() + 4));

        cx.with::<Foo, _>(tag1, "foo", &params, |x| {
            assert_eq!(x.x.get(), 14);
        });
        cx.with::<Foo, _>(tag1, "baz", &params, |x| {
            assert_eq!(x.x.get(), 16);
        });

        cx.shutdown_tag(tag1);
        cx.with::<Foo, _>(tag1, "foo", &params, |x| {
            assert_eq!(x.x.get(), 10);
        });
    }
}
