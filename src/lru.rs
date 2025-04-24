use std::{array, collections::VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheHit {
    Hit,
    Miss,
}

pub struct MainMemory<const SIZE: usize, const LINE_SIZE: usize> {
    data: [u8; SIZE],
}

impl<const SIZE: usize, const LINE_SIZE: usize> MainMemory<SIZE, LINE_SIZE> {
    pub fn new(data: [u8; SIZE]) -> Self {
        const {
            assert!(
                is_power_of_two(SIZE),
                "SIZE of MainMemory is not a power of two"
            );
            assert!(
                is_power_of_two(LINE_SIZE),
                "LINE_SIZE of MainMemory is not a power of two"
            );
        }

        Self { data }
    }

    pub fn create_cache<const SETS: usize, const LINES: usize>(
        &self,
    ) -> LruCache<SIZE, SETS, LINES, LINE_SIZE> {
        const {
            assert!(
                is_power_of_two(SETS),
                "SIZE of MainMemory is not a power of two"
            );
            assert!(
                is_power_of_two(LINES),
                "SIZE of MainMemory is not a power of two"
            );
            assert!(
                LINE_SIZE * LINES * SETS <= SIZE,
                "the LruCache can't be bigger than the MainMemory"
            )
        }

        LruCache::new(self)
    }

    fn get(&self, address: usize) -> [u8; LINE_SIZE] {
        let addr = address / LINE_SIZE * LINE_SIZE;
        array::from_fn(|i| self.data.get(addr + i).copied().unwrap_or(0))
    }
}

pub struct LruCache<
    'mm,
    const SIZE: usize,
    const SETS: usize,
    const LINES: usize,
    const LINE_SIZE: usize,
> {
    main_memory: &'mm MainMemory<SIZE, LINE_SIZE>,
    sets: [CacheSet<LINES, LINE_SIZE>; SETS],
}

impl<'mm, const SIZE: usize, const SETS: usize, const LINES: usize, const LINE_SIZE: usize>
    LruCache<'mm, SIZE, SETS, LINES, LINE_SIZE>
{
    fn new(main_memory: &'mm MainMemory<SIZE, LINE_SIZE>) -> Self {
        Self {
            main_memory,
            sets: array::from_fn(|_| CacheSet::new()),
        }
    }

    pub fn get(&mut self, address: usize) -> ([u8; LINE_SIZE], CacheHit) {
        let set_addr = address / LINE_SIZE * LINE_SIZE;
        let set = &mut self.sets[address / LINE_SIZE % LINES];

        let cache_line = set
            .lines
            .iter()
            .enumerate()
            .find(|(_, line)| line.as_ref().is_some_and(|l| l.set_address == set_addr));

        if let Some((idx, Some(cache_line))) = cache_line {
            set.meta.remove(
                set.meta
                    .iter()
                    .enumerate()
                    .find_map(|(i1, &i2)| if i2 == idx { Some(i1) } else { None })
                    .unwrap(),
            );
            set.meta.push_front(idx);
            return (cache_line.line, CacheHit::Hit);
        }

        let line = self.main_memory.get(address);

        let lru = set.meta.pop_back().unwrap();
        set.meta.push_front(lru);
        set.lines[lru] = Some(CacheLine::new(set_addr, line));

        (line, CacheHit::Miss)
    }
}

#[derive(Debug, Clone)]
struct CacheSet<const LINES: usize, const LINE_SIZE: usize> {
    lines: [Option<CacheLine<LINE_SIZE>>; LINES],
    meta: VecDeque<usize>,
}

impl<const LINES: usize, const LINE_SIZE: usize> CacheSet<LINES, LINE_SIZE> {
    fn new() -> Self {
        let mut meta = VecDeque::with_capacity(LINES);
        for i in (0..LINES).rev() {
            meta.push_back(i);
        }

        Self {
            lines: array::from_fn(|_| None),
            meta,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheLine<const LINE_SIZE: usize> {
    set_address: usize,
    line: [u8; LINE_SIZE],
}

impl<const LINE_SIZE: usize> CacheLine<LINE_SIZE> {
    fn new(set_address: usize, line: [u8; LINE_SIZE]) -> Self {
        Self { set_address, line }
    }
}

const fn is_power_of_two(i: usize) -> bool {
    i > 1 && i.count_ones() == 1
}
