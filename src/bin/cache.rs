use lru_sim::lru::LruCache;

fn main() {
    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let Some(filename) = std::env::args().nth(1) else {
        println!("no file given");
        return;
    };

    match lru_cache.simulate(filename) {
        Ok(simulation_result) => {
            simulation_result.print_cache_info();
            simulation_result.print_summary();
            simulation_result.print_trace();
        }
        Err(e) => println!("{e}"),
    };
}
