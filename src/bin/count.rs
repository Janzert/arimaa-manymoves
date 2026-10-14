//! Prints the unique move count of each board file given; with `--ref` also
//! the count from howdah-arimaa's TurnBuilder.

use std::time::Instant;

use manymoves::{Counter, parse_diagram, reference_count};

fn main() {
    let mut reference = false;
    let mut counter = Counter::default();
    for arg in std::env::args().skip(1) {
        if arg == "--ref" {
            reference = true;
            continue;
        }
        let board = parse_diagram(&std::fs::read_to_string(&arg).expect("readable board file"));
        let t = Instant::now();
        let n = counter.count(&board);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        if reference {
            let r = reference_count(&board.to_position());
            let ok = if r == n { "ok" } else { "MISMATCH" };
            println!("{arg} {n} {ms:.1}ms reference {r} {ok}");
        } else {
            println!("{arg} {n} {ms:.1}ms");
        }
    }
}
