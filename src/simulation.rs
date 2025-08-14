use std::path::Path;

use crate::{lru::LruCache, trace::TraceFile};

#[derive(Debug, Copy, Clone)]
pub struct Params {
    pub cycles_hit: u32,
    pub cycles_miss: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Simulation<const CLOCK_SPEED_MHZ: u32> {
    name: String,
    hit_count: u32,
    miss_count: u32,
}

impl<const CLOCK_SPEED_MHZ: u32> Simulation<CLOCK_SPEED_MHZ> {
    pub fn simulate_file<const SETS: usize, const WAYS: usize, const LINE_SIZE: usize>(
        lru_cache: &mut LruCache<SETS, WAYS, LINE_SIZE>,
        file: impl AsRef<Path>,
    ) -> Result<Vec<Self>, String> {
        let current_dir =
            std::env::current_dir().map_err(|e| format!("unable to get current directory: {e}"))?;

        let file_content = std::fs::read_to_string(current_dir.join(file))
            .map_err(|e| format!("failed to read file: {e}"))?;
        Self::simulate(lru_cache, file_content.as_str())
    }

    pub fn simulate<const SETS: usize, const WAYS: usize, const LINE_SIZE: usize>(
        lru_cache: &mut LruCache<SETS, WAYS, LINE_SIZE>,
        file_data: &str,
    ) -> Result<Vec<Self>, String> {
        let trace_file = match TraceFile::try_from(file_data) {
            Ok(trace_file) => trace_file,
            Err(e) => {
                return Err(format!("failed to parse access trace file: {e}"));
            }
        };

        let simulation_results = trace_file
            .into_iter()
            .map(|trace| {
                lru_cache.reset();

                let name = trace.name().to_string();
                trace.into_iter().fold(
                    Simulation {
                        name,
                        hit_count: 0,
                        miss_count: 0,
                    },
                    |mut simulation_result, instruction| {
                        // check all byte addresses
                        // if we just check the start address of the instruction
                        // we would fail to consider the case where an instruction spans multiple cache-blocks
                        // this happens for variable size instruction sets (x86, Arm thumb)
                        let mut hit = true;
                        for i in 0..(instruction.length / 8) {
                            hit &= lru_cache.get(instruction.address + i) == CacheHit::Hit;
                        }

                        if hit {
                            simulation_result.hit_count += 1;
                        } else {
                            simulation_result.miss_count += 1;
                        }

                        simulation_result
                    },
                )
            })
            .collect();

        Ok(simulation_results)
    }

    fn percent_hit(&self) -> f64 {
        100.0 * f64::from(self.hit_count) / (f64::from(self.hit_count) + f64::from(self.miss_count))
    }

    fn percent_miss(&self) -> f64 {
        100.0 * f64::from(self.miss_count)
            / (f64::from(self.hit_count) + f64::from(self.miss_count))
    }

    pub fn format_summary(
        &self,
        Params {
            cycles_hit,
            cycles_miss,
        }: &Params,
    ) -> String {
        let mut result = vec![
            format!("Trace: {}", self.name),
            format!(
                "Number of Instructions: {}",
                self.hit_count + self.miss_count
            ),
            format!("Hits: {}, Misses: {}", self.hit_count, self.miss_count),
            format!("Percent Hits: {:.3}%", self.percent_hit()),
            format!("Percent Misses: {:.3}%", self.percent_miss()),
            format!(
                "Assuming Clock-Speed: {CLOCK_SPEED_MHZ} MHz, Cache-Hit: {cycles_hit} cycles, Cache-Miss: {cycles_miss} cycles"
            ),
        ];

        let cycle_time_us = f64::from(CLOCK_SPEED_MHZ).recip();
        let total_time_us = f64::from(self.hit_count) * f64::from(*cycles_hit) * cycle_time_us
            + f64::from(self.miss_count) * f64::from(*cycles_miss) * cycle_time_us;
        if total_time_us >= 1_000_000.0 {
            result.push(format!("Total time: {:.3}s", total_time_us / 1_000_000.0));
        } else if total_time_us >= 1_000.0 {
            result.push(format!("Total time: {:.3}ms", total_time_us / 1_000.0));
        } else {
            result.push(format!("Total time: {:.3}us", total_time_us));
        }

        result.join("\n")
    }

    pub fn compare(simulation_results: &[Self], config: Params) -> String {
        let cycle_time_hit_us = f64::from(config.cycles_hit) * f64::from(CLOCK_SPEED_MHZ).recip();
        let cycle_time_miss_us = f64::from(config.cycles_miss) * f64::from(CLOCK_SPEED_MHZ).recip();
        let mut results = simulation_results
            .iter()
            .map(|r| {
                (
                    r,
                    f64::from(r.hit_count) * cycle_time_hit_us
                        + f64::from(r.miss_count) * cycle_time_miss_us,
                )
            })
            .collect::<Vec<_>>();

        results.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
        let (_, baseline) = *results.first().unwrap();

        results
            .into_iter()
            .flat_map(|(sim, time)| {
                vec![
                    sim.format_summary(&config),
                    format!(
                        "Relative Time: +{:.3}%\n",
                        (time - baseline) / baseline * 100.0
                    ),
                ]
            })
            .collect::<Vec<_>>()
            .join("\n")
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
            CacheHit::Miss { prev } => match prev {
                Some(prev) => f.write_fmt(format_args!("Miss prev={prev:#X}")),
                None => f.write_str("Miss"),
            },
        }
    }
}
