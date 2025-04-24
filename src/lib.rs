pub mod lru;
pub mod simulation;
mod trace;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen]
pub fn run_simulation(trace: &str) -> String {
    use lru::LruCache;
    use simulation::Simulation;

    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let mut result = Vec::new();
    result.push(lru_cache.format_info());

    match Simulation::<1_600, 1, 10>::simulate(&mut lru_cache, trace) {
        Ok(simulation_results) => result.push(Simulation::compare(&simulation_results)),
        Err(e) => return e,
    };

    result.join("\n")
}
