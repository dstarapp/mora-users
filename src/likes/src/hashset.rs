use candid::{CandidType, Deserialize};
use serde::Serialize;
use std::collections::HashMap;
use std::hash::Hash;

#[derive(Clone, CandidType, Serialize, Deserialize, Default)]
pub struct Field<K, V>
where
    K: Hash + Eq,
{
    pub field: HashMap<K, V>,
}

impl<K, V> Field<K, V>
where
    K: Hash + Eq,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            field: HashMap::default(),
        }
    }
}

#[derive(Clone, CandidType, Serialize, Deserialize, Default)]
pub struct HashSet<K, V>
where
    K: Hash + Eq,
    V:Clone,
{
    pub hset: HashMap<K, Field<K, V>>,
}

impl<K, V> HashSet<K, V>
where
    K: Hash + Eq,
    V: Clone,
{
    pub fn new() -> Self {
        Self {
            hset: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, field: K, value: V) -> bool {
        match self.hset.get_mut(&key) {
            Some(set) => {
                set.field.insert(field, value);
                true
            }
            None => {
                let mut insert_field: Field<K, V> = Field::new();
                insert_field.field.insert(field, value);
                self.hset.insert(key, insert_field);
                true
            }
        }
    }

    pub fn remove_key(&mut self, key: K) -> bool {
        match self.hset.get_mut(&key) {
            Some(_set) => {
                self.hset.remove(&key);
                true
            }
            None => false,
        }
    }

    pub fn remove_field(&mut self, key: K, field: K) -> bool {
        match self.hset.get_mut(&key) {
            Some(set) => match set.field.get_mut(&field) {
                Some(_v) => {
                    set.field.remove(&field);

                    if self.field_len(&key) == 0 {
                        self.remove_key(key);
                    }
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    pub fn get_key(&self, key: &K) -> Option<&Field<K, V>> {
        match self.hset.get(&key) {
            Some(set) => Some(set),
            None => None,
        }
    }

    pub fn get_field(&self, key: &K, field: &K) -> Option<V> {
        match self.hset.get(&key) {
            Some(set) => match set.field.get(&field) {
                Some(v) => Some(v.clone()),
                None => None,
            },
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.hset.len()
    }

    pub fn field_len(&self, key: &K) -> usize {
        match self.hset.get(&key) {
            Some(v) => v.field.len(),
            None => 0,
        }
    }
}
