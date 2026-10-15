//! An upper bound on unique moves over all positions (see BOUND.md).
//!
//! Each end board of a turn has a description: every piece whose state
//! changed, with its end square or "removed". Each step moves one piece one
//! square, so a piece's Manhattan displacement costs at least that many
//! steps, and the costs add up to at most 4. The number of descriptions
//! within that budget bounds the number of end boards. It's a truncated
//! product of polynomials, t counting steps.
//!
//! Removals and survivals on traps need care. A piece is removed only on a
//! trap with no friendly neighbor, and after every step each trap's piece has
//! a friendly neighbor. So the friendly pieces next to a trap at the start
//! (its "guards") must all have moved before anything is removed there, and a
//! piece still on its trap at the end has a friendly neighbor at the end:
//! an unmoved guard, a guard that moved to another side of the trap, or a
//! "witness" from elsewhere that ended next to it. Each trap with its guards
//! and occupant gets joint polynomials for these cases.

const D: usize = 5; // degrees 0..=4

#[derive(Clone, Copy, PartialEq, Debug)]
struct Poly([i64; D]);

impl Poly {
    const ZERO: Poly = Poly([0; D]);

    fn one() -> Poly {
        Poly::term(0, 1)
    }
    fn term(k: usize, c: i64) -> Poly {
        let mut p = Poly::ZERO;
        if k < D {
            p.0[k] = c;
        }
        p
    }
    fn mul(&self, o: &Poly) -> Poly {
        let mut r = Poly::ZERO;
        for a in 0..D {
            if self.0[a] == 0 {
                continue;
            }
            for b in 0..D - a {
                r.0[a + b] += self.0[a] * o.0[b];
            }
        }
        r
    }
    fn add(&self, o: &Poly) -> Poly {
        Poly(std::array::from_fn(|k| self.0[k] + o.0[k]))
    }
    fn max(&self, o: &Poly) -> Poly {
        Poly(std::array::from_fn(|k| self.0[k].max(o.0[k])))
    }
    fn sub(&self, o: &Poly) -> Poly {
        Poly(std::array::from_fn(|k| self.0[k] - o.0[k]))
    }
    fn scale(&self, c: i64) -> Poly {
        Poly(self.0.map(|x| x * c))
    }
    /// Negative coefficients raised to 0.
    fn clamp(&self) -> Poly {
        Poly(self.0.map(|x| x.max(0)))
    }
    fn pow(&self, n: usize) -> Poly {
        (0..n).fold(Poly::one(), |acc, _| acc.mul(self))
    }
    fn min_degree(&self) -> usize {
        (0..D).find(|&k| self.0[k] != 0).unwrap_or(D)
    }
    /// End boards: descriptions within 4 steps, leaving out the empty one.
    fn total(&self) -> i64 {
        self.0[1..].iter().sum()
    }
}

type Sq = (i32, i32);

const TRAPS: [Sq; 4] = [(2, 2), (5, 2), (2, 5), (5, 5)]; // c3 f3 c6 f6

fn dist(a: Sq, b: Sq) -> usize {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as usize
}

fn squares() -> impl Iterator<Item = Sq> {
    (0..8).flat_map(|r| (0..8).map(move |f| (f, r)))
}

fn trap_neighbors(t: usize) -> Vec<Sq> {
    squares().filter(|&s| dist(s, TRAPS[t]) == 1).collect()
}

#[derive(Clone, Copy, PartialEq)]
enum Class {
    Trap,
    NextToTrap,
    Other,
}

fn class(sq: Sq) -> Class {
    match TRAPS.iter().map(|&t| dist(sq, t)).min().unwrap() {
        0 => Class::Trap,
        1 => Class::NextToTrap,
        _ => Class::Other,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Rabbit,
    Other,
}

/// A piece's outcomes by cost from square `s`: an end square at each
/// distance, plus removal on a trap it isn't next to (its own trap's removals
/// are in the trap polynomials), costing at least the distance to that trap.
fn square_poly(kind: Kind, s: Sq) -> Poly {
    let mut c = Poly::one();
    for e in squares() {
        let d = dist(s, e);
        if (1..D).contains(&d) && !(kind == Kind::Rabbit && e.1 < s.1) {
            c.0[d] += 1;
        }
    }
    let other_trap = TRAPS.iter().map(|&t| dist(s, t)).filter(|&d| d >= 2).min().unwrap();
    if other_trap < D {
        c.0[other_trap] += 1;
    }
    c
}

/// The coefficientwise maximum of `square_poly` over the squares of a class.
fn piece_poly(kind: Kind, cls: Class) -> Poly {
    squares().filter(|&s| class(s) == cls).fold(Poly::ZERO, |a, s| a.max(&square_poly(kind, s)))
}

/// Endings next to trap `to`, by cost, for a piece starting on one of
/// `from` (the maximum over them).
fn reach(from: &[Sq], to: usize) -> Poly {
    let ends = trap_neighbors(to);
    let mut best = Poly::ZERO;
    for &q in from {
        let mut c = Poly::ZERO;
        for &e in &ends {
            c = c.add(&Poly::term(dist(q, e), 1));
        }
        best = best.max(&c);
    }
    best
}

/// A member of a trap's group: its occupant or one of its guards.
#[derive(Clone, Copy)]
struct Piece {
    at: Sq,
    kind: Kind,
    occupant: bool,
    away: bool,        // ends elsewhere at a cost counted outside the group
    guarding: bool,    // when away, it still ends next to this trap
    fixed: Option<Sq>, // ends on this square, at a cost counted outside the group
}

/// A member's end state.
#[derive(Clone, Copy, PartialEq)]
enum End {
    Stay,
    At(Sq),
    Removed,
    Away,      // next to another trap; its cost is in the witness edge
    Fixed(Sq), // on this square, at a cost counted outside the group
}

/// One trap's joint polynomials, by exact enumeration of its members' end
/// states: `n` for those that need nothing from outside, `s` for those that
/// leave a piece on the trap with no member next to it, so a witness from
/// outside must end next to it.
///
/// Each end state is counted at a lower bound of its cost: the members'
/// Manhattan displacements, 2 for a guard that must have stepped away and
/// back, and for a removal the distance to the trap where it happens. End
/// squares are distinct and never the start of a member that stays. Removal
/// on this trap needs the trap empty of other pieces and every other guard
/// to have moved. Anything on the trap at the end has a gold neighbor then.
fn cluster(t: usize, members: &[Piece]) -> (Poly, Poly) {
    cluster_blocked(t, members, &[])
}

/// `cluster` with members unable to end on the `blocked` squares.
fn cluster_blocked(t: usize, members: &[Piece], blocked: &[Sq]) -> (Poly, Poly) {
    let mut ends: Vec<Vec<(End, usize)>> = Vec::new();
    for p in members {
        let mut v = Vec::new();
        if let Some(q) = p.fixed {
            v.push((End::Fixed(q), 0));
        } else if p.away {
            v.push((End::Away, 0));
        } else {
            v.push((End::Stay, 0));
            // Removal's cost is settled with the others' ends; this is a floor.
            let floor = [true, false].iter().filter_map(|&own| removal_cost(t, p, own)).min();
            if let Some(c) = floor {
                v.push((End::Removed, c));
            }
            for e in squares() {
                let d = dist(p.at, e);
                if (1..D).contains(&d) && !(p.kind == Kind::Rabbit && e.1 < p.at.1) && !blocked.contains(&e) {
                    v.push((End::At(e), d));
                }
            }
        }
        ends.push(v);
    }
    let mut walk =
        Walk { t, members, ends, pick: vec![End::Stay; members.len()], n: Poly::ZERO, s: Poly::ZERO };
    walk.go(0, 0);
    (walk.n, walk.s)
}

/// Enumerates the members' end states for `cluster`.
struct Walk<'a> {
    t: usize,
    members: &'a [Piece],
    ends: Vec<Vec<(End, usize)>>, // each member's end states, with cost floors
    pick: Vec<End>,
    n: Poly,
    s: Poly,
}

impl Walk<'_> {
    fn go(&mut self, i: usize, floor: usize) {
        if i < self.members.len() {
            for k in 0..self.ends[i].len() {
                let (e, c) = self.ends[i][k];
                if floor + c < D {
                    self.pick[i] = e;
                    self.go(i + 1, floor + c);
                }
            }
            return;
        }
        // End squares are distinct.
        let squares: Vec<Sq> = self
            .members
            .iter()
            .zip(&self.pick)
            .filter_map(|(p, e)| match e {
                End::Stay => Some(p.at),
                End::At(q) | End::Fixed(q) => Some(*q),
                _ => None,
            })
            .collect();
        for (a, x) in squares.iter().enumerate() {
            if squares[a + 1..].contains(x) {
                return;
            }
        }
        let Some(cost) = min_cost(self.t, self.members, &self.pick) else { return };
        if cost >= D {
            return;
        }
        // Something on the trap at the end needs a gold neighbor then.
        let trap = TRAPS[self.t];
        let occupied = squares.contains(&trap);
        let guarded =
            squares.iter().any(|&q| dist(q, trap) == 1) || self.members.iter().any(|p| p.away && p.guarding);
        if occupied && !guarded {
            self.s.0[cost] += 1;
        } else {
            self.n.0[cost] += 1;
        }
    }
}

/// The fewest steps for the members' end states `pick`, or None if the
/// rules rule them out.
fn min_cost(t: usize, members: &[Piece], pick: &[End]) -> Option<usize> {
    let mut base = 0;
    for (p, e) in members.iter().zip(pick) {
        if let End::At(q) = e {
            base += dist(p.at, *q);
        }
    }
    let removed: Vec<usize> = (0..members.len()).filter(|&i| pick[i] == End::Removed).collect();
    let mut best = None;
    // Each removed member is removed on this trap or the nearest other one.
    'choice: for mask in 0..1u32 << removed.len() {
        let mut cost = base;
        let mut must_move = vec![false; members.len()]; // guards that left at some point
        for (b, &i) in removed.iter().enumerate() {
            let own = mask >> b & 1 == 1;
            let Some(c) = removal_cost(t, &members[i], own) else { continue 'choice };
            cost += c;
            if own {
                // On this trap: it is empty but for this piece, and every other
                // guard is away.
                for (j, m) in must_move.iter_mut().enumerate() {
                    *m |= j != i;
                }
            }
        }
        // A member that stays but had to leave stepped away and back.
        for (j, e) in pick.iter().enumerate() {
            if must_move[j] && *e == End::Stay {
                cost += 2;
            }
        }
        // An occupant removed on its trap waits for its guards to leave, so a
        // guard that ends on the trap, or is removed there, first went
        // elsewhere: by parity, two steps more than the one onto it.
        let occupant_gone =
            removed.iter().enumerate().any(|(b, &i)| members[i].occupant && mask >> b & 1 == 1);
        if occupant_gone {
            for (b, &i) in removed.iter().enumerate() {
                if !members[i].occupant && mask >> b & 1 == 1 {
                    cost += 2;
                }
            }
            for (p, e) in members.iter().zip(pick) {
                if !p.occupant && *e == End::At(TRAPS[t]) {
                    cost += 2;
                }
            }
        }
        best = Some(best.map_or(cost, |b: usize| b.min(cost)));
    }
    best
}

/// The fewest steps to remove member `p` of trap `t`'s group on that trap
/// (`own`) or on another one, if it can reach one. Rabbits never step south.
fn removal_cost(t: usize, p: &Piece, own: bool) -> Option<usize> {
    let reachable = |x: Sq| !(p.kind == Kind::Rabbit && x.1 < p.at.1);
    if own {
        let c = if p.occupant { 0 } else { 1 };
        return reachable(TRAPS[t]).then_some(c);
    }
    TRAPS.iter().enumerate().filter(|&(k, &x)| k != t && reachable(x)).map(|(_, &x)| dist(p.at, x)).min()
}

/// Which member of a trap's group serves as another trap's witness.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Member {
    RabbitGuard,
    OtherGuard,
    Occupant,
}

const MEMBERS: [Member; 3] = [Member::RabbitGuard, Member::OtherGuard, Member::Occupant];

/// What one trap holds at the start, and its gold neighbors.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TrapUse {
    occupant: Option<Kind>,
    rabbits: usize,
    others: usize,
}

impl TrapUse {
    fn has(&self, m: Member) -> usize {
        match m {
            Member::RabbitGuard => self.rabbits,
            Member::OtherGuard => self.others,
            Member::Occupant => usize::from(self.occupant.is_some()),
        }
    }
    fn members(&self) -> Vec<Member> {
        MEMBERS.into_iter().filter(|&m| self.has(m) > 0).collect()
    }
    fn gold(&self) -> (usize, usize) {
        let occ = |k| usize::from(self.occupant == Some(k));
        (self.rabbits + occ(Kind::Rabbit), self.others + occ(Kind::Other))
    }
}

struct Polys {
    free: [Poly; 2], // by kind, away from traps
    w: Poly,         // free witnesses for one trap
    // h[member][from trap][to trap]: a member's endings next to another trap
    h: [[[Poly; 4]; 4]; 3],
}

impl Polys {
    fn new() -> Polys {
        use Class::*;
        let k = |c| [piece_poly(Kind::Rabbit, c), piece_poly(Kind::Other, c)];
        // A free witness is one of the pieces away from the traps, each on its
        // own square.
        let w = squares().filter(|&s| class(s) == Other).fold(Poly::ZERO, |a, s| a.add(&reach(&[s], 0)));
        let mut h = [[[Poly::ZERO; 4]; 4]; 3];
        for from in 0..4 {
            for to in (0..4).filter(|&to| to != from) {
                let g = reach(&trap_neighbors(from), to);
                h[Member::RabbitGuard as usize][from][to] = g;
                h[Member::OtherGuard as usize][from][to] = g;
                h[Member::Occupant as usize][from][to] = reach(&[TRAPS[from]], to);
            }
        }
        Polys { free: k(Other), w, h }
    }
}

/// A trap's polynomials with some members serving as witnesses elsewhere:
/// `s` (it needs a witness itself) and `f`, everything else plus `s` with a
/// free witness.
#[derive(Clone, Copy)]
struct Group {
    s: Poly,
    f: Poly,
}

fn group(t: usize, u: &TrapUse, serving: &[Member], ps: &Polys) -> Option<Group> {
    let n = |m| serving.iter().filter(|&&x| x == m).count();
    if MEMBERS.iter().any(|&m| n(m) > u.has(m)) {
        return None;
    }
    // Every way to place the guards around the trap, and to pick which of
    // them serve: the coefficientwise maximum.
    let sides = trap_neighbors(t);
    let (mut bf, mut bs) = (Poly::ZERO, Poly::ZERO);
    let guards = u.rabbits + u.others;
    for mask in 0..1u32 << 4 {
        if mask.count_ones() as usize != guards {
            continue;
        }
        let at: Vec<Sq> = (0..4).filter(|&b| mask >> b & 1 == 1).map(|b| sides[b]).collect();
        // Which of the placed guards are rabbits, and which serve.
        for rabbits in 0..1u32 << guards {
            if rabbits.count_ones() as usize != u.rabbits {
                continue;
            }
            for away in 0..1u32 << guards {
                let is_rabbit = |g: usize| rabbits >> g & 1 == 1;
                let count =
                    |r: bool| (0..guards).filter(|&g| away >> g & 1 == 1 && is_rabbit(g) == r).count();
                if count(true) != n(Member::RabbitGuard) || count(false) != n(Member::OtherGuard) {
                    continue;
                }
                let mut members: Vec<Piece> = u
                    .occupant
                    .map(|kind| Piece {
                        at: TRAPS[t],
                        kind,
                        occupant: true,
                        away: n(Member::Occupant) > 0,
                        guarding: false,
                        fixed: None,
                    })
                    .into_iter()
                    .collect();
                for (g, &sq) in at.iter().enumerate() {
                    let kind = if is_rabbit(g) { Kind::Rabbit } else { Kind::Other };
                    members.push(Piece {
                        at: sq,
                        kind,
                        occupant: false,
                        away: away >> g & 1 == 1,
                        guarding: false,
                        fixed: None,
                    });
                }
                let (cn, cs) = cluster(t, &members);
                bf = bf.max(&cn.add(&cs.mul(&ps.w)));
                bs = bs.max(&cs);
            }
        }
    }
    Some(Group { s: bs, f: bf })
}

/// A trap use with its groups for every way up to two members serve as
/// witnesses.
struct Prepared {
    u: TrapUse,
    members: Vec<Member>,
    plain: Group,
    one: Vec<(Member, Group)>,
    two: Vec<((Member, Member), Group)>,
}

impl Prepared {
    fn new(t: usize, u: TrapUse, ps: &Polys) -> Prepared {
        let members = u.members();
        let mut one = Vec::new();
        let mut two = Vec::new();
        for (i, &a) in members.iter().enumerate() {
            one.push((a, group(t, &u, &[a], ps).unwrap()));
            for &b in &members[i..] {
                if let Some(g) = group(t, &u, &[a, b], ps) {
                    two.push(((a, b), g));
                }
            }
        }
        Prepared { u, members, plain: group(t, &u, &[], ps).unwrap(), one, two }
    }
    fn serving(&self, ms: &[Member]) -> Option<&Group> {
        match ms {
            [] => Some(&self.plain),
            [a] => self.one.iter().find(|(m, _)| m == a).map(|(_, g)| g),
            [a, b] => self.two.iter().find(|(m, _)| *m == (*a, *b) || *m == (*b, *a)).map(|(_, g)| g),
            _ => None,
        }
    }
}

/// A witness edge: a member of trap `from`'s group ends next to trap `to`.
#[derive(Clone, Copy)]
struct Edge {
    to: usize,
    from: usize,
    m: Member,
}

/// The joint polynomial of the four traps placed as `t` (indexed like
/// `TRAPS`). Every trap that needs a witness gets one: a free piece (inside
/// `f`) or a member of another trap's group (an edge). Each edge costs at
/// least 2 steps, the trap's guard moving and the witness arriving, so there
/// are at most two.
fn traps_poly(t: [&Prepared; 4], ps: &Polys) -> Poly {
    let mut total = t.iter().fold(Poly::one(), |a, p| a.mul(&p.plain.f));
    let mut edges = Vec::new();
    for to in 0..4 {
        for from in (0..4).filter(|&f| f != to) {
            for &m in &t[from].members {
                let deg = t[to].plain.s.min_degree() + ps.h[m as usize][from][to].min_degree();
                if deg < D {
                    edges.push((Edge { to, from, m }, deg));
                }
            }
        }
    }
    let term = |es: &[Edge]| -> Poly {
        let mut r = Poly::one();
        for (c, p) in t.iter().enumerate() {
            let serving: Vec<Member> = es.iter().filter(|e| e.from == c).map(|e| e.m).collect();
            let Some(g) = p.serving(&serving) else { return Poly::ZERO };
            r = r.mul(if es.iter().any(|e| e.to == c) { &g.s } else { &g.f });
        }
        es.iter().fold(r, |a, e| a.mul(&ps.h[e.m as usize][e.from][e.to]))
    };
    for (i, &(e, d)) in edges.iter().enumerate() {
        total = total.add(&term(&[e]));
        for &(e2, d2) in &edges[i + 1..] {
            if e2.to != e.to && d + d2 < D {
                total = total.add(&term(&[e, e2]));
            }
        }
    }
    total
}

fn trap_uses(occupied: bool) -> Vec<TrapUse> {
    let mut v = Vec::new();
    let occupants: &[Option<Kind>] =
        if occupied { &[None, Some(Kind::Rabbit), Some(Kind::Other)] } else { &[None] };
    for &occupant in occupants {
        for rabbits in 0..=4 {
            for others in 0..=4 - rabbits {
                // No position has an unguarded piece on a trap.
                if occupant.is_none() || rabbits + others > 0 {
                    v.push(TrapUse { occupant, rabbits, others });
                }
            }
        }
    }
    v
}

/// The largest bound for gold pieces, over every way to use the traps, with
/// at most `rabbits` gold rabbits and `others` other gold pieces (fewer
/// pieces never raise the bound).
///
/// Also returns the most that the witness edges between traps add to any
/// configuration (`legal_bound` adds them separately).
fn max_bound(rabbits: usize, others: usize, occupied: bool, ps: &Polys) -> (i64, Vec<TrapUse>, i64) {
    let uses = trap_uses(occupied);
    let prepared: Vec<Vec<Prepared>> =
        (0..4).map(|t| uses.iter().map(|&u| Prepared::new(t, u, ps)).collect()).collect();
    let free: Vec<Vec<Poly>> = (0..=rabbits)
        .map(|r| (0..=others).map(|o| ps.free[0].pow(r).mul(&ps.free[1].pow(o))).collect())
        .collect();
    let n = uses.len();
    let threads = 8.min(n);
    std::thread::scope(|sc| {
        let handles: Vec<_> = (0..threads)
            .map(|th| {
                let (prepared, free) = (&prepared, &free);
                sc.spawn(move || {
                    let mut best = (0, Vec::new(), 0);
                    for a in (th..n).step_by(threads) {
                        for b in 0..n {
                            for c in 0..n {
                                for d in 0..n {
                                    let t =
                                        [&prepared[0][a], &prepared[1][b], &prepared[2][c], &prepared[3][d]];
                                    let (r, o) = t.iter().fold((0, 0), |acc, p| {
                                        let g = p.u.gold();
                                        (acc.0 + g.0, acc.1 + g.1)
                                    });
                                    if r > rabbits || o > others {
                                        continue;
                                    }
                                    let fr = &free[rabbits - r][others - o];
                                    let x = traps_poly(t, ps).mul(fr).total();
                                    let plain = t.iter().fold(Poly::one(), |a, p| a.mul(&p.plain.f));
                                    best.2 = best.2.max(x - plain.mul(fr).total());
                                    if x > best.0 {
                                        best = (x, t.iter().map(|p| p.u).collect(), best.2);
                                    }
                                }
                            }
                        }
                    }
                    best
                })
            })
            .collect();
        let all: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let edges = all.iter().map(|b| b.2).max().unwrap();
        let best = all.into_iter().max_by_key(|b| b.0).unwrap();
        (best.0, best.1, edges)
    })
}

/// End states of a gold piece and a weaker silver piece alone on the board,
/// reached by steps of the gold piece and pushes or pulls of the silver one
/// with at least one push or pull, each at its fewest steps. Either piece
/// landing on a trap may live or die there (other pieces decide). The
/// maximum over placements at distance `d`.
fn pair_poly(d: usize) -> Poly {
    pair_poly_where(d, |_| true, |_, _| true)
}

/// `pair_poly` over the gold piece's starting squares in `start`, counting
/// only end states whose gold piece's ending passes `end(start, ending)`.
fn pair_poly_where(d: usize, start: impl Fn(Sq) -> bool, end: impl Fn(Sq, Option<Sq>) -> bool) -> Poly {
    let mut best = Poly::ZERO;
    for g0 in squares().filter(|&g| start(g)) {
        for s0 in squares().filter(|&s| dist(g0, s) == d) {
            let mut p = Poly::ZERO;
            for (&(g, _, pp), &c) in &pair_states(g0, s0) {
                if pp && end(g0, g) {
                    p.0[c] += 1;
                }
            }
            best = best.max(&p);
        }
    }
    best
}

/// A pair's end states: gold ending, silver ending, whether a push or pull
/// happened; each with its fewest steps.
type PairState = (Option<Sq>, Option<Sq>, bool);

fn pair_states(g0: Sq, s0: Sq) -> std::collections::HashMap<PairState, usize> {
    let on = |q: Sq| (0..8).contains(&q.0) && (0..8).contains(&q.1);
    let nbrs = |q: Sq| {
        [(q.0 + 1, q.1), (q.0 - 1, q.1), (q.0, q.1 + 1), (q.0, q.1 - 1)].into_iter().filter(move |&x| on(x))
    };
    let fates = |q: Sq| if TRAPS.contains(&q) { vec![Some(q), None] } else { vec![Some(q)] };
    let mut cost = std::collections::HashMap::new();
    cost.insert((Some(g0), Some(s0), false), 0);
    for steps in 1..D {
        let mut found: Vec<PairState> = Vec::new();
        for (&(g, s, pp), &c) in &cost {
            let Some(g) = g else { continue };
            if c + 1 == steps {
                for x in nbrs(g).filter(|&x| Some(x) != s) {
                    found.extend(fates(x).into_iter().map(|gf| (gf, s, pp)));
                }
            }
            let Some(s) = s else { continue };
            if c + 2 == steps && dist(g, s) == 1 {
                // Push: the silver piece steps away, the gold one follows.
                for y in nbrs(s).filter(|&y| y != g) {
                    for sf in fates(y) {
                        found.extend(fates(s).into_iter().map(|gf| (gf, sf, true)));
                    }
                }
                // Pull: the gold piece steps away, the silver one follows.
                for x in nbrs(g).filter(|&x| x != s) {
                    for gf in fates(x) {
                        found.extend(fates(g).into_iter().map(|sf| (gf, sf, true)));
                    }
                }
            }
        }
        for st in found {
            cost.entry(st).or_insert(steps);
        }
    }
    cost
}

/// Every way to place a use's guards around trap `t`, with their kinds.
fn placements(t: usize, u: &TrapUse) -> Vec<Vec<Piece>> {
    let sides = trap_neighbors(t);
    let guards = u.rabbits + u.others;
    let mut v = Vec::new();
    for mask in (0..1u32 << 4).filter(|m| m.count_ones() as usize == guards) {
        let at: Vec<Sq> = (0..4).filter(|&b| mask >> b & 1 == 1).map(|b| sides[b]).collect();
        for rabbits in (0..1u32 << guards).filter(|r| r.count_ones() as usize == u.rabbits) {
            let mut members: Vec<Piece> = u
                .occupant
                .map(|kind| Piece {
                    at: TRAPS[t],
                    kind,
                    occupant: true,
                    away: false,
                    guarding: false,
                    fixed: None,
                })
                .into_iter()
                .collect();
            for (g, &sq) in at.iter().enumerate() {
                let kind = if rabbits >> g & 1 == 1 { Kind::Rabbit } else { Kind::Other };
                members.push(Piece {
                    at: sq,
                    kind,
                    occupant: false,
                    away: false,
                    guarding: false,
                    fixed: None,
                });
            }
            v.push(members);
        }
    }
    v
}

/// How close (as a factor) polynomials in the legal search must be to merge.
const MERGE: f64 = 0.01;

/// Far off the board: where a member fixed "away" ends.
const AWAY: Sq = (-9, -9);

/// The silver-related polynomials of one trap use, maximized over the four
/// trap positions and the guards' placements.
///
/// A pushing member (a non-rabbit) with a silver piece d squares away loses
/// its endings on that square: at least the ending with the group-mates in
/// a state counted in `lost` (a lower bound of 1) and at most `l_hi[d]`. Its
/// pushes and pulls of that piece go with group-mates' states among the
/// same ones (`legal_bound` explains why).
#[derive(Clone, Copy)]
struct UseSilver {
    u: TrapUse,
    f: Poly,
    l_hi: [Poly; 4],
    /// The most incidences at each distance.
    k: [usize; 4],
    /// Group-mates ending on the pushed piece's vacated square.
    at_s: [Poly; 4],
    /// Silver on an empty trap: a guard's endings on it, with the
    /// group-mates' states, and those of them needing a witness.
    l_trap: Poly,
    s_trap: Poly,
    k_trap: usize,
    /// States needing a witness (unweighted), and states with a member
    /// fixed on another square.
    s: Poly,
    fixed: Poly,
}

#[allow(clippy::needless_range_loop)] // trap and member indices with their squares
fn use_silver(u: TrapUse, ps: &Polys) -> UseSilver {
    let z = Poly::ZERO;
    let mut x = UseSilver {
        u,
        f: z,
        l_hi: [z; 4],
        k: [0; 4],
        at_s: [z; 4],
        l_trap: z,
        s_trap: z,
        k_trap: 0,
        s: z,
        fixed: z,
    };
    let weigh = |(n, s): (Poly, Poly)| n.add(&s.mul(&ps.w));
    for t in 0..4 {
        x.f = x.f.max(&Prepared::new(t, u, ps).plain.f);
        let trap = TRAPS[t];
        for members in placements(t, &u) {
            x.s = x.s.max(&cluster(t, &members).1);
            let mut k = [0; 4];
            let mut k_trap = 0;
            for i in 0..members.len() {
                let fix = |i: usize, q: Sq| {
                    let mut ms = members.clone();
                    ms[i].fixed = Some(q);
                    ms
                };
                // A non-rabbit member fixed on a square within 3: it may end
                // on another push's vacated square that is blocked for it.
                for q in squares()
                    .filter(|&q| members[i].kind == Kind::Other && (1..4).contains(&dist(q, members[i].at)))
                {
                    if !members.iter().any(|m| m.at == q) {
                        x.fixed = x.fixed.max(&weigh(cluster(t, &fix(i, q))));
                    }
                }
                if members[i].kind == Kind::Rabbit {
                    continue;
                }
                for q in squares() {
                    let d = dist(q, members[i].at);
                    if !(1..4).contains(&d) || members.iter().any(|m| m.at == q) {
                        continue;
                    }
                    if q == trap {
                        // Silver on an empty trap next to this guard.
                        let (n, s) = cluster(t, &fix(i, q));
                        x.l_trap = x.l_trap.max(&n.add(&s.mul(&ps.w)));
                        x.s_trap = x.s_trap.max(&s);
                        k_trap += 1;
                        continue;
                    }
                    k[d] += 1;
                    x.l_hi[d] = x.l_hi[d].max(&weigh(cluster(t, &fix(i, q))));
                    // A group-mate ending on q after the silver piece leaves,
                    // with the pusher anywhere.
                    let mut at = Poly::ZERO;
                    for j in (0..members.len()).filter(|&j| j != i) {
                        let dj = dist(members[j].at, q);
                        if dj < D && !(members[j].kind == Kind::Rabbit && q.1 < members[j].at.1) {
                            let mut ms = fix(i, AWAY);
                            ms[j].fixed = Some(q);
                            at = at.add(&Poly::term(dj, 1).mul(&weigh(cluster(t, &ms))));
                        }
                    }
                    x.at_s[d] = x.at_s[d].max(&at);
                }
            }
            for d in 1..4 {
                x.k[d] = x.k[d].max(k[d]);
            }
            x.k_trap = x.k_trap.max(k_trap);
        }
    }
    x
}

/// What every push or pull costs or adds, independent of the position.
struct SilverModel {
    /// The pair's joint end states at distance d; those with the gold piece
    /// next to any trap; those with a trap group's member next to its own.
    j: [Poly; 4],
    j_adj: [Poly; 4],
    j_own: [Poly; 4],
    /// Unweighted states needing a witness, over all uses.
    s_max: Poly,
    /// Witnesses from other traps' groups.
    h: Poly,
    /// A pushing member fixed elsewhere: the most extra group states.
    extra: Poly,
    /// Another gold piece ending on the vacated square.
    ext: Poly,
    /// Second pushes or pulls.
    second: Poly,
    /// Silver pieces on traps whose fate depends on order.
    order: i64,
}

impl SilverModel {
    #[allow(clippy::needless_range_loop)] // trap indices with their squares
    fn new(uses: &[UseSilver]) -> SilverModel {
        let next_to_trap = |x: Sq| TRAPS.iter().any(|&t| dist(t, x) == 1);
        let own_trap = |g0: Sq| *TRAPS.iter().find(|&&t| dist(t, g0) <= 1).unwrap();
        let mut j = [Poly::ZERO; 4];
        let mut j_adj = [Poly::ZERO; 4];
        let mut j_own = [Poly::ZERO; 4];
        for d in 1..4 {
            j[d] = pair_poly(d);
            j_adj[d] = pair_poly_where(d, |_| true, |_, g| g.is_some_and(next_to_trap));
            // Only a guard's group-mates can rely on it (an occupant's cannot:
            // with it off the trap, the trap is empty or theirs).
            j_own[d] = pair_poly_where(
                d,
                |g| class(g) == Class::NextToTrap,
                |g0, g| g.is_some_and(|g| dist(g, own_trap(g0)) <= 1),
            );
        }
        let s_max = uses.iter().fold(Poly::ZERO, |a, x| a.max(&x.s));
        // Witnesses from other traps' groups: any member's ending next to trap 0.
        let mut h = Poly::ZERO;
        for from in 1..4 {
            for q in trap_neighbors(from).into_iter().chain([TRAPS[from]]) {
                h = h.add(&reach(&[q], 0));
            }
        }
        // Another gold piece may end on the pushed piece's vacated square: at
        // most 3, 8 and 12 at distance 1 to 3 (with a push the pusher is there;
        // with a pull the silver piece is on one neighbor). Only pieces that
        // can push it are blocked there; if one belongs to a trap group, the
        // group's states with it fixed there exceed the group's own by at most
        // `extra`.
        let extra = uses.iter().fold(Poly::ZERO, |a, x| a.max(&x.fixed.sub(&x.f).clamp()));
        let ext = Poly::one().add(&Poly([0, 3, 8, 12, 0]).mul(&Poly::one().add(&extra)));
        // After a first push or pull at distance 1 with nothing else moving, a
        // second by G on another silver piece next to it or by another gold
        // piece on S (three of each); with an occupant, alive or not.
        let second = Poly::term(4, j[1].0[2] * 2 * 3 * j[1].0[2]);
        // A silver piece on a trap whose guard leaves while another silver
        // piece arrives, in either order: with a of the trap's four neighbors
        // silver guards, their departures by push or pull (3 gold pieces
        // each, 6 ways) against arrivals on the other 4 - a (18 ways each):
        // 324 a (4 - a) <= 1296 on each trap.
        let order = 4 * 1296;
        SilverModel { j, j_adj, j_own, s_max, h, extra, ext, second, order }
    }

    /// An incidence's gain per state of L, net of its lost ending. `own`: the
    /// pusher's group-mates may rely on it next to their trap (an occupant,
    /// or another guard that may end on the trap).
    fn net(&self, d: usize, own: bool, occupied: bool) -> Poly {
        let jj = if own { self.j[d].add(&self.j_own[d]) } else { self.j[d] };
        let mut g = jj
            .mul(&self.ext)
            .add(&self.j_adj[d].mul(&self.s_max))
            .add(&self.j[d].mul(&self.s_max).mul(&self.h).mul(&Poly::one().add(&self.extra)));
        if d == 1 {
            g = g.add(&self.second.scale(if occupied { 2 } else { 1 }));
        }
        g.sub(&Poly::term(d, 1))
    }
}

/// Pareto frontier: drops polynomials coefficientwise at most another.
fn pareto(mut v: Vec<Poly>) -> Vec<Poly> {
    v.sort_by_key(|p| std::cmp::Reverse(p.0));
    v.dedup();
    let mut out: Vec<Poly> = Vec::new();
    for p in v {
        if !out.iter().any(|q| (0..D).all(|k| q.0[k] >= p.0[k])) {
            out.push(p);
        }
    }
    out
}

/// Merges polynomials whose coefficients agree within a factor of
/// `1 + tol` into their coefficientwise maximum (an upper bound for each),
/// then keeps the Pareto frontier.
fn merge(v: Vec<Poly>, tol: f64) -> Vec<Poly> {
    use std::collections::HashMap;
    let step = (1.0 + tol).ln();
    let mut buckets: HashMap<[i64; D], Poly> = HashMap::new();
    for p in v {
        let key = p.0.map(|c| if c <= 0 { -1 } else { ((c as f64).ln() / step) as i64 });
        buckets.entry(key).and_modify(|q| *q = q.max(&p)).or_insert(p);
    }
    pareto(buckets.into_values().collect())
}

/// The corners of {0 <= x_i <= ub_i, sum x_i <= cap}: some at their bound,
/// at most one more taking what is left, the rest 0.
fn capped_corners(ub: &[i64], cap: i64) -> Vec<Vec<i64>> {
    let n = ub.len();
    let mut out = Vec::new();
    for full in 0..1u32 << n {
        let used: i64 = (0..n).filter(|&i| full >> i & 1 == 1).map(|i| ub[i]).sum();
        if used > cap {
            continue;
        }
        let base: Vec<i64> = (0..n).map(|i| if full >> i & 1 == 1 { ub[i] } else { 0 }).collect();
        out.push(base.clone());
        for i in (0..n).filter(|&i| full >> i & 1 == 0) {
            if ub[i] > cap - used && cap > used {
                let mut v = base.clone();
                v[i] = cap - used;
                out.push(v);
            }
        }
    }
    out
}

/// An upper bound on unique moves over all legal positions.
///
/// Silver pieces move only in pushes and pulls by stronger gold pieces. Say a
/// non-rabbit gold piece G and a silver piece S d ≤ 3 squares away form an
/// incidence. Every board is either reached without a push or pull, or by
/// turns whose first push or pull is some incidence's G on S.
///
/// Boards of the first kind never have G on S's square, so for each
/// incidence the gold-only model's objects with G ending there are not
/// needed: G's ending there (cost d) times its group-mates' states L (and
/// the rest of the board). Taking only states with no group-mate on another
/// incidence's square keeps these sets disjoint.
///
/// Boards of the second kind: G and S end in one of the pair's joint end
/// states (`pair_poly`). If G ends away from its own trap, the group-mates'
/// state goes with G on S's square too, so it is among L; if G ends next to
/// its trap, the group-mates may rely on it, and removing the trap's
/// occupant instead gives a state in L (so those count twice). Another gold
/// piece may end on S's vacated square (`ext`, at most 4, 8 and 12 pieces at
/// distance 1 to 3, their groups' states at most `kappa` times as many), G
/// may end next to another trap as its witness, another trap's member may
/// be one, and up to two more pushes or pulls can follow a first one at
/// distance 1. The rest of the board is as in the gold-only model.
///
/// So each incidence adds (gains - t^d) * L with L between 1 and its upper
/// bound, plus terms that do not depend on L. Everything is a product over
/// free pieces and trap groups, linear in each L and each number of
/// incidences, so its maximum is at corners of those ranges: the search
/// tries them all.
#[allow(clippy::needless_range_loop)] // counts index the tables
fn legal_bound(ps: &Polys, edges: i64) -> i64 {
    let uses: Vec<UseSilver> = trap_uses(true).into_iter().map(|u| use_silver(u, ps)).collect();
    let model = SilverModel::new(&uses);
    let j = model.j;
    let order = model.order;
    let net = |d: usize, own: bool, occupied: bool| model.net(d, own, occupied);
    eprintln!("s_max {:?} h {:?} extra {:?} ext {:?}", model.s_max.0, model.h.0, model.extra.0, model.ext.0);
    for d in 1..4 {
        eprintln!("net free d={d} {:?} cluster {:?}", net(d, false, false).0, net(d, true, true).0);
    }

    // Options for a free other piece: no silver, or the most at a distance.
    let other = ps.free[1];
    let mut free_opts = Vec::new();
    for m in 0..8u32 {
        let mut p = other;
        for (b, (d, n)) in [(1, 4), (2, 8), (3, 12)].into_iter().enumerate() {
            if m >> b & 1 == 1 {
                p = p.add(&net(d, false, false).scale(n));
            }
        }
        free_opts.push(p.clamp());
    }
    let free_opts = pareto(free_opts);
    // Options for each trap use. The lost endings are distinct states of
    // the group, so at each number of steps they total at most the group's
    // count there: the amounts lie in a polytope, and the corners suffice.
    let mut use_opts: Vec<(TrapUse, Vec<Poly>)> = Vec::new();
    for x in &uses {
        // (distance, coefficient of L, upper bound, net per unit)
        let mut vars: Vec<(usize, usize, i64, Poly)> = Vec::new();
        let own = x.u.occupant.is_some() || x.u.rabbits + x.u.others >= 2;
        let mut abs = Poly::ZERO;
        for d in 1..4 {
            if x.k[d] > 0 {
                for jj in 0..4 - d {
                    vars.push((d, jj, x.k[d] as i64 * x.l_hi[d].0[jj], net(d, own, x.u.occupant.is_some())));
                }
                abs = abs.add(&j[d].mul(&x.at_s[d]).scale(x.k[d] as i64));
            }
        }
        if x.k_trap > 0 {
            for jj in 0..3 {
                vars.push((1, jj, x.k_trap as i64 * x.l_trap.0[jj], net(1, own, x.u.occupant.is_some())));
            }
            abs = abs.add(&j[1].mul(&x.s_trap).scale(x.k_trap as i64));
        }
        let mut opts = vec![x.f];
        if !vars.is_empty() {
            // Each step count's corners, then every combination.
            let mut partial = vec![x.f.add(&abs)];
            for deg in 1..4 {
                let group: Vec<&(usize, usize, i64, Poly)> =
                    vars.iter().filter(|v| v.0 + v.1 == deg).collect();
                let ub: Vec<i64> = group.iter().map(|v| v.2.max(0)).collect();
                let mut next = Vec::new();
                for amounts in capped_corners(&ub, x.f.0[deg]) {
                    let add = group
                        .iter()
                        .zip(&amounts)
                        .fold(Poly::ZERO, |acc, (v, &a)| acc.add(&v.3.mul(&Poly::term(v.1, a))));
                    next.extend(partial.iter().map(|p| p.add(&add)));
                }
                partial = merge(next.into_iter().map(|p| p.clamp()).collect(), MERGE);
            }
            opts.extend(partial);
        }
        let opts = pareto(opts.into_iter().map(|p| p.clamp()).collect());
        use_opts.push((x.u, opts));
    }
    assert!(use_opts.len() < 128 && use_opts.iter().all(|(_, o)| o.len() < 512), "labels need more bits");
    eprintln!(
        "options: free {}, traps {} (largest {})",
        free_opts.len(),
        use_opts.iter().map(|(_, o)| o.len()).sum::<usize>(),
        use_opts.iter().map(|(_, o)| o.len()).max().unwrap()
    );
    // The traps: frontier by gold used.
    // Free other pieces: frontier by count.
    let mut fr: Vec<Vec<Poly>> = vec![vec![Poly::one()]];
    for _ in 0..8 {
        let last = fr.last().unwrap();
        let next: Vec<Poly> = last.iter().flat_map(|a| free_opts.iter().map(move |p| a.mul(p))).collect();
        fr.push(merge(next, MERGE));
    }
    eprintln!("free frontier {}", fr[8].len());
    let rabbit = ps.free[0];
    let rabbits: Vec<Poly> = (0..9).map(|k| rabbit.pow(k)).collect();
    // A value the maximum reaches: the best configuration without silver.
    let mut lb = 0;
    {
        let mut plain: Vec<Vec<Vec<Poly>>> = vec![vec![Vec::new(); 9]; 9];
        plain[0][0].push(Poly::one());
        for _ in 0..4 {
            let mut next: Vec<Vec<Vec<Poly>>> = vec![vec![Vec::new(); 9]; 9];
            for r in 0..9 {
                for o in 0..9 {
                    for a in &plain[r][o] {
                        for x in &uses {
                            let (ur, uo) = x.u.gold();
                            if r + ur <= 8 && o + uo <= 8 {
                                next[r + ur][o + uo].push(a.mul(&x.f));
                            }
                        }
                    }
                }
            }
            for row in &mut next {
                for v in row.iter_mut() {
                    *v = pareto(std::mem::take(v));
                }
            }
            plain = next;
        }
        for r in 0..9 {
            for o in 0..9 {
                for a in &plain[r][o] {
                    lb = lb.max(a.mul(&rabbits[8 - r]).mul(&other.pow(8 - o)).total());
                }
            }
        }
    }
    eprintln!("without silver {lb}");
    // Upper bounds on what the remaining traps, free pieces and rabbits can
    // multiply a partial configuration by: coefficientwise maxima.
    let cap_free: Vec<Poly> = fr.iter().map(|v| v.iter().fold(Poly::ZERO, |a, p| a.max(p))).collect();
    let cap_opt: Vec<Poly> =
        use_opts.iter().map(|(_, o)| o.iter().fold(Poly::ZERO, |a, p| a.max(p))).collect();
    let mut cap_trap: Vec<Vec<Vec<Option<Poly>>>> = vec![vec![vec![None; 9]; 9]];
    cap_trap[0][0][0] = Some(Poly::one());
    for m in 1..=4 {
        let mut c: Vec<Vec<Option<Poly>>> = vec![vec![None; 9]; 9];
        for r in 0..9 {
            for o in 0..9 {
                let Some(a) = cap_trap[m - 1][r][o] else { continue };
                for (ui, (u, _)) in use_opts.iter().enumerate() {
                    let (ur, uo) = u.gold();
                    if r + ur <= 8 && o + uo <= 8 {
                        let v = a.mul(&cap_opt[ui]);
                        let e = &mut c[r + ur][o + uo];
                        *e = Some(e.map_or(v, |e| e.max(&v)));
                    }
                }
            }
        }
        cap_trap.push(c);
    }
    // rest[k][r][o]: the most a configuration with k traps placed, using r
    // rabbits and o others, can still be multiplied by.
    let mut rest = vec![vec![vec![Poly::ZERO; 9]; 9]; 5];
    for (k, rk) in rest.iter_mut().enumerate() {
        for r in 0..9 {
            for o in 0..9 {
                for r2 in 0..9 - r {
                    for o2 in 0..9 - o {
                        if let Some(c) = cap_trap[4 - k][r2][o2] {
                            let v = c.mul(&cap_free[8 - o - o2]).mul(&rabbits[8 - r - r2]);
                            rk[r][o] = rk[r][o].max(&v);
                        }
                    }
                }
            }
        }
    }
    // Each entry keeps the options of one configuration it bounds (for
    // reporting only).
    // The options are packed 16 bits per trap: use index, then option index.
    type Entry = (Poly, u64);
    let merge_e = |v: Vec<Entry>| -> Vec<Entry> {
        use std::collections::HashMap;
        let step = (1.0 + MERGE).ln();
        let mut buckets: HashMap<[i64; D], Entry> = HashMap::new();
        for (p, l) in v {
            let key = p.0.map(|c| if c <= 0 { -1 } else { ((c as f64).ln() / step) as i64 });
            buckets
                .entry(key)
                .and_modify(|q| {
                    let keep = if p.total() > q.0.total() { l } else { q.1 };
                    *q = (q.0.max(&p), keep);
                })
                .or_insert((p, l));
        }
        // Keep the Pareto frontier.
        let v: Vec<Entry> = buckets.into_values().collect();
        let front = pareto(v.iter().map(|e| e.0).collect());
        let front: std::collections::HashSet<[i64; D]> = front.into_iter().map(|p| p.0).collect();
        v.into_iter().filter(|e| front.contains(&e.0.0)).collect()
    };
    let mut st: Vec<Vec<Vec<Entry>>> = vec![vec![Vec::new(); 9]; 9];
    st[0][0].push((Poly::one(), 0));
    for slot in 0..4 {
        let items: Vec<(usize, usize, &Entry)> = (0..9)
            .flat_map(|r| (0..9).map(move |o| (r, o)))
            .flat_map(|(r, o)| st[r][o].iter().map(move |e| (r, o, e)))
            .collect();
        let threads = 8;
        let locals: Vec<Vec<Vec<Vec<Entry>>>> = std::thread::scope(|sc| {
            let hs: Vec<_> = (0..threads)
                .map(|th| {
                    let (items, use_opts, merge_e, rest) = (&items, &use_opts, &merge_e, &rest);
                    sc.spawn(move || {
                        let mut next: Vec<Vec<Vec<Entry>>> = vec![vec![Vec::new(); 9]; 9];
                        for &(r, o, (a, la)) in items.iter().skip(th).step_by(threads) {
                            for (ui, (u, opts)) in use_opts.iter().enumerate() {
                                let (ur, uo) = u.gold();
                                if r + ur > 8 || o + uo > 8 {
                                    continue;
                                }
                                let target = &mut next[r + ur][o + uo];
                                let room = &rest[slot + 1][r + ur][o + uo];
                                for (oi, p) in opts.iter().enumerate() {
                                    let v = a.mul(p);
                                    // No completion can reach the value without silver.
                                    if v.mul(room).total() < lb {
                                        continue;
                                    }
                                    let l = la << 16 | (ui as u64) << 9 | oi as u64;
                                    target.push((v, l));
                                }
                                // Merge as it grows, so the lists stay small.
                                if target.len() > 100_000 {
                                    *target = merge_e(std::mem::take(target));
                                }
                            }
                        }
                        for row in &mut next {
                            for v in row.iter_mut() {
                                *v = merge_e(std::mem::take(v));
                            }
                        }
                        next
                    })
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        let mut next: Vec<Vec<Vec<Entry>>> = vec![vec![Vec::new(); 9]; 9];
        let _ = slot;
        for local in locals {
            for (r, row) in local.into_iter().enumerate() {
                for (o, v) in row.into_iter().enumerate() {
                    next[r][o].extend(v);
                }
            }
        }
        let cells: Vec<std::sync::Mutex<Vec<Entry>>> =
            next.into_iter().flatten().map(std::sync::Mutex::new).collect();
        std::thread::scope(|sc| {
            for th in 0..threads {
                let (cells, merge_e) = (&cells, &merge_e);
                sc.spawn(move || {
                    for c in cells.iter().skip(th).step_by(threads) {
                        let mut v = c.lock().unwrap();
                        *v = merge_e(std::mem::take(&mut *v));
                    }
                });
            }
        });
        let mut it = cells.into_iter().map(|c| c.into_inner().unwrap());
        st = (0..9).map(|_| (0..9).map(|_| it.next().unwrap()).collect()).collect();
        eprintln!("trap frontier {}", st.iter().flatten().map(|v| v.len()).sum::<usize>());
    }
    let mut best = 0;
    let mut arg = (0, 0, Poly::ZERO, Poly::ZERO);
    let mut arg_l = 0u64;
    for r in 0..9 {
        for o in 0..9 {
            for (a, la) in &st[r][o] {
                let ar = a.mul(&rabbits[8 - r]);
                for f in &fr[8 - o] {
                    let v = ar.mul(f).total();
                    if v > best {
                        best = v;
                        arg = (r, o, *a, *f);
                        arg_l = *la;
                    }
                }
            }
        }
    }
    eprintln!("best: traps use {} rabbits {} others; traps {:?} free {:?}", arg.0, arg.1, arg.2.0, arg.3.0);
    eprintln!("free options {:?}", free_opts.iter().map(|p| p.0).collect::<Vec<_>>());
    for k in (0..4).rev() {
        let code = arg_l >> (16 * k) & 0xffff;
        let (ui, oi) = ((code >> 9) as usize, (code & 0x1ff) as usize);
        let (u, opts) = &use_opts[ui];
        eprintln!("  {u:?}: option {:?} (no silver {:?})", opts[oi].0, uses[ui].f.0);
    }
    eprintln!("silver-free free part {:?}", other.pow(8 - arg.1).0);
    best + edges + order
}

fn main() {
    let ps = Polys::new();
    for (name, k) in [("gold rabbit", 0), ("gold other", 1)] {
        println!("{name:12} elsewhere     {:?}", ps.free[k].0);
    }
    println!("free witness               {:?}", ps.w.0);
    // Usage: bound [empty-traps | legal]
    let arg = std::env::args().nth(1);
    let occupied = arg.as_deref() != Some("empty-traps");
    let (b, t, edges) = max_bound(8, 8, occupied, &ps);
    let traps = if occupied { "" } else { ", traps empty" };
    println!("gold army{traps}: {b}");
    for (sq, u) in ["c3", "f3", "c6", "f6"].iter().zip(t) {
        println!("    {sq}: {u:?}");
    }
    if arg.as_deref() == Some("legal") {
        println!("witness edges add at most {edges}");
        println!("legal positions: {}", legal_bound(&ps, edges));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use manymoves::{Board, Counter, GOLD};

    /// Checks that the model counts at least as many boards as the position
    /// has within each number of steps, returning the tightest ratio.
    fn check(counter: &mut Counter, b: &Board, model: &Poly) -> f64 {
        let mut worst = f64::MAX;
        let mut cum = 0;
        for k in 1..D {
            cum += model.0[k];
            let real = counter.count_steps(b, k as u32) as i64;
            assert!(real <= cum, "{k} steps: real {real} > model {cum}\n{}", b.diagram());
            if real > 0 {
                worst = worst.min(cum as f64 / real as f64);
            }
        }
        worst
    }

    /// The model for one actual position: a group on trap `t`, free pieces
    /// elsewhere, the other traps empty. It must never count fewer end
    /// boards than there are.
    #[test]
    fn model_covers_real_positions() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut rnd = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n) as usize
        };
        let mut counter = Counter::default();
        let free_squares: Vec<Sq> = squares().filter(|&s| class(s) == Class::Other).collect();
        let mut worst = f64::MAX;
        for _ in 0..3000 {
            let t = rnd(4);
            let kind = |r: usize| if r == 0 { Kind::Rabbit } else { Kind::Other };
            let mut members = Vec::new();
            let occupant = rnd(3);
            if occupant > 0 {
                members.push(Piece {
                    at: TRAPS[t],
                    kind: kind(occupant - 1),
                    occupant: true,
                    away: false,
                    guarding: false,
                    fixed: None,
                });
            }
            for sq in trap_neighbors(t) {
                if rnd(2) == 1 {
                    members.push(Piece {
                        at: sq,
                        kind: kind(rnd(2)),
                        occupant: false,
                        away: false,
                        guarding: false,
                        fixed: None,
                    });
                }
            }
            if occupant > 0 && members.len() == 1 {
                continue;
            }
            let mut free: Vec<(Sq, Kind)> = Vec::new();
            for _ in 0..rnd(6) {
                let sq = free_squares[rnd(free_squares.len() as u64)];
                if !free.iter().any(|&(q, _)| q == sq) {
                    free.push((sq, kind(rnd(2))));
                }
            }
            let mut b = Board::empty();
            let mut put = |sq: Sq, k: Kind, i: usize| {
                let kk = if k == Kind::Rabbit { 0 } else { 1 + i % 5 };
                b.put(GOLD, kk, (sq.1 * 8 + sq.0) as usize);
            };
            for (i, m) in members.iter().enumerate() {
                put(m.at, m.kind, i);
            }
            for (i, &(sq, k)) in free.iter().enumerate() {
                put(sq, k, i + 3);
            }
            let (n, s) = cluster(t, &members);
            let w = free.iter().fold(Poly::ZERO, |a, &(sq, _)| a.add(&reach(&[sq], t)));
            let model = free.iter().fold(n.add(&s.mul(&w)), |a, &(sq, k)| a.mul(&square_poly(k, sq)));
            worst = worst.min(check(&mut counter, &b, &model));
        }
        println!("tightest model/real ratio {worst:.3}");
    }

    /// Prepared groups for the actual members of one trap's group.
    fn prepared_from(t: usize, members: &[Piece], w: &Poly) -> Prepared {
        let member = |p: &Piece| match (p.occupant, p.kind) {
            (true, _) => Member::Occupant,
            (false, Kind::Rabbit) => Member::RabbitGuard,
            (false, Kind::Other) => Member::OtherGuard,
        };
        let grp = |away: &[usize]| {
            let ms: Vec<Piece> =
                members.iter().enumerate().map(|(i, p)| Piece { away: away.contains(&i), ..*p }).collect();
            let (n, s) = cluster(t, &ms);
            Group { s, f: n.add(&s.mul(w)) }
        };
        let max = |a: Option<Group>, b: Group| match a {
            None => Some(b),
            Some(a) => Some(Group { s: a.s.max(&b.s), f: a.f.max(&b.f) }),
        };
        let kinds: Vec<Member> =
            MEMBERS.into_iter().filter(|&m| members.iter().any(|p| member(p) == m)).collect();
        let mut one = Vec::new();
        let mut two = Vec::new();
        for (x, &a) in kinds.iter().enumerate() {
            let mut g1 = None;
            for i in (0..members.len()).filter(|&i| member(&members[i]) == a) {
                g1 = max(g1, grp(&[i]));
            }
            one.push((a, g1.unwrap()));
            for &b in &kinds[x..] {
                let mut g2 = None;
                for i in (0..members.len()).filter(|&i| member(&members[i]) == a) {
                    for j in (0..members.len()).filter(|&j| j != i && member(&members[j]) == b) {
                        g2 = max(g2, grp(&[i, j]));
                    }
                }
                if let Some(g) = g2 {
                    two.push(((a, b), g));
                }
            }
        }
        let count = |m| members.iter().filter(|p| member(p) == m).count();
        let u = TrapUse {
            occupant: members.iter().find(|p| p.occupant).map(|p| p.kind),
            rabbits: count(Member::RabbitGuard),
            others: count(Member::OtherGuard),
        };
        Prepared { u, members: kinds, plain: grp(&[]), one, two }
    }

    /// The same check with groups on every trap, so witnesses can come from
    /// other traps' groups.
    #[test]
    fn model_covers_positions_with_several_groups() {
        let ps = Polys::new();
        let mut seed = 0x9e37_79b9_7f4a_7c15u64;
        let mut rnd = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n) as usize
        };
        let mut counter = Counter::default();
        let free_squares: Vec<Sq> = squares().filter(|&s| class(s) == Class::Other).collect();
        let kind = |r: usize| if r == 0 { Kind::Rabbit } else { Kind::Other };
        let mut worst = f64::MAX;
        let mut tested = 0;
        while tested < 1500 {
            let mut groups: Vec<Vec<Piece>> = Vec::new();
            let mut ok = true;
            for (t, &trap) in TRAPS.iter().enumerate() {
                let mut members = Vec::new();
                let occupant = rnd(4).saturating_sub(1);
                if occupant > 0 {
                    members.push(Piece {
                        at: trap,
                        kind: kind(occupant - 1),
                        occupant: true,
                        away: false,
                        guarding: false,
                        fixed: None,
                    });
                }
                for sq in trap_neighbors(t) {
                    if rnd(3) == 0 {
                        members.push(Piece {
                            at: sq,
                            kind: kind(rnd(2)),
                            occupant: false,
                            away: false,
                            guarding: false,
                            fixed: None,
                        });
                    }
                }
                ok &= !(occupant > 0 && members.len() == 1);
                groups.push(members);
            }
            if !ok {
                continue;
            }
            let mut free: Vec<(Sq, Kind)> = Vec::new();
            for _ in 0..rnd(5) {
                let sq = free_squares[rnd(free_squares.len() as u64)];
                if !free.iter().any(|&(q, _)| q == sq) {
                    free.push((sq, kind(rnd(2))));
                }
            }
            let mut b = Board::empty();
            let all: Vec<(Sq, Kind)> =
                groups.iter().flatten().map(|p| (p.at, p.kind)).chain(free.iter().copied()).collect();
            if all.len() > 16 {
                continue;
            }
            for (i, &(sq, k)) in all.iter().enumerate() {
                let kk = if k == Kind::Rabbit { 0 } else { 1 + i % 5 };
                b.put(GOLD, kk, (sq.1 * 8 + sq.0) as usize);
            }
            let prepared: Vec<Prepared> = (0..4)
                .map(|t| {
                    let w = free.iter().fold(Poly::ZERO, |a, &(sq, _)| a.add(&reach(&[sq], t)));
                    prepared_from(t, &groups[t], &w)
                })
                .collect();
            let t4 = [&prepared[0], &prepared[1], &prepared[2], &prepared[3]];
            let model = free.iter().fold(traps_poly(t4, &ps), |a, &(sq, k)| a.mul(&square_poly(k, sq)));
            worst = worst.min(check(&mut counter, &b, &model));
            tested += 1;
        }
        println!("tightest model/real ratio {worst:.3}");
    }

    /// A trap whose only guard leaves while a guard of the next trap steps
    /// across to replace it: only a witness edge counts that.
    #[test]
    fn witness_from_next_trap() {
        let ps = Polys::new();
        let mut counter = Counter::default();
        let groups = [
            vec![
                Piece {
                    at: (2, 2),
                    kind: Kind::Rabbit,
                    occupant: true,
                    away: false,
                    guarding: false,
                    fixed: None,
                },
                Piece {
                    at: (2, 1),
                    kind: Kind::Other,
                    occupant: false,
                    away: false,
                    guarding: false,
                    fixed: None,
                },
            ],
            vec![
                Piece {
                    at: (5, 2),
                    kind: Kind::Rabbit,
                    occupant: true,
                    away: false,
                    guarding: false,
                    fixed: None,
                },
                Piece {
                    at: (4, 2),
                    kind: Kind::Other,
                    occupant: false,
                    away: false,
                    guarding: false,
                    fixed: None,
                },
            ],
            vec![],
            vec![],
        ];
        let mut b = Board::empty();
        for (i, p) in groups.iter().flatten().enumerate() {
            let kk = if p.kind == Kind::Rabbit { 0 } else { 1 + i % 5 };
            b.put(GOLD, kk, (p.at.1 * 8 + p.at.0) as usize);
        }
        let prepared: Vec<Prepared> = (0..4).map(|t| prepared_from(t, &groups[t], &Poly::ZERO)).collect();
        let model = traps_poly([&prepared[0], &prepared[1], &prepared[2], &prepared[3]], &ps);
        println!("model {:?}", model.0);
        check(&mut counter, &b, &model);
    }

    /// The legal model for one actual position: silver pieces (weaker than
    /// every non-rabbit gold piece, none on traps) block the endings of the
    /// gold pieces that can push them, and add their pushes and pulls.
    fn silver_model(
        ps: &Polys,
        model: &SilverModel,
        groups: &[Vec<Piece>],
        free: &[(Sq, Kind)],
        silver: &[Sq],
    ) -> Poly {
        let weigh = |(n, s): (Poly, Poly)| n.add(&s.mul(&ps.w));
        let mut units = Poly::one();
        let mut free_plain = Poly::one();
        for &(sq, k) in free {
            let mut p = square_poly(k, sq);
            free_plain = free_plain.mul(&p);
            if k == Kind::Other {
                for &x in silver {
                    let d = dist(sq, x);
                    if (1..4).contains(&d) {
                        p = p.add(&model.net(d, false, false));
                    }
                }
            }
            units = units.mul(&p.clamp());
        }
        let prepared: Vec<Prepared> = (0..4).map(|t| prepared_from(t, &groups[t], &ps.w)).collect();
        let mut plain = Poly::one();
        for (t, members) in groups.iter().enumerate() {
            let mut f = prepared[t].plain.f;
            plain = plain.mul(&f);
            let own = members.iter().any(|m| m.occupant) || members.len() >= 2;
            let occupied = members.iter().any(|m| m.occupant);
            for (i, m) in members.iter().enumerate() {
                if m.kind == Kind::Rabbit {
                    continue;
                }
                for &x in silver {
                    let d = dist(m.at, x);
                    if !(1..4).contains(&d) {
                        continue;
                    }
                    if x == TRAPS[t] {
                        // Silver on this (empty) trap next to the guard.
                        let mut ms = members.clone();
                        ms[i].fixed = Some(x);
                        let (n, sn) = cluster_blocked(t, &ms, silver);
                        f = f.add(&model.net(1, own, occupied).mul(&n.add(&sn.mul(&ps.w))));
                        f = f.add(&model.j[1].mul(&cluster(t, &ms).1));
                        continue;
                    }
                    let mut ms = members.clone();
                    ms[i].fixed = Some(x);
                    let l = weigh(cluster_blocked(t, &ms, silver));
                    f = f.add(&model.net(d, own, occupied).mul(&l));
                    let mut at = Poly::ZERO;
                    for (j, q) in members.iter().enumerate().filter(|&(j, _)| j != i) {
                        let dj = dist(q.at, x);
                        if dj < D && !(q.kind == Kind::Rabbit && x.1 < q.at.1) {
                            let mut ms = members.clone();
                            ms[i].fixed = Some(AWAY);
                            ms[j].fixed = Some(x);
                            at = at.add(&Poly::term(dj, 1).mul(&weigh(cluster(t, &ms))));
                        }
                    }
                    f = f.add(&model.j[d].mul(&at));
                }
            }
            units = units.mul(&f.clamp());
        }
        let t4 = [&prepared[0], &prepared[1], &prepared[2], &prepared[3]];
        let edges = traps_poly(t4, ps).sub(&plain).mul(&free_plain);
        // A silver piece on a trap whose fate depends on the order of two
        // pushes or pulls: a silver guard's departure against another silver
        // piece's arrival next to the trap.
        let gold: Vec<(Sq, Kind)> =
            groups.iter().flatten().map(|p| (p.at, p.kind)).chain(free.iter().copied()).collect();
        let pushers =
            |q: Sq| gold.iter().filter(|&&(g, k)| k == Kind::Other && dist(g, q) == 1).count() as i64;
        let mut order = 0;
        for &trap in TRAPS.iter().filter(|t| silver.contains(t)) {
            let around = trap_neighbors(TRAPS.iter().position(|&x| x == trap).unwrap());
            let departures: i64 = around.iter().filter(|q| silver.contains(q)).map(|&q| pushers(q) * 6).sum();
            let arrivals = 18 * around.iter().filter(|q| !silver.contains(q)).count() as i64;
            order += departures * arrivals;
        }
        units.add(&edges).add(&Poly::term(4, order))
    }

    /// The legal model never counts fewer boards than a position with silver
    /// pieces has, within each number of steps.
    #[test]
    fn legal_model_covers_positions_with_silver() {
        let ps = Polys::new();
        let uses: Vec<UseSilver> = trap_uses(true).into_iter().map(|u| use_silver(u, &ps)).collect();
        let model = SilverModel::new(&uses);
        let mut seed = 0x1234_5678_9abc_def1u64;
        let mut rnd = |n: u64| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n) as usize
        };
        let mut counter = Counter::default();
        let free_squares: Vec<Sq> = squares().filter(|&s| class(s) == Class::Other).collect();
        let kind = |r: usize| if r == 0 { Kind::Rabbit } else { Kind::Other };
        let (mut worst, mut tested, mut with_push) = (f64::MAX, 0, 0);
        while tested < 600 {
            let mut groups: Vec<Vec<Piece>> = Vec::new();
            let mut ok = true;
            for (t, &trap) in TRAPS.iter().enumerate() {
                let mut members = Vec::new();
                let occupant = rnd(5).saturating_sub(2);
                if occupant > 0 {
                    members.push(Piece {
                        at: trap,
                        kind: kind(occupant - 1),
                        occupant: true,
                        away: false,
                        guarding: false,
                        fixed: None,
                    });
                }
                for sq in trap_neighbors(t) {
                    if rnd(4) == 0 {
                        members.push(Piece {
                            at: sq,
                            kind: kind(rnd(2)),
                            occupant: false,
                            away: false,
                            guarding: false,
                            fixed: None,
                        });
                    }
                }
                ok &= !(occupant > 0 && members.len() == 1);
                groups.push(members);
            }
            if !ok {
                continue;
            }
            let mut taken: Vec<Sq> = groups.iter().flatten().map(|p| p.at).collect();
            let mut free: Vec<(Sq, Kind)> = Vec::new();
            for _ in 0..rnd(5) {
                let sq = free_squares[rnd(free_squares.len() as u64)];
                if !taken.contains(&sq) {
                    taken.push(sq);
                    free.push((sq, kind(rnd(2))));
                }
            }
            let mut silver = Vec::new();
            for _ in 1..2 + rnd(6) {
                let sq = (rnd(8) as i32, rnd(8) as i32);
                // Silver on a trap only where no gold piece is.
                if !taken.contains(&sq) {
                    taken.push(sq);
                    silver.push(sq);
                }
            }
            let mut b = Board::empty();
            let gold: Vec<(Sq, Kind)> =
                groups.iter().flatten().map(|p| (p.at, p.kind)).chain(free.iter().copied()).collect();
            if gold.len() > 16 {
                continue;
            }
            for (i, &(sq, k)) in gold.iter().enumerate() {
                let kk = if k == Kind::Rabbit { 0 } else { 2 + i % 4 };
                b.put(GOLD, kk, (sq.1 * 8 + sq.0) as usize);
            }
            for (i, &sq) in silver.iter().enumerate() {
                b.put(manymoves::SILVER, i % 2, (sq.1 * 8 + sq.0) as usize);
            }
            let pushes =
                gold.iter().any(|&(g, k)| k == Kind::Other && silver.iter().any(|&x| dist(g, x) <= 3));
            let m = silver_model(&ps, &model, &groups, &free, &silver);
            worst = worst.min(check(&mut counter, &b, &m));
            tested += 1;
            with_push += usize::from(pushes);
        }
        println!(
            "{tested} positions ({with_push} with a possible push); tightest model/real ratio {worst:.3}"
        );
    }
}
