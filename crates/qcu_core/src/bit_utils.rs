pub struct BitPack;

impl BitPack {
    #[inline(always)]
    pub fn get(storage: &[u64], index: usize) -> bool {
        let word = storage[index / 64];
        let bit = index % 64;
        (word >> bit) & 1 == 1
    }

    #[inline(always)]
    pub fn toggle(storage: &mut [u64], index: usize) {
        let word_idx = index / 64;
        let bit_idx = index % 64;
        storage[word_idx] ^= 1 << bit_idx;
    }

    #[inline(always)]
    pub fn set(storage: &mut [u64], index: usize, val: bool) {
        if val {
            let word_idx = index / 64;
            let bit_idx = index % 64;
            storage[word_idx] |= 1 << bit_idx;
        } else {
            let word_idx = index / 64;
            let bit_idx = index % 64;
            storage[word_idx] &= !(1 << bit_idx);
        }
    }
}
