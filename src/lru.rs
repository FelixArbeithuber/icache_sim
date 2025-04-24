use std::array;

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
        println!(
            "\t| {} tag bits | {} set bits | {} offset bits |",
            std::mem::size_of::<usize>() * 8 - (self.set_index_width + self.offset_width),
            self.set_index_width,
            self.offset_width
        );
        println!();
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

    pub fn reset(&mut self) {
        self.sets = array::from_fn(|_| CacheSet::new());
    }

    pub fn get(&mut self, address: usize) -> CacheHit {
        let set_index = (address >> self.offset_width) & self.set_index_mask;
        let tag = address >> (self.set_index_width + self.offset_width);

        self.sets.get_mut(set_index).unwrap().get(address, tag)
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
struct CacheSet<const WAYS: usize> {
    lines: [CacheLine; WAYS],
    lru: [usize; WAYS],
}

impl<const LINES: usize> CacheSet<LINES> {
    fn new() -> Self {
        Self {
            lines: [CacheLine {
                address: None,
                tag: None,
            }; LINES],
            lru: array::from_fn(|i| i),
        }
    }

    fn get(&mut self, address: usize, tag: usize) -> CacheHit {
        // linear search for cache_line with tag
        let cache_line = self
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| line.tag == Some(tag))
            .map(|(line_idx, _)| line_idx);

        match cache_line {
            // Cache-Hit: set cache-line as the most recently used
            Some(line_idx) => {
                let meta_idx = self
                    .lru
                    .iter()
                    .enumerate()
                    .find(|(_, idx)| **idx == line_idx)
                    .map(|(meta_idx, _)| meta_idx)
                    .unwrap();

                let tmp = *self.lru.get(meta_idx).unwrap();
                for i in (1..=meta_idx).rev() {
                    *self.lru.get_mut(i).unwrap() = *self.lru.get(i - 1).unwrap();
                }
                *self.lru.get_mut(0).unwrap() = tmp;

                CacheHit::Hit
            }
            // Cache-Miss: replace least recently used cache-line and set it as the most recently used
            None => {
                self.lru.rotate_right(1);
                let lru = *self.lru.get(0).unwrap();

                let lru_line = self.lines.get_mut(lru).unwrap();
                let prev = lru_line.address;
                *lru_line = CacheLine {
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
