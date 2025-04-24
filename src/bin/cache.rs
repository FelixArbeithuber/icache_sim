use lru_sim::lru::LruCache;

fn main() {
    let mut lru_cache: LruCache<8, 4, 1> = LruCache::new();

    let Some(filename) = std::env::args().nth(1) else {
        println!("no file given");
        return;
    };

    match lru_cache.simulate(filename) {
        Ok(simulation_result) => {
            simulation_result.print_trace();
            simulation_result.print_summary();
        }
        Err(e) => println!("{e}"),
    };
}
