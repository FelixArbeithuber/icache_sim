#[derive(Debug, Default)]
pub struct SimulationResult {
    pub cache_sets: usize,
    pub cache_ways: usize,
    pub line_size: usize,

    pub data: Vec<(usize, CacheHit)>,
    pub hit_count: u32,
    pub miss_count: u32,
}

impl SimulationResult {
    pub fn new(cache_sets: usize, cache_ways: usize, line_size: usize) -> Self {
        Self {
            cache_sets,
            cache_ways,
            line_size,
            ..Default::default()
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
        println!(
            "\tTotal Size: {}B",
            self.line_size * self.cache_ways * self.cache_sets
        );
        println!("\tSets: {}", self.cache_sets);
        println!("\tWays {}", self.cache_ways);
        println!("\tLine-Size: {}B", self.line_size);
        println!()
    }

    pub fn print_summary(&self) {
        println!("Hits: {}, Misses: {}", self.hit_count, self.miss_count);
        println!("Percent Hits: {:.3} %", self.percent_hit());
        println!("Percent Misses: {:.3} %", self.percent_miss());
        // TODO
        println!()
    }

    pub fn print_trace(&self) {
        use std::io::{Write, stdout};

        let mut stdout = stdout().lock();

        for (address, cache_hit) in self.data.iter() {
            stdout
                .write_fmt(format_args!("{address:#X} {cache_hit}\n"))
                .unwrap();
        }
        stdout.write_all(b"\n").unwrap();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum CacheHit {
    Hit,
    Miss { prev: Option<usize> },
}

impl std::fmt::Display for CacheHit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheHit::Hit => f.write_str("Hit"),
            CacheHit::Miss { prev } => {
                match prev {
                    Some(prev) => f.write_fmt(format_args!("Miss prev={prev:#X}")),
                    None => f.write_str("Miss"),
                }
                // let prev = prev
                //     .map(|addr| format!("{addr:#X}"))
                //     .unwrap_or(String::new());
                // f.write_fmt(format_args!("Miss, prev {prev}"))
            }
        }
    }
}
