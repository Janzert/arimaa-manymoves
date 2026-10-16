//! Compares the fast counter with howdah-arimaa's TurnBuilder on random
//! crowded positions, where pushes, pulls, freezing and traps all come up.
//! Silver's count (the fast count of the flipped board) is checked too.
//! Usage: crosscheck [positions] [seed]

use howdah_arimaa::Color;
use manymoves::search::{ARMY, Rng};
use manymoves::{Board, Counter, NKINDS, TRAPS, neighbors, reference_count_for};

fn random_crowded(rng: &mut Rng) -> Board {
    loop {
        // Pieces go in a random rectangle so they touch each other.
        let (w, h) = (3 + rng.below(4), 3 + rng.below(4));
        let (f0, r0) = (rng.below(9 - w), rng.below(9 - h));
        let mut area = 0u64;
        for r in r0..r0 + h {
            for f in f0..f0 + w {
                area |= 1 << (r * 8 + f);
            }
        }
        let mut b = Board::empty();
        let n = 2 + rng.below((w * h * 3 / 4).min(20));
        for _ in 0..n {
            let (c, k) = (rng.below(2), rng.below(NKINDS));
            if b.bb[c][k].count_ones() < ARMY[k] && area & !b.all() != 0 {
                b.put(c, k, rng.bit(area & !b.all()));
            }
        }
        let unguarded = (0..2).any(|c| b.occ[c] & TRAPS & !neighbors(b.occ[c]) != 0);
        if !unguarded && b.occ[0] != 0 {
            return b;
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let n: usize = args.get(1).map_or(2000, |s| s.parse().unwrap());
    let seed: u64 = args.get(2).map_or(1, |s| s.parse().unwrap());
    let mut rng = Rng::new(seed);
    let mut counter = Counter::default();
    let mut bad = 0;
    for i in 0..n {
        let b = random_crowded(&mut rng);
        let pos = b.to_position();
        for (side, color, board) in [("gold", Color::Gold, b), ("silver", Color::Silver, b.flipped())] {
            let (fast, slow) = (counter.count(&board), reference_count_for(&pos, color));
            if fast != slow {
                bad += 1;
                println!("#{i} {side}: fast {fast} reference {slow}\n{}", b.diagram());
            }
        }
    }
    println!("{n} positions, {bad} mismatches");
}
