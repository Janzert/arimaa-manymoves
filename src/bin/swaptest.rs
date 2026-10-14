//! Swaps the gold rabbits on the given squares with stronger gold pieces (each
//! one alone, and every pair) and prints the counts. Usage: swaptest BOARD SQ...

use manymoves::search::{Variant, valid};
use manymoves::{Board, Counter, GOLD, RABBIT, parse_diagram};

fn sq(s: &str) -> usize {
    let b = s.as_bytes();
    (b[1] - b'1') as usize * 8 + (b[0] - b'a') as usize
}

fn name(s: usize) -> String {
    format!("{}{}", (b'a' + (s % 8) as u8) as char, s / 8 + 1)
}

fn swap(b: &Board, r: usize, o: usize) -> Board {
    let k = b.kind_at(GOLD, o).unwrap();
    let mut q = *b;
    q.remove(GOLD, RABBIT, r);
    q.remove(GOLD, k, o);
    q.put(GOLD, k, r);
    q.put(GOLD, RABBIT, o);
    q
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let b = parse_diagram(&std::fs::read_to_string(&args[1]).unwrap());
    let rs: Vec<usize> = args[2..].iter().map(|s| sq(s)).collect();
    let strong: Vec<usize> =
        (0..64).filter(|&s| b.occ[GOLD] >> s & 1 != 0 && b.kind_at(GOLD, s) != Some(RABBIT)).collect();
    let mut c = Counter::default();
    let base = c.count(&b) as i64;
    let letter =
        |q: &Board, s: usize| manymoves::KINDS[q.kind_at(GOLD, s).unwrap()].letter().to_ascii_uppercase();
    let mut out = Vec::new();
    for &r in &rs {
        for &o in &strong {
            let q = swap(&b, r, o);
            if valid(&q, Variant::Legal) {
                out.push((
                    c.count(&q) as i64 - base,
                    format!("{}<->{} ({} to {})", name(r), name(o), letter(&q, r), name(r)),
                ));
            }
        }
    }
    if rs.len() == 2 {
        for &o1 in &strong {
            for &o2 in &strong {
                if o1 != o2 {
                    let q = swap(&swap(&b, rs[0], o1), rs[1], o2);
                    if valid(&q, Variant::Legal) {
                        out.push((
                            c.count(&q) as i64 - base,
                            format!(
                                "{} to {}, {} to {}",
                                letter(&q, rs[0]),
                                name(rs[0]),
                                letter(&q, rs[1]),
                                name(rs[1])
                            ),
                        ));
                    }
                }
            }
        }
    }
    out.sort();
    println!("base {base}; best changes:");
    for (d, s) in out.iter().rev().take(8) {
        println!("  {d:+7}  {s}");
    }
    println!("worst: {:+} ({})", out[0].0, out[0].1);
}
