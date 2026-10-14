//! Counting the distinct positions one Arimaa turn can reach.
//!
//! A turn is 1 to 4 steps made of single steps, pushes and pulls (a push or
//! pull takes 2 steps). Its "unique moves" are the distinct boards it can end
//! on, other than the board it started from. Repetition is ignored. Gold is
//! always the side to move here.
//!
//! [`Counter`] is the fast count used by the search. [`reference_count`]
//! enumerates the same set with howdah-arimaa's `TurnBuilder`, step by step
//! with its full legality checks, to cross-check it.

use std::collections::HashSet;

use howdah_arimaa::{Color, Piece, PieceKind, Position, Square, StepKind, TurnBuilder};

pub mod search;

pub const GOLD: usize = 0;
pub const SILVER: usize = 1;
pub const RABBIT: usize = 0;
pub const NKINDS: usize = 6;
pub const KINDS: [PieceKind; NKINDS] = [
    PieceKind::Rabbit,
    PieceKind::Cat,
    PieceKind::Dog,
    PieceKind::Horse,
    PieceKind::Camel,
    PieceKind::Elephant,
];
pub const COLORS: [Color; 2] = [Color::Gold, Color::Silver];
pub const TRAPS: u64 = (1 << 18) | (1 << 21) | (1 << 42) | (1 << 45);
const FILE_A: u64 = 0x0101_0101_0101_0101;
const FILE_H: u64 = FILE_A << 7;
pub const RANK_1: u64 = 0xFF;
pub const RANK_8: u64 = 0xFF << 56;

#[inline]
fn north(b: u64) -> u64 {
    b << 8
}
#[inline]
fn south(b: u64) -> u64 {
    b >> 8
}
#[inline]
fn east(b: u64) -> u64 {
    (b & !FILE_H) << 1
}
#[inline]
fn west(b: u64) -> u64 {
    (b & !FILE_A) >> 1
}
#[inline]
pub fn neighbors(b: u64) -> u64 {
    north(b) | south(b) | east(b) | west(b)
}

const fn splitmix64(state: u64) -> (u64, u64) {
    let state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    (state, z ^ (z >> 31))
}

struct Keys {
    pieces: [[[u64; 64]; NKINDS]; 2],
    steps_left: [u64; 5],
}

const KEYS: Keys = {
    let mut pieces = [[[0u64; 64]; NKINDS]; 2];
    let mut state = 0x6D61_6E79_6D76_u64;
    let mut i = 0;
    while i < 2 * NKINDS * 64 {
        let (next, key) = splitmix64(state);
        state = next;
        pieces[i / (NKINDS * 64)][i / 64 % NKINDS][i % 64] = key;
        i += 1;
    }
    let mut steps_left = [0u64; 5];
    let mut j = 0;
    while j < 5 {
        let (next, key) = splitmix64(state);
        state = next;
        steps_left[j] = key;
        j += 1;
    }
    Keys { pieces, steps_left }
};

/// Bitboards by color and kind, with a Zobrist hash of the pieces.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Board {
    pub bb: [[u64; NKINDS]; 2],
    pub occ: [u64; 2],
    pub hash: u64,
}

impl Board {
    pub fn empty() -> Board {
        Board { bb: [[0; NKINDS]; 2], occ: [0; 2], hash: 0 }
    }

    pub fn from_position(pos: &Position) -> Board {
        let mut b = Board::empty();
        for (sq, piece) in pos.pieces() {
            b.put(piece.color.index(), piece.kind.index(), sq.index() as usize);
        }
        b
    }

    pub fn to_position(&self) -> Position {
        let pieces = (0..64).filter_map(|sq| {
            let (c, k) = self.piece_at(sq)?;
            Some((Piece { color: COLORS[c], kind: KINDS[k] }, Square::new(sq as u8).unwrap()))
        });
        Position::from_pieces(Color::Gold, pieces).expect("one piece per square")
    }

    #[inline]
    pub fn all(&self) -> u64 {
        self.occ[0] | self.occ[1]
    }

    #[inline]
    pub fn put(&mut self, c: usize, k: usize, sq: usize) {
        let b = 1u64 << sq;
        self.bb[c][k] |= b;
        self.occ[c] |= b;
        self.hash ^= KEYS.pieces[c][k][sq];
    }

    #[inline]
    pub fn remove(&mut self, c: usize, k: usize, sq: usize) {
        let b = 1u64 << sq;
        self.bb[c][k] &= !b;
        self.occ[c] &= !b;
        self.hash ^= KEYS.pieces[c][k][sq];
    }

    #[inline]
    fn shift(&mut self, c: usize, k: usize, from: usize, to: usize) {
        let bits = (1u64 << from) | (1u64 << to);
        self.bb[c][k] ^= bits;
        self.occ[c] ^= bits;
        self.hash ^= KEYS.pieces[c][k][from] ^ KEYS.pieces[c][k][to];
    }

    #[inline]
    pub fn kind_at(&self, c: usize, sq: usize) -> Option<usize> {
        let b = 1u64 << sq;
        if self.occ[c] & b == 0 {
            return None;
        }
        (0..NKINDS).find(|&k| self.bb[c][k] & b != 0)
    }

    pub fn piece_at(&self, sq: usize) -> Option<(usize, usize)> {
        (0..2).find_map(|c| self.kind_at(c, sq).map(|k| (c, k)))
    }

    /// Removes unguarded pieces on the traps next to `sq` (the square a piece
    /// just left).
    #[inline]
    fn captures_near(&mut self, sq: usize) {
        let traps = TRAPS & neighbors(1u64 << sq) & self.all();
        if traps == 0 {
            return;
        }
        let t = traps.trailing_zeros() as usize;
        let c = if self.occ[0] >> t & 1 != 0 { 0 } else { 1 };
        if neighbors(1u64 << t) & self.occ[c] == 0 {
            let k = self.kind_at(c, t).unwrap();
            self.remove(c, k, t);
        }
    }

    /// Gold pieces that aren't frozen.
    pub fn unfrozen(&self, c: usize) -> u64 {
        let op = 1 - c;
        let friends = neighbors(self.occ[c]);
        let mut stronger = 0;
        let mut free = 0;
        for k in (0..NKINDS).rev() {
            free |= self.bb[c][k] & (friends | !neighbors(stronger));
            stronger |= self.bb[op][k];
        }
        free
    }

    /// Board diagram with `x` for empty traps.
    pub fn diagram(&self) -> String {
        let mut s = String::from(" +-----------------+\n");
        for r in (0..8).rev() {
            s.push_str(&format!("{}|", r + 1));
            for f in 0..8 {
                let sq = r * 8 + f;
                let ch = match self.piece_at(sq) {
                    Some((c, k)) => Piece { color: COLORS[c], kind: KINDS[k] }.letter(),
                    None if TRAPS >> sq & 1 != 0 => 'x',
                    None => '.',
                };
                s.push(' ');
                s.push(ch);
            }
            s.push_str(" |\n");
        }
        s.push_str(" +-----------------+\n   a b c d e f g h\n");
        s
    }
}

/// Reads a board diagram (`8| . . r ... |` rows); other lines are ignored.
pub fn parse_diagram(text: &str) -> Board {
    let mut b = Board::empty();
    for line in text.lines() {
        let bytes = line.as_bytes();
        if bytes.len() < 4 || !(b'1'..=b'8').contains(&bytes[0]) || bytes[1] != b'|' {
            continue;
        }
        let r = (bytes[0] - b'1') as usize;
        for f in 0..8 {
            let ch = bytes.get(3 + 2 * f).copied().unwrap_or(b' ') as char;
            if let Some(p) = Piece::from_letter(ch) {
                b.put(p.color.index(), p.kind.index(), r * 8 + f);
            }
        }
    }
    b
}

/// Open-addressing set of 64-bit keys that clears in time proportional to
/// its size.
pub struct KeySet {
    tab: Vec<u64>,
    used: Vec<u32>,
    mask: usize,
}

impl KeySet {
    pub fn new(bits: u32) -> KeySet {
        KeySet { tab: vec![0; 1 << bits], used: Vec::new(), mask: (1 << bits) - 1 }
    }

    pub fn len(&self) -> usize {
        self.used.len()
    }

    pub fn is_empty(&self) -> bool {
        self.used.is_empty()
    }

    #[inline]
    fn slot(&self, key: u64) -> usize {
        (key.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 20) as usize & self.mask
    }

    #[inline]
    pub fn insert(&mut self, key: u64) -> bool {
        let key = key.max(1);
        if self.used.len() * 2 > self.tab.len() {
            self.grow();
        }
        let mut i = self.slot(key);
        loop {
            match self.tab[i] {
                0 => break,
                k if k == key => return false,
                _ => i = (i + 1) & self.mask,
            }
        }
        self.tab[i] = key;
        self.used.push(i as u32);
        true
    }

    pub fn clear(&mut self) {
        for &i in &self.used {
            self.tab[i as usize] = 0;
        }
        self.used.clear();
    }

    pub fn keys(&self) -> impl Iterator<Item = u64> + '_ {
        self.used.iter().map(|&i| self.tab[i as usize])
    }

    fn grow(&mut self) {
        let keys: Vec<u64> = self.keys().collect();
        self.tab = vec![0; self.tab.len() * 2];
        self.mask = self.tab.len() - 1;
        self.used.clear();
        for k in keys {
            self.insert(k);
        }
    }
}

/// Calls `f(child, steps_used)` for every gold single step, push and pull.
#[inline]
pub fn actions(p: &Board, steps_left: u32, mut f: impl FnMut(&Board, u32)) {
    let (me, op) = (GOLD, SILVER);
    let empty = !p.all();
    let free = p.unfrozen(me);
    for k in 0..NKINDS {
        let mut movers = p.bb[me][k] & free;
        let weaker_enemies: u64 = p.bb[op][..k].iter().fold(0, |a, b| a | b);
        while movers != 0 {
            let from = movers.trailing_zeros() as usize;
            movers &= movers - 1;
            let fb = 1u64 << from;
            let dests = if k == RABBIT { north(fb) | east(fb) | west(fb) } else { neighbors(fb) };
            let mut dests = dests & empty;
            let victims = neighbors(fb) & weaker_enemies;
            while dests != 0 {
                let to = dests.trailing_zeros() as usize;
                dests &= dests - 1;
                let mut q = *p;
                q.shift(me, k, from, to);
                q.captures_near(from);
                f(&q, 1);
                if steps_left < 2 {
                    continue;
                }
                // Pulls: a weaker enemy next to `from` follows into it.
                let mut vs = victims;
                while vs != 0 {
                    let v = vs.trailing_zeros() as usize;
                    vs &= vs - 1;
                    let vk = p.kind_at(op, v).unwrap();
                    let mut r = q;
                    r.shift(op, vk, v, from);
                    r.captures_near(v);
                    f(&r, 2);
                }
            }
            if steps_left < 2 {
                continue;
            }
            // Pushes: a weaker enemy moves to an empty square and we follow.
            let mut vs = victims;
            while vs != 0 {
                let v = vs.trailing_zeros() as usize;
                vs &= vs - 1;
                let vk = p.kind_at(op, v).unwrap();
                let mut pd = neighbors(1u64 << v) & empty;
                while pd != 0 {
                    let to = pd.trailing_zeros() as usize;
                    pd &= pd - 1;
                    let mut q = *p;
                    q.shift(op, vk, v, to);
                    q.captures_near(v);
                    q.shift(me, k, from, v);
                    q.captures_near(from);
                    f(&q, 2);
                }
            }
        }
    }
}

/// Fast unique move counter, reusing its tables between calls.
pub struct Counter {
    pub finals: KeySet,
    partials: KeySet,
    start: u64,
}

impl Default for Counter {
    fn default() -> Self {
        Counter { finals: KeySet::new(20), partials: KeySet::new(17), start: 0 }
    }
}

impl Counter {
    fn dfs(&mut self, p: &Board, steps_left: u32) {
        actions(p, steps_left, |q, used| {
            if q.hash != self.start {
                self.finals.insert(q.hash);
            }
            let left = steps_left - used;
            if left > 0 && self.partials.insert(q.hash ^ KEYS.steps_left[left as usize]) {
                self.dfs(q, left);
            }
        });
    }

    /// Unique moves for gold from `p`.
    pub fn count(&mut self, p: &Board) -> usize {
        self.count_steps(p, 4)
    }

    /// Distinct boards reachable in 1 to `steps` steps (a cheaper proxy
    /// for [`Counter::count`] with fewer steps).
    pub fn count_steps(&mut self, p: &Board, steps: u32) -> usize {
        self.finals.clear();
        self.partials.clear();
        self.start = p.hash;
        self.dfs(p, steps);
        self.finals.len()
    }
}

/// Unique moves for gold from `pos`, by trying every legal step sequence with
/// howdah-arimaa's `TurnBuilder`. Slow; for cross-checking [`Counter`].
pub fn reference_count(pos: &Position) -> usize {
    let mut start = pos.clone();
    start.set_side_to_move(Color::Gold);
    let mut seen = HashSet::new();
    let mut finals = HashSet::new();
    let mut tb = TurnBuilder::new(&start);
    reference_dfs(&mut tb, &mut seen, &mut finals);
    finals.len()
}

type SearchKey = (String, usize, Option<Square>, Option<(Square, PieceKind)>);

fn reference_dfs(tb: &mut TurnBuilder, seen: &mut HashSet<SearchKey>, finals: &mut HashSet<String>) {
    for (step, _) in tb.legal_steps() {
        tb.try_step(step).expect("legal step");
        // What the next steps depend on: the board, steps left, a pending
        // push, and the last simple step (which a pull may follow).
        let last = tb.steps().last().unwrap();
        let pull =
            (last.kind == StepKind::Simple).then_some((last.effect.step.from, last.effect.step.piece.kind));
        let board = tb.position().to_short_string();
        if tb.can_finish().is_ok() {
            finals.insert(board.clone());
        }
        let key = (board, tb.steps_left(), tb.push_pending(), pull);
        if tb.steps_left() > 0 && seen.insert(key) {
            reference_dfs(tb, seen, finals);
        }
        tb.undo();
    }
}
