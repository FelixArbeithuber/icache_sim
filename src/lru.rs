use std::{array, collections::VecDeque, path::Path};

use crate::simulatiton_result::{CacheHit, SimulationResult};
use crate::trace::Trace;

/// ## const generics
/// - `SETS`: number of sets in case
/// - `LINES`: number of cache-lines in a set
/// - `LINE_SIZE`: number of bytes in a cache-line
#[derive(Debug)]
pub struct LruCache<const SETS: usize, const LINES: usize, const LINE_SIZE: usize = 1> {
    offset_width: usize,
    set_index_width: usize,
    sets: [CacheSet<LINES>; SETS],
}

impl<const SETS: usize, const LINES: usize, const LINE_SIZE: usize>
    LruCache<SETS, LINES, LINE_SIZE>
{
    pub fn new() -> Self {
        const {
            assert!(
                SETS.count_ones() == 1,
                "SETS of LruCache is not a power of two"
            );
            assert!(
                LINES.count_ones() == 1,
                "LINES of LruCache is not a power of two"
            );
            assert!(
                LINE_SIZE.count_ones() == 1,
                "LINE_SIZE of LruCache is not a power of two"
            );
            assert!(
                (SETS.ilog2() + LINES.ilog2() + LINE_SIZE.ilog2()) as usize
                    <= std::mem::size_of::<usize>() * 8,
                "not enough bits in adress to index all elements in the cache"
            );
        }

        let offset_width = (LINE_SIZE).ilog2() as usize;
        let set_index_width = offset_width + LINES.ilog2() as usize;

        Self {
            offset_width,
            set_index_width,
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

        let mut simulation_result = SimulationResult::new(SETS, LINES, LINE_SIZE);
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
        let set_index = (address >> self.offset_width) & (SETS - 1);
        let tag = address >> self.set_index_width;

        self.sets[set_index].get(tag)
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
            lines: [CacheLine { tag: None }; LINES],
            lru: VecDeque::from_iter(0..LINES),
        }
    }

    fn get(&mut self, tag: usize) -> CacheHit {
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

                let prev = self.lines[lru].tag;
                self.lines[lru] = CacheLine { tag: Some(tag) };

                CacheHit::Miss { prev }
            }
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct CacheLine {
    tag: Option<usize>,
}
