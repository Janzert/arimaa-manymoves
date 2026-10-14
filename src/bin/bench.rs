//! Times the counter: best of N counts per board. Usage: [STEPS=k] bench N BOARD...

use std::time::Instant;

use manymoves::{Counter, parse_diagram};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args[1].parse().unwrap();
    let steps: u32 = std::env::var("STEPS").map_or(4, |s| s.parse().unwrap());
    let mut c = Counter::default();
    for f in &args[2..] {
        let b = parse_diagram(&std::fs::read_to_string(f).unwrap());
        let mut best = f64::MAX;
        let mut count = 0;
        for _ in 0..n {
            let t = Instant::now();
            count = c.count_steps(&b, steps);
            best = best.min(t.elapsed().as_secs_f64() * 1000.0);
        }
        println!("{f} {count} best {best:.1}ms");
    }
}
