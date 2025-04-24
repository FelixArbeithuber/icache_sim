use std::{array, collections::VecDeque, path::Path};

use crate::simulatiton_result::{CacheHit, SimulationResult};
use crate::trace::Trace;

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

        // println!("offset_width={offset_width}, set_index_width={set_index_width}");
        // println!("set_index_mask={set_index_mask:#b}");

        Self {
            offset_width,
            set_index_width,
            set_index_mask,
            sets: array::from_fn(|_| CacheSet::new()),
        }
    }

    pub fn simulate(&mut self, file: impl AsRef<Path>) -> Result<SimulationResult, String> {
        let Ok(file_data) = std::fs::read_to_string(
            std::env::current_dir()
                .map_err(|_| "unable to get current directory")?
                .join(file),
        ) else {
            return Err("unable to read file".into());
        };

        let access_trace = match Trace::try_from(&mut file_data.as_str()) {
            Ok(access_trace) => access_trace,
            Err(e) => {
                return Err(format!("failed to parse access trace file: {e}"));
            }
        };

        let mut simulation_result = SimulationResult::new(SETS, WAYS, LINE_SIZE);
        for address in access_trace.into_iter() {
            let cache_hit = self.get(address);
            simulation_result.data.push((address, cache_hit));
            match cache_hit {
                CacheHit::Hit => simulation_result.hit_count += 1,
                CacheHit::Miss { .. } => simulation_result.miss_count += 1,
            }
        }

        Ok(simulation_result)
    }

    pub fn get(&mut self, address: usize) -> CacheHit {
        let set_index = (address >> self.offset_width) & self.set_index_mask;
        let tag = address >> (self.set_index_width + self.offset_width);
        // println!("{address:#13b}, {set_index:#13b}, {tag:#13b}");

        self.sets[set_index].get(address, tag)
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
