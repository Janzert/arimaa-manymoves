//! Prints the unique move count of each board file given; with `--ref` also
//! the count from howdah-arimaa's TurnBuilder, and with `--both` silver's
//! count with silver to move as well.

use std::time::Instant;

use howdah_arimaa::Color;
use manymoves::{Counter, parse_diagram, reference_count_for};

fn main() {
    let (mut reference, mut both) = (false, false);
    let mut counter = Counter::default();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--ref" => reference = true,
            "--both" => both = true,
            _ => {
                let board = parse_diagram(&std::fs::read_to_string(&arg).expect("readable board file"));
                let mut sides = vec![("", Color::Gold, board)];
                if both {
                    sides = vec![("gold ", Color::Gold, board), ("silver ", Color::Silver, board.flipped())];
                }
                let mut line = arg.clone();
                for (label, color, b) in sides {
                    let t = Instant::now();
                    let n = counter.count(&b);
                    let ms = t.elapsed().as_secs_f64() * 1000.0;
                    line.push_str(&format!(" {label}{n} {ms:.1}ms"));
                    if reference {
                        let r = reference_count_for(&board.to_position(), color);
                        let ok = if r == n { "ok" } else { "MISMATCH" };
                        line.push_str(&format!(" reference {r} {ok}"));
                    }
                }
                println!("{line}");
            }
        }
    }
}
