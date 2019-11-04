use std::collections::HashMap;

type K = u8;
type V = HashMap<[u8; 32], [u8; 32]>;

#[derive(Clone, Default)]
pub struct Buffer {
    map: HashMap<K, V>,
}

impl Buffer {
    pub fn get(&self, frame: u8, key: [u8; 32]) -> Option<&[u8; 32]> {
        match self.map.get(&frame) {
            Some(map) => map.get(&key),
            None => None,
        }
    }

    pub fn insert(&mut self, frame: u8, key: [u8; 32], value: [u8; 32]) -> Option<[u8; 32]> {
        let map = match self.map.get_mut(&frame) {
            Some(map) => map,
            None => {
                self.map.insert(frame, HashMap::new());
                self.map.get_mut(&frame).unwrap()
            }
        };

        map.insert(key, value)
    }
}
