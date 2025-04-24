use lru_sim::{lru::LruCache, simulatiton::Simulation};

fn main() {
    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let Some(filename) = std::env::args().nth(1) else {
        println!("no argument for filename given");
        return;
    };

    lru_cache.print_info();
    match Simulation::<1_600, 1, 100>::run(&mut lru_cache, &filename) {
        Ok(simulation_results) => Simulation::compare(&simulation_results),
        Err(e) => println!("{e}"),
    };
}
