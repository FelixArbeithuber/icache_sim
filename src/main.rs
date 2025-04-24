use std::env::current_dir;
use std::io::{Write, stdout};

use access_trace::AccessTrace;
use lru::{CacheHit, MainMemory};

mod access_trace;
mod lru;

fn main() {
    let main_memory = MainMemory::<256, 4>::new(std::array::from_fn(|i| i as u8));
    let mut lru_cache = main_memory.create_cache::<8, 4>();

    let mut stdout = stdout();
    let Some(ref filename) = std::env::args().nth(1) else {
        println!("no file given");
        return;
    };

    let Ok(file_data) = std::fs::read_to_string(current_dir().unwrap().join(filename)) else {
        println!("unable to read file");
        return;
    };

    let access_trace = match AccessTrace::try_from(&mut file_data.as_str()) {
        Ok(access_trace) => access_trace,
        Err(e) => {
            println!("failed to parse access trace file: {e}");
            return;
        }
    };

    let mut cache_hits_cnt = 0;
    let mut cache_misses_cnt = 0;
    for address in access_trace {
        let (data, cache_hit) = lru_cache.get(address);
        stdout
            .write_fmt(format_args!(
                "{:?}{} ",
                data,
                if cache_hit == CacheHit::Hit { "" } else { "!" }
            ))
            .unwrap();

        if cache_hit == CacheHit::Hit {
            cache_hits_cnt += 1
        } else {
            cache_misses_cnt += 1
        }
    }

    stdout
        .write_fmt(format_args!(
            "\nHits: {cache_hits_cnt}, Misses: {cache_misses_cnt}\n"
        ))
        .unwrap();
    stdout
        .write_fmt(format_args!(
            "Percent Hits: {}%\n",
            100.0 * f64::from(cache_hits_cnt)
                / (f64::from(cache_hits_cnt) + f64::from(cache_misses_cnt))
        ))
        .unwrap();
    stdout
        .write_fmt(format_args!(
            "Percent Misses: {}%\n",
            100.0 * f64::from(cache_misses_cnt)
                / (f64::from(cache_hits_cnt) + f64::from(cache_misses_cnt))
        ))
        .unwrap();
}
