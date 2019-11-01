use std::collections::HashMap;

type K = (u8, [u8; 32]);
type V = [u8; 32];

#[derive(Clone, Default)]
pub struct Buffer {
    map: HashMap<K, V>,
}

impl Buffer {
    pub fn get(&self, frame: u8, key: [u8; 32]) -> Option<&V> {
        self.map.get(&(frame, key))
    }

    pub fn insert(&mut self, frame: u8, key: [u8; 32], value: V) -> Option<V> {
        self.map.insert((frame, key), value)
    }
}
