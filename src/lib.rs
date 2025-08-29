pub mod lru;
pub mod simulation;
mod trace;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn run_simulation(
    trace: &str,
    cycles_hit: u32,
    cycles_miss: u32,
    log_memory_accesses: bool,
) -> String {
    use lru::LruCache;
    use simulation::{Params, Simulation};

    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let mut result = Vec::new();
    result.push(lru_cache.format_info());

    match Simulation::<1_600>::simulate(&mut lru_cache, trace, log_memory_accesses) {
        Ok(simulation_results) => {
            result.push(Simulation::memory_accesses(&simulation_results));
            result.push(Simulation::compare(
                &simulation_results,
                Params {
                    cycles_hit,
                    cycles_miss,
                },
            ));
        }
        Err(e) => return e,
    };

    result.join("\n")
}
