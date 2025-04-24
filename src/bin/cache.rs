use lru_sim::{lru::LruCache, simulatiton::Simulation};

fn main() {
    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let filename = std::env::args().skip(1).collect::<Vec<_>>();

    lru_cache.print_info();
    match Simulation::<1_600, 1, 100>::run(&mut lru_cache, &filename) {
        Ok(simulation_results) => {
            Simulation::compare(&simulation_results);
        }
        Err(e) => println!("{e}"),
    };
}
