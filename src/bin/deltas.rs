//! How much random search mutations change the count of a board, for
//! choosing annealing temperatures. Usage: deltas legal|gold BOARD [N]

use manymoves::search::{Rng, Variant, mutate, valid};
use manymoves::{Counter, parse_diagram};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let variant = if args[1] == "legal" { Variant::Legal } else { Variant::GoldOnly };
    let b = parse_diagram(&std::fs::read_to_string(&args[2]).unwrap());
    let n: usize = args.get(3).map_or(400, |s| s.parse().unwrap());
    let mut rng = Rng::new(5);
    let mut c = Counter::default();
    let base = c.count(&b) as i64;
    let mut d = Vec::new();
    while d.len() < n {
        let q = mutate(&b, &mut rng, variant);
        if q != b && valid(&q, variant) {
            d.push(c.count(&q) as i64 - base);
        }
    }
    let mut u = d.clone();
    u.sort();
    u.dedup();
    println!("{} distinct deltas", u.len());
    d.sort();
    let q = |f: f64| d[((d.len() - 1) as f64 * f) as usize];
    println!("base {base}: min {} 10% {} 25% {} 50% {} 75% {} 90% {} max {}", q(0.0), q(0.1), q(0.25), q(0.5), q(0.75), q(0.9), q(1.0));
}
