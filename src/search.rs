//! Simulated annealing over positions for many unique moves.
//!
//! The `Both` variants score a position by the fewer of gold's moves with
//! gold to move and silver's with silver to move.

use crate::{Board, Counter, GOLD, NKINDS, RABBIT, RANK_1, RANK_8, SILVER, TRAPS, neighbors};

/// Which positions count.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Variant {
    /// A position that can arise in a game with gold to move: both sides
    /// have a rabbit, no rabbit is on its goal, and no trap holds an
    /// unguarded piece.
    Legal,
    /// Gold pieces only.
    GoldOnly,
    /// Legal with either side to move, scored by the side with fewer moves.
    Both,
    /// `Both`, limited to positions that are their own [`Board::flipped`]
    /// (so both sides have the same count).
    BothSym,
}

impl Variant {
    pub fn name(self) -> &'static str {
        match self {
            Variant::Legal => "legal",
            Variant::GoldOnly => "gold",
            Variant::Both => "both",
            Variant::BothSym => "both-sym",
        }
    }

    pub fn from_name(name: &str) -> Option<Variant> {
        [Variant::Legal, Variant::GoldOnly, Variant::Both, Variant::BothSym]
            .into_iter()
            .find(|v| v.name() == name)
    }

    fn both(self) -> bool {
        matches!(self, Variant::Both | Variant::BothSym)
    }
}

/// The score `search` maximizes: gold's unique moves (in up to `steps`
/// steps), or for the `Both` variants the fewer of gold's and silver's.
/// Stops early with a result of at most `floor` once that is certain.
pub fn score(counter: &mut Counter, b: &Board, variant: Variant, steps: u32, floor: usize) -> usize {
    let gold = counter.count_steps(b, steps);
    if !variant.both() || gold <= floor {
        return gold;
    }
    let flipped = b.flipped();
    if flipped == *b { gold } else { gold.min(counter.count_steps(&flipped, steps)) }
}

/// For `BothSym`: `b`'s gold pieces with their flipped copies as silver,
/// or `None` if a gold piece's copy would land on another gold piece.
pub fn symmetrize(b: &Board) -> Option<Board> {
    let mut gold = *b;
    for k in 0..NKINDS {
        let mut bits = b.bb[SILVER][k];
        while bits != 0 {
            let sq = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            gold.remove(SILVER, k, sq);
        }
    }
    let copy = gold.flipped();
    if copy.occ[SILVER] & gold.occ[GOLD] != 0 {
        return None;
    }
    for k in 0..NKINDS {
        gold.bb[SILVER][k] = copy.bb[SILVER][k];
    }
    gold.occ[SILVER] = copy.occ[SILVER];
    gold.hash ^= copy.hash;
    Some(gold)
}

/// Pieces per kind in a full army (rabbit to elephant).
pub const ARMY: [u32; NKINDS] = [8, 2, 2, 2, 1, 1];

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (((self.next_u64() >> 32) * n as u64) >> 32) as usize
    }
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// A random set bit of `b` (nonzero).
    pub fn bit(&mut self, mut b: u64) -> usize {
        for _ in 0..self.below(b.count_ones() as usize) {
            b &= b - 1;
        }
        b.trailing_zeros() as usize
    }
}

pub fn valid(b: &Board, variant: Variant) -> bool {
    if b.bb[GOLD][RABBIT] & RANK_8 != 0 || b.bb[SILVER][RABBIT] & RANK_1 != 0 {
        return false;
    }
    for c in 0..2 {
        if b.occ[c] & TRAPS & !neighbors(b.occ[c]) != 0 {
            return false;
        }
        for (k, &max) in ARMY.iter().enumerate() {
            if b.bb[c][k].count_ones() > max {
                return false;
            }
        }
    }
    match variant {
        Variant::Legal | Variant::Both => b.bb[GOLD][RABBIT] != 0 && b.bb[SILVER][RABBIT] != 0,
        Variant::BothSym => b.bb[GOLD][RABBIT] != 0 && b.flipped() == *b,
        Variant::GoldOnly => b.occ[SILVER] == 0 && b.occ[GOLD] != 0,
    }
}

/// A full gold army on random squares (plus a silver rabbit for `Legal`).
/// For the `Both` variants, gold's army and its flipped copy.
pub fn random_board(rng: &mut Rng, variant: Variant) -> Board {
    if variant.both() {
        loop {
            if let Some(b) = symmetrize(&random_board(rng, Variant::GoldOnly))
                && valid(&b, variant)
            {
                return b;
            }
        }
    }
    loop {
        let mut b = Board::empty();
        for (k, &n) in ARMY.iter().enumerate() {
            for _ in 0..n {
                let sq = rng.bit(!b.all());
                b.put(GOLD, k, sq);
            }
        }
        if variant == Variant::Legal {
            let sq = rng.bit(!b.all());
            b.put(SILVER, RABBIT, sq);
        }
        if valid(&b, variant) {
            return b;
        }
    }
}

/// A random small change to `b` (which may make it invalid). For `BothSym`
/// the change is made to gold and copied to silver.
pub fn mutate(b: &Board, rng: &mut Rng, variant: Variant) -> Board {
    let q = mutate_any(b, rng, variant);
    if variant == Variant::BothSym { symmetrize(&q).unwrap_or(*b) } else { q }
}

fn mutate_any(b: &Board, rng: &mut Rng, variant: Variant) -> Board {
    let mut q = *b;
    let colors = if variant == Variant::GoldOnly { 1 } else { 2 };
    match rng.below(10) {
        // Move a piece to an adjacent empty square.
        0..=3 => {
            let sq = rng.bit(q.all());
            let (c, k) = q.piece_at(sq).unwrap();
            let to = neighbors(1 << sq) & !q.all();
            if to != 0 {
                q.remove(c, k, sq);
                q.put(c, k, rng.bit(to));
            }
        }
        // Move a piece anywhere.
        4..=5 => {
            let sq = rng.bit(q.all());
            let (c, k) = q.piece_at(sq).unwrap();
            q.remove(c, k, sq);
            q.put(c, k, rng.bit(!q.all() & !(1 << sq)));
        }
        // Swap two different pieces.
        6..=7 => {
            let a = rng.bit(q.all());
            let b2 = rng.bit(q.all());
            let (pa, pb) = (q.piece_at(a).unwrap(), q.piece_at(b2).unwrap());
            if pa != pb {
                q.remove(pa.0, pa.1, a);
                q.remove(pb.0, pb.1, b2);
                q.put(pa.0, pa.1, b2);
                q.put(pb.0, pb.1, a);
            }
        }
        // Remove a piece.
        8 => {
            let sq = rng.bit(q.all());
            let (c, k) = q.piece_at(sq).unwrap();
            q.remove(c, k, sq);
        }
        // Add a piece.
        _ => {
            let c = rng.below(colors);
            let k = rng.below(NKINDS);
            if q.bb[c][k].count_ones() < ARMY[k] && q.all() != !0 {
                q.put(c, k, rng.bit(!q.all()));
            }
        }
    }
    q
}

pub struct Anneal {
    pub variant: Variant,
    pub iters: u64,
    pub t_start: f64,
    pub t_end: f64,
    /// Steps per turn for the counts it anneals on: 4 is the real count,
    /// fewer is a cheaper proxy.
    pub steps: u32,
    /// How many of the best distinct boards visited to return.
    pub keep: usize,
}

/// One annealing run from `start`; calls `report` on each new best of the
/// run. Returns the best board and its count, and the best `keep` distinct
/// boards visited (best first).
pub fn anneal(
    start: Board,
    params: &Anneal,
    rng: &mut Rng,
    counter: &mut Counter,
    mut report: impl FnMut(&Board, usize),
) -> (Board, usize, Vec<(usize, Board)>) {
    let mut cur = start;
    let mut cur_n = score(counter, &cur, params.variant, params.steps, 0);
    let (mut best, mut best_n) = (cur, cur_n);
    let mut top: Vec<(usize, Board)> = vec![(cur_n, cur)];
    report(&best, best_n);
    for i in 0..params.iters {
        let frac = i as f64 / params.iters as f64;
        let t = params.t_start * (params.t_end / params.t_start).powf(frac);
        let q = mutate(&cur, rng, params.variant);
        if q == cur || !valid(&q, params.variant) {
            continue;
        }
        let n = score(counter, &q, params.variant, params.steps, 0);
        if n >= cur_n || rng.unit() < ((n as f64 - cur_n as f64) / t).exp() {
            cur = q;
            cur_n = n;
            let full = top.len() >= params.keep;
            if (!full || n > top.last().unwrap().0) && !top.iter().any(|(_, b)| b.hash == q.hash) {
                let at = top.partition_point(|(m, _)| *m >= n);
                top.insert(at, (n, q));
                top.truncate(params.keep.max(1));
            }
            if n > best_n {
                best = q;
                best_n = n;
                report(&best, best_n);
            }
        }
    }
    (best, best_n, top)
}

/// Every position one change away from `b`: a piece moved to any empty
/// square, two different pieces swapped, a piece removed or added.
/// For `BothSym` each change is made to gold and copied to silver.
pub fn neighborhood(b: &Board, variant: Variant) -> Vec<Board> {
    let mut out = Vec::new();
    let colors = if variant == Variant::GoldOnly { 1 } else { 2 };
    let occupied: Vec<(usize, (usize, usize))> =
        (0..64).filter_map(|sq| b.piece_at(sq).map(|p| (sq, p))).collect();
    let empties: Vec<usize> = (0..64).filter(|&sq| b.all() >> sq & 1 == 0).collect();
    for &(sq, (c, k)) in &occupied {
        let mut without = *b;
        without.remove(c, k, sq);
        out.push(without);
        for &to in &empties {
            let mut q = without;
            q.put(c, k, to);
            out.push(q);
        }
    }
    for (i, &(a, pa)) in occupied.iter().enumerate() {
        for &(s, ps) in &occupied[i + 1..] {
            if pa != ps {
                let mut q = *b;
                q.remove(pa.0, pa.1, a);
                q.remove(ps.0, ps.1, s);
                q.put(pa.0, pa.1, s);
                q.put(ps.0, ps.1, a);
                out.push(q);
            }
        }
    }
    for c in 0..colors {
        for (k, &max) in ARMY.iter().enumerate() {
            if b.bb[c][k].count_ones() < max {
                for &to in &empties {
                    let mut q = *b;
                    q.put(c, k, to);
                    out.push(q);
                }
            }
        }
    }
    if variant == Variant::BothSym {
        out = out.iter().filter_map(symmetrize).filter(|q| q != b).collect();
        out.sort_by_key(|q| q.hash);
        out.dedup();
    }
    out.retain(|q| valid(q, variant));
    out
}

/// First-improvement hill climbing over [`neighborhood`] until no single
/// change helps. Returns a local maximum.
pub fn polish(
    start: Board,
    variant: Variant,
    rng: &mut Rng,
    counter: &mut Counter,
    mut report: impl FnMut(&Board, usize),
) -> (Board, usize) {
    let mut cur = start;
    let mut cur_n = score(counter, &cur, variant, 4, 0);
    'outer: loop {
        let mut nb = neighborhood(&cur, variant);
        // Shuffle so repeated polishes don't always take the same path.
        for i in (1..nb.len()).rev() {
            nb.swap(i, rng.below(i + 1));
        }
        for q in nb {
            let n = score(counter, &q, variant, 4, cur_n);
            if n > cur_n {
                cur = q;
                cur_n = n;
                report(&cur, cur_n);
                continue 'outer;
            }
        }
        return (cur, cur_n);
    }
}
