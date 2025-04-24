#[derive(Debug, Default)]
pub struct SimulationResult {
    pub cache_sets: usize,
    pub cache_lines: usize,
    pub line_size: usize,

    pub data: Vec<(usize, CacheHit)>,
    pub hit_count: u32,
    pub miss_count: u32,
}

impl SimulationResult {
    pub fn new(cache_sets: usize, cache_lines: usize, line_size: usize) -> Self {
        Self {
            cache_sets,
            cache_lines,
            line_size,

            data: Vec::new(),
            hit_count: 0,
            miss_count: 0,
        }
    }

    pub fn percent_hit(&self) -> f64 {
        100.0 * f64::from(self.hit_count) / (f64::from(self.hit_count) + f64::from(self.miss_count))
    }

    pub fn percent_miss(&self) -> f64 {
        100.0 * f64::from(self.miss_count)
            / (f64::from(self.hit_count) + f64::from(self.miss_count))
    }

    pub fn print_cache_info(&self) {
        println!("LRU Cache:");
        println!("\tSets: {}", self.cache_sets);
        println!("\tLines {}", self.cache_lines);
        println!("\tLine-Size: {}", self.line_size);
        println!(
            "\tTotal Size: {}B",
            self.line_size * self.cache_lines * self.cache_sets
        );
        println!()
    }

    pub fn print_summary(&self) {
        println!("Hits: {}, Misses: {}", self.hit_count, self.miss_count);
        println!("Percent Hits: {:.3} %", self.percent_hit());
        println!("Percent Misses: {:.3} %", self.percent_miss());
        println!()
    }

    pub fn print_trace(&self) {
        use std::io::{Write, stdout};

        let mut stdout = stdout().lock();

        for (address, cache_hit) in self.data.iter() {
            stdout
                .write_fmt(format_args!("{address:#X} ({cache_hit})\n"))
                .unwrap();
        }
        stdout.write(b"\n").unwrap();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CacheHit {
    Hit,
    Miss,
}

impl std::fmt::Display for CacheHit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheHit::Hit => f.write_str("Hit"),
            CacheHit::Miss => f.write_str("Miss"),
        }
    }
}
