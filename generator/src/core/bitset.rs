#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BitSet {
    words: Vec<u64>,
}

impl BitSet {
    pub fn new(capacity: usize) -> Self {
        let len = capacity.div_ceil(64);
        Self {
            words: vec![0; len],
        }
    }

    pub fn add(&mut self, idx: usize) {
        let word_idx = idx / 64;
        let bit = idx % 64;
        if word_idx >= self.words.len() {
            self.words.resize(word_idx + 1, 0);
        }
        self.words[word_idx] |= 1 << bit;
    }

    pub fn contains(&self, idx: usize) -> bool {
        let word_idx = idx / 64;
        if word_idx >= self.words.len() {
            false
        } else {
            (self.words[word_idx] >> (idx % 64)) & 1 == 1
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(wi, &w)| {
            (0..64).filter_map(move |bit| {
                if (w >> bit) & 1 == 1 {
                    Some(wi * 64 + bit)
                } else {
                    None
                }
            })
        })
    }

    pub fn extend<I: IntoIterator<Item = usize>>(&mut self, iter: I) {
        for idx in iter {
            self.add(idx);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }
}

pub fn empty_set(capacity: usize) -> BitSet {
    BitSet::new(capacity)
}

pub fn add_state(set: &mut BitSet, state: usize) {
    set.add(state);
}

pub fn contains(set: &BitSet, state: usize) -> bool {
    set.contains(state)
}

pub fn clone_set(set: &BitSet) -> BitSet {
    set.clone()
}
