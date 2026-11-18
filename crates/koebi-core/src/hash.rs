//! Hashing for small keys made by the program itself: ids, indices and addresses.
//!
//! The standard library's hasher resists keys chosen by an attacker, at a cost that dominates
//! lookups by id: a document's evaluation looks thousands of ids up per edit. [`IdHasher`]
//! instead mixes each word with one rotation and one multiplication, and spreads the result
//! over the low bits at the end, where hash tables take their bucket: addresses aligned to 8 or
//! 16 bytes do not crowd into the same buckets.
//!
//! Use it only for keys the program makes ([`IdMap`], [`IdSet`]), never for keys that come
//! from outside (file contents, names typed by the user).

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// A fast hasher for ids, indices and addresses (see the module).
#[derive(Clone, Copy, Debug, Default)]
pub struct IdHasher {
    hash: u64,
}

/// An odd constant whose bits are spread evenly: the multiplication carries every bit of a
/// word to the high bits.
const SEED: u64 = 0xf135_7aea_2e62_a9c5;

impl IdHasher {
    fn add(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for IdHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.add(u64::from_le_bytes(*word));
        }
        if !rest.is_empty() {
            let mut last = [0_u8; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(last));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(u64::from(value));
    }

    fn write_u16(&mut self, value: u16) {
        self.add(u64::from(value));
    }

    fn write_u32(&mut self, value: u32) {
        self.add(u64::from(value));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        // The high bits are the well mixed ones: rotated down to where tables take buckets.
        self.hash.rotate_left(26)
    }
}

/// Builds [`IdHasher`]s.
pub type IdBuildHasher = BuildHasherDefault<IdHasher>;

/// A hash map keyed by ids, indices or addresses.
pub type IdMap<K, V> = HashMap<K, V, IdBuildHasher>;

/// A hash set of ids, indices or addresses.
pub type IdSet<K> = HashSet<K, IdBuildHasher>;

#[cfg(test)]
mod tests {
    use std::hash::{BuildHasher, Hash};

    use super::*;

    fn hash_of(value: impl Hash) -> u64 {
        IdBuildHasher::default().hash_one(value)
    }

    #[test]
    fn aligned_addresses_spread_over_the_low_bits() {
        // Addresses 16 bytes apart, as allocations are: the 64 buckets of a table each get some.
        let mut buckets = [0_u32; 64];
        for i in 0..4096_usize {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "the low six bits of a hash, a bucket of 64"
            )]
            let bucket = (hash_of(0x7f00_0000_usize + 16 * i) & 63) as usize;
            buckets[bucket] += 1;
        }
        let (least, most) = (
            buckets.iter().min().copied().unwrap_or(0),
            buckets.iter().max().copied().unwrap_or(0),
        );
        assert!(least >= 32 && most <= 96, "{least}..{most}");
    }

    #[test]
    fn maps_and_sets_find_what_they_hold() {
        let mut map: IdMap<(u64, usize), &str> = IdMap::default();
        let mut set: IdSet<u64> = IdSet::default();
        for i in 0..1000_u64 {
            map.insert((i, 3), "a");
            set.insert(i * 7);
        }
        assert_eq!(map.get(&(500, 3)), Some(&"a"));
        assert_eq!(map.get(&(500, 4)), None);
        assert!(set.contains(&700));
        assert!(!set.contains(&701));
        // Bytes hash by words, the last one padded: different lengths differ.
        assert_ne!(hash_of([1_u8, 2, 3]), hash_of([1_u8, 2, 3, 0]));
        assert_ne!(hash_of(1_u64), hash_of(2_u64));
    }
}
