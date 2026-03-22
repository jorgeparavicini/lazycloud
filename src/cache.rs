use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::Hash;

// TODO: Improve ergonomics of this API
pub struct Cache {
    stores: HashMap<TypeId, Box<dyn Any>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            stores: HashMap::new(),
        }
    }

    pub fn get<K, T>(&self, key: &K) -> Option<&T>
    where
        K: Hash + Eq + 'static,
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        self.stores
            .get(&type_id)?
            .downcast_ref::<HashMap<K, T>>()?
            .get(key)
    }

    pub fn insert<K, T>(&mut self, key: K, value: T)
    where
        K: Hash + Eq + 'static,
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let store = self
            .stores
            .entry(type_id)
            .or_insert_with(|| Box::new(HashMap::<K, T>::new()));
        let store = store
            .downcast_mut::<HashMap<K, T>>()
            .expect("Type mismatch in cache");
        store.insert(key, value);
    }

    pub fn clear<T>(&mut self)
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        self.stores.remove(&type_id);
    }

    pub fn invalidate<K, T>(&mut self, key: &K)
    where
        K: Hash + Eq + 'static,
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        if let Some(store) = self.stores.get_mut(&type_id) {
            if let Some(store) = store.downcast_mut::<HashMap<K, T>>() {
                store.remove(key);
            }
        }
    }
}
