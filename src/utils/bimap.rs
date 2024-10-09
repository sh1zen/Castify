use std::collections::hash_map::Iter;
use std::collections::HashMap;

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

pub struct BiMap<K1, K2> {
    forward_map: HashMap<K1, K2>,
    backward_map: HashMap<K2, K1>,
}

impl<K1, K2> BiMap<K1, K2>
where
    K1: std::hash::Hash + Eq + Clone + Copy,
    K2: std::hash::Hash + Eq + Clone + Copy,
{
    pub fn new() -> Self {
        BiMap {
            forward_map: HashMap::new(),
            backward_map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key1: K1, key2: K2) {
        self.forward_map.insert(key1, key2);
        self.backward_map.insert(key2, key1);
    }

    pub fn contains(&self, t: impl Into<Either<K1, K2>>) -> bool {
        let key: Either<K1, K2> = t.into();

        match key {
            Either::Left(key1) => self.forward_map.contains_key(&key1),
            Either::Right(key2) => self.backward_map.contains_key(&key2),
        }
    }

    // Use Either to return K1 or K2 based on input
    pub fn get(&self, t: impl Into<Either<K1, K2>>) -> Option<Either<&K1, &K2>> {
        let key: Either<K1, K2> = t.into();

        match key {
            Either::Left(key1) => self.forward_map.get(&key1).map(Either::Right),
            Either::Right(key2) => self.backward_map.get(&key2).map(Either::Left),
        }
    }

    pub fn get_mut(&mut self, t: impl Into<Either<K1, K2>>) -> Option<Either<&mut K1, &mut K2>> {
        let key: Either<K1, K2> = t.into();

        match key {
            Either::Left(key1) => self.forward_map.get_mut(&key1).map(Either::Right),
            Either::Right(key2) => self.backward_map.get_mut(&key2).map(Either::Left),
        }
    }

    pub fn remove(&mut self, t: impl Into<Either<K1, K2>>) -> Option<K1> {
        let key: Either<K1, K2> = t.into();

        match key {
            Either::Left(key1) => {
                if let Some(value) = self.forward_map.remove(&key1) {
                    self.backward_map.remove(&value);
                    Some(key1)
                } else {
                    None
                }
            }
            Either::Right(key2) => {
                if let Some(value) = self.backward_map.remove(&key2) {
                    self.forward_map.remove(&value);
                    Some(value)
                } else {
                    None
                }
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, K1, K2> {
        self.forward_map.iter()
    }

    pub fn iter_inv(&self) -> Iter<'_, K2, K1> {
        self.backward_map.iter()
    }
}
