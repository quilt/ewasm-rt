use std::collections::HashMap;

type K = u8;
type V = HashMap<[u8; 32], [u8; 32]>;

#[derive(Debug, Clone, Default)]
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
        let map = self.map.entry(frame).or_insert(HashMap::new());
        map.insert(key, value)
    }

    pub fn merge(&mut self, a: u8, b: u8) {
        let b = self.map.entry(b).or_insert(HashMap::new()).to_owned();
        let a = self.map.entry(a).or_insert(HashMap::new());

        for (key, value) in b.iter() {
            a.insert(*key, *value);
        }
    }

    pub fn clear(&mut self, frame: u8) {
        self.map.remove(&frame);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn merge() {
        let mut buffer = Buffer::default();

        buffer.insert(0, [0u8; 32], [0u8; 32]);
        buffer.insert(0, [1u8; 32], [1u8; 32]);
        buffer.insert(1, [2u8; 32], [2u8; 32]);
        buffer.insert(1, [0u8; 32], [3u8; 32]);

        buffer.merge(0, 1);

        assert_eq!(buffer.get(0, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(buffer.get(0, [1u8; 32]), Some(&[1u8; 32]));
        assert_eq!(buffer.get(0, [2u8; 32]), Some(&[2u8; 32]));
        assert_eq!(buffer.get(1, [0u8; 32]), Some(&[3u8; 32]));
        assert_eq!(buffer.get(1, [2u8; 32]), Some(&[2u8; 32]));
    }
}
