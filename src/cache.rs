use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;

/// A cache key that determines the type of value it maps to.
///
/// Tying the value type to the key type keeps lookups, insertions and
/// invalidations for one entry in agreement: a caller cannot invalidate a
/// different store than the one the value was inserted into.
pub trait CacheKey: Hash + Eq + 'static {
    /// The value this key maps to.
    type Value: 'static;
}

/// A heterogeneous key-value store, holding one map per [`CacheKey`] type.
pub struct Cache {
    stores: HashMap<TypeId, Box<dyn Any>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn get<K: CacheKey>(&self, key: &K) -> Option<&K::Value> {
        self.store::<K>()?.get(key)
    }

    pub fn insert<K: CacheKey>(&mut self, key: K, value: K::Value) {
        self.store_mut::<K>().insert(key, value);
    }

    pub fn invalidate<K: CacheKey>(&mut self, key: &K) {
        if let Some(store) = self.store_mut_if_present::<K>() {
            store.remove(key);
        }
    }

    /// Drop every entry whose key satisfies `predicate`.
    ///
    /// Useful when one change invalidates a family of entries, e.g. every
    /// cached payload of a single secret.
    pub fn invalidate_where<K: CacheKey>(&mut self, predicate: impl Fn(&K) -> bool) {
        if let Some(store) = self.store_mut_if_present::<K>() {
            store.retain(|key, _| !predicate(key));
        }
    }

    #[allow(dead_code)]
    pub fn clear<K: CacheKey>(&mut self) {
        self.stores.remove(&TypeId::of::<K>());
    }

    // A store is only ever created as `HashMap<K, K::Value>` for the key type it
    // is filed under, so the downcasts below cannot fail.

    fn store<K: CacheKey>(&self) -> Option<&HashMap<K, K::Value>> {
        self.stores.get(&TypeId::of::<K>()).map(|store| {
            store
                .downcast_ref()
                .expect("cache store holds the map type its key type declares")
        })
    }

    fn store_mut<K: CacheKey>(&mut self) -> &mut HashMap<K, K::Value> {
        self.stores
            .entry(TypeId::of::<K>())
            .or_insert_with(|| Box::new(HashMap::<K, K::Value>::new()))
            .downcast_mut()
            .expect("cache store holds the map type its key type declares")
    }

    fn store_mut_if_present<K: CacheKey>(&mut self) -> Option<&mut HashMap<K, K::Value>> {
        self.stores.get_mut(&TypeId::of::<K>()).map(|store| {
            store
                .downcast_mut()
                .expect("cache store holds the map type its key type declares")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Hash, PartialEq, Eq)]
    struct Names;

    impl CacheKey for Names {
        type Value = Vec<String>;
    }

    #[derive(Hash, PartialEq, Eq)]
    struct Payload {
        secret: String,
        version: Option<String>,
    }

    impl CacheKey for Payload {
        type Value = String;
    }

    // Same value type as `Names`, to catch key types sharing a store.
    #[derive(Hash, PartialEq, Eq)]
    struct Labels;

    impl CacheKey for Labels {
        type Value = Vec<String>;
    }

    fn payload(secret: &str, version: Option<&str>) -> Payload {
        Payload {
            secret: secret.to_string(),
            version: version.map(str::to_string),
        }
    }

    #[test]
    fn returns_inserted_value() {
        let mut cache = Cache::new();
        cache.insert(Names, vec!["a".to_string()]);

        assert_eq!(cache.get(&Names), Some(&vec!["a".to_string()]));
    }

    #[test]
    fn misses_before_insertion() {
        let cache = Cache::new();

        assert_eq!(cache.get(&Names), None);
    }

    #[test]
    fn invalidate_drops_only_the_given_key() {
        let mut cache = Cache::new();
        cache.insert(payload("db", None), "old".to_string());
        cache.insert(payload("db", Some("1")), "v1".to_string());

        cache.invalidate(&payload("db", None));

        assert_eq!(cache.get(&payload("db", None)), None);
        assert_eq!(
            cache.get(&payload("db", Some("1"))),
            Some(&"v1".to_string())
        );
    }

    #[test]
    fn invalidate_where_drops_every_matching_key() {
        let mut cache = Cache::new();
        cache.insert(payload("db", None), "latest".to_string());
        cache.insert(payload("db", Some("1")), "v1".to_string());
        cache.insert(payload("api", None), "latest".to_string());

        cache.invalidate_where::<Payload>(|key| key.secret == "db");

        assert_eq!(cache.get(&payload("db", None)), None);
        assert_eq!(cache.get(&payload("db", Some("1"))), None);
        assert_eq!(
            cache.get(&payload("api", None)),
            Some(&"latest".to_string())
        );
    }

    #[test]
    fn key_types_sharing_a_value_type_get_separate_stores() {
        let mut cache = Cache::new();
        cache.insert(Names, vec!["name".to_string()]);
        cache.insert(Labels, vec!["label".to_string()]);

        assert_eq!(cache.get(&Names), Some(&vec!["name".to_string()]));
        assert_eq!(cache.get(&Labels), Some(&vec!["label".to_string()]));

        cache.invalidate(&Names);

        assert_eq!(cache.get(&Names), None);
        assert_eq!(cache.get(&Labels), Some(&vec!["label".to_string()]));
    }

    #[test]
    fn clear_drops_the_whole_store() {
        let mut cache = Cache::new();
        cache.insert(payload("db", None), "latest".to_string());
        cache.insert(payload("api", None), "latest".to_string());

        cache.clear::<Payload>();

        assert_eq!(cache.get(&payload("db", None)), None);
        assert_eq!(cache.get(&payload("api", None)), None);
    }
}
