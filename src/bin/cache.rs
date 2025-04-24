use lru_sim::{lru::LruCache, simulation::Simulation};

fn main() {
    // https://developer.arm.com/documentation/102199/0001/Memory-System/Level-1-caches?lang=en
    let mut lru_cache: LruCache<128, 4, 64> = LruCache::new();

    let Some(filename) = std::env::args().nth(1) else {
        println!("no argument for filename given");
        return;
    };

    let current_dir = std::env::current_dir()
        .map_err(|e| format!("unable to get current directory: {e}"))
        .unwrap();
    let file_content = std::fs::read_to_string(current_dir.join(filename))
        .map_err(|e| format!("failed to read file: {e}"))
        .unwrap();

    println!("{}", lru_cache.format_info());
    match Simulation::<1_600, 1, 10>::simulate(&mut lru_cache, &file_content) {
        Ok(simulation_results) => println!("{}", Simulation::compare(&simulation_results)),
        Err(e) => println!("{e}"),
    };
}
