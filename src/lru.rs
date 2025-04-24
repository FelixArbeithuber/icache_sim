use std::{array, collections::VecDeque};

use crate::simulatiton::CacheHit;

/// ## const generics
/// - `SETS`: number of sets in case
/// - `WAYS`: number of cache-lines in a set
/// - `LINE_SIZE`: number of bytes in a cache-line
#[derive(Debug)]
pub struct LruCache<const SETS: usize, const WAYS: usize, const LINE_SIZE: usize = 1> {
    offset_width: usize,
    set_index_width: usize,
    set_index_mask: usize,
    sets: [CacheSet<WAYS>; SETS],
}

impl<const SETS: usize, const WAYS: usize, const LINE_SIZE: usize> LruCache<SETS, WAYS, LINE_SIZE> {
    pub fn print_info(&self) {
        println!("LRU Cache:");
        println!("\tTotal Size: {}B", LINE_SIZE * WAYS * SETS);
        println!("\tSets: {}", SETS);
        println!("\tWays {}", WAYS);
        println!("\tLine-Size: {}B", LINE_SIZE);
        println!()
    }

    pub fn new() -> Self {
        // for e.g. 64 different sets we need to index 0..=63
        // the number of bits required to represent that number is log2(64 - 1) + 1
        const fn required_bits(i: usize) -> usize {
            (i - 1).ilog2() as usize + 1
        }

        const {
            assert!(
                required_bits(SETS) + required_bits(LINE_SIZE) <= std::mem::size_of::<usize>() * 8,
                "not enough bits in adress to index all elements in the cache"
            );
        }

        let offset_width = required_bits(LINE_SIZE);
        let set_index_width = required_bits(SETS);
        let set_index_mask = !(!0usize << set_index_width);

        Self {
            offset_width,
            set_index_width,
            set_index_mask,
            sets: array::from_fn(|_| CacheSet::new()),
        }
    }

    pub fn get(&mut self, address: usize) -> CacheHit {
        let set_index = (address >> self.offset_width) & self.set_index_mask;
        let tag = address >> (self.set_index_width + self.offset_width);

        self.sets[set_index].get(address, tag)
    }
}

impl<const SETS: usize, const WAYS: usize, const LINE_SIZE: usize> Default
    for LruCache<SETS, WAYS, LINE_SIZE>
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
struct CacheSet<const LINES: usize> {
    lines: [CacheLine; LINES],
    lru: VecDeque<usize>,
}

impl<const LINES: usize> CacheSet<LINES> {
    fn new() -> Self {
        Self {
            lines: [CacheLine {
                address: None,
                tag: None,
            }; LINES],
            lru: VecDeque::from_iter(0..LINES),
        }
    }

    fn get(&mut self, address: usize, tag: usize) -> CacheHit {
        // linear search for cache_line with tag
        let cache_line = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| line.tag == Some(tag));

        match cache_line {
            // Cache-Hit: set cache-line as the most recently used
            Some((line_idx, _)) => {
                let (meta_idx, _) = self
                    .lru
                    .iter()
                    .enumerate()
                    .find(|(_, idx)| **idx == line_idx)
                    .unwrap();

                self.lru.remove(meta_idx);
                self.lru.push_back(line_idx);

                CacheHit::Hit
            }
            // Cache-Miss: replace least recently used cache-line and set it as the most recently used
            None => {
                let lru = self.lru.pop_front().unwrap();
                self.lru.push_back(lru);

                let prev = self.lines[lru].address;
                self.lines[lru] = CacheLine {
                    address: Some(address),
                    tag: Some(tag),
                };

                CacheHit::Miss { prev }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CacheLine {
    address: Option<usize>,
    tag: Option<usize>,
}
