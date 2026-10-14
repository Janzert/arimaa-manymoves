//! Runs simulated annealing on many threads and saves each new best board.
//! Usage: search [--proxy STEPS] legal|gold [threads] [iters per run] [t_start] [t_end] [start board file]
//! Boards go in results/<variant>/<count>.txt.
//!
//! With `--proxy 3` (or 2) it anneals on the cheaper 3-step count, then
//! counts the run's best few boards exactly and polishes the best of them.

use std::sync::Mutex;
use std::time::Instant;

use manymoves::search::{Anneal, Rng, Variant, anneal, polish, random_board};
use manymoves::{Counter, parse_diagram};

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    let mut steps = 4;
    if let Some(i) = args.iter().position(|a| a == "--proxy") {
        steps = args[i + 1].parse().unwrap();
        args.drain(i..i + 2);
    }
    let variant = match args.get(1).map(String::as_str) {
        Some("legal") => Variant::Legal,
        Some("gold") => Variant::GoldOnly,
        _ => panic!("usage: search legal|gold [threads] [iters] [t_start] [t_end] [start]"),
    };
    let threads: usize = args.get(2).map_or(8, |s| s.parse().unwrap());
    let iters: u64 = args.get(3).map_or(20000, |s| s.parse().unwrap());
    let t_start: f64 = args.get(4).map_or(3000.0, |s| s.parse().unwrap());
    let t_end: f64 = args.get(5).map_or(30.0, |s| s.parse().unwrap());
    let start = args.get(6).map(|f| parse_diagram(&std::fs::read_to_string(f).unwrap()));
    let dir = format!("results/{}", if variant == Variant::Legal { "legal" } else { "gold" });
    std::fs::create_dir_all(&dir).unwrap();
    let best = Mutex::new(0usize);
    let t0 = Instant::now();
    let params = Anneal { variant, iters, t_start, t_end, steps, keep: if steps < 4 { 50 } else { 1 } };
    std::thread::scope(|s| {
        for th in 0..threads {
            let (best, params, dir) = (&best, &params, &dir);
            s.spawn(move || {
                let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64;
                let mut rng = Rng::new(seed ^ (th as u64).wrapping_mul(0x9E37_79B9));
                let mut counter = Counter::default();
                for run in 0.. {
                    let b0 = start.unwrap_or_else(|| random_board(&mut rng, variant));
                    let report = |b: &manymoves::Board, n: usize| {
                        let mut g = best.lock().unwrap();
                        if n > *g {
                            *g = n;
                            let text = format!("2g\n{}\n# {n} unique moves\n", b.diagram());
                            std::fs::write(format!("{dir}/{n}.txt"), &text).unwrap();
                            println!("[{:.0}s] thread {th} run {run}: {n}\n{}", t0.elapsed().as_secs_f64(), b.diagram());
                        }
                    };
                    let (b1, n1) = if steps < 4 {
                        let (_, proxy, top) = anneal(b0, params, &mut rng, &mut counter, |_, _| {});
                        let exact = top.iter().map(|(_, b)| (counter.count(b), *b)).max_by_key(|x| x.0).unwrap();
                        report(&exact.1, exact.0);
                        println!("[{:.0}s] thread {th} run {run}: proxy best {proxy}, exact best of top {} {}",
                            t0.elapsed().as_secs_f64(), top.len(), exact.0);
                        (exact.1, exact.0)
                    } else {
                        let (b, n, _) = anneal(b0, params, &mut rng, &mut counter, report);
                        (b, n)
                    };
                    let (b2, n) = polish(b1, variant, &mut rng, &mut counter, report);
                    println!("[{:.0}s] thread {th} run {run} done: anneal {n1} polished {n}", t0.elapsed().as_secs_f64());
                    std::fs::write(format!("{dir}/local-{n}.txt"), format!("2g\n{}\n# {n} unique moves\n", b2.diagram())).unwrap();
                }
            });
        }
    });
}
