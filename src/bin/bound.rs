//! An upper bound on unique moves over all positions (see BOUND.md).
//!
//! Each end board of a turn has a description: every piece whose state
//! changed, with its end square or "removed". Each step moves one piece one
//! square, so a piece's Manhattan displacement costs at least that many
//! steps, and the costs add up to at most 4; silver pieces move only in
//! pushes and pulls, so silver steps never outnumber gold steps. The number
//! of descriptions within that budget bounds the number of end boards. It's
//! a truncated product of polynomials, x counting gold steps and y silver
//! steps.
//!
//! Removals are what need care. A piece is removed only on a trap with no
//! friendly neighbor. The friendly pieces next to a trap at the start (its
//! "guards") must all have moved before anything of their color is removed
//! there, so each trap and its guards get a joint polynomial: a branch with no
//! removal at that trap, and one where every guard has moved.

const D: usize = 5; // degrees 0..=4

#[derive(Clone, Copy, PartialEq, Debug)]
struct Poly([[i128; D]; D]); // [gold steps][silver steps]

impl Poly {
    fn zero() -> Poly {
        Poly([[0; D]; D])
    }
    fn one() -> Poly {
        let mut p = Poly::zero();
        p.0[0][0] = 1;
        p
    }
    fn term(g: usize, s: usize) -> Poly {
        let mut p = Poly::zero();
        p.0[g][s] = 1;
        p
    }
    fn mul(&self, o: &Poly) -> Poly {
        let mut r = Poly::zero();
        for a in 0..D {
            for b in 0..D - a {
                if self.0[a][b] == 0 {
                    continue;
                }
                for c in 0..D - a - b {
                    for d in 0..D - a - b - c {
                        r.0[a + c][b + d] += self.0[a][b] * o.0[c][d];
                    }
                }
            }
        }
        r
    }
    fn add(&self, o: &Poly) -> Poly {
        let mut r = *self;
        for a in 0..D {
            for b in 0..D {
                r.0[a][b] += o.0[a][b];
            }
        }
        r
    }
    fn sub(&self, o: &Poly) -> Poly {
        let mut r = *self;
        for a in 0..D {
            for b in 0..D {
                r.0[a][b] -= o.0[a][b];
            }
        }
        r
    }
    fn pow(&self, n: usize) -> Poly {
        (0..n).fold(Poly::one(), |acc, _| acc.mul(self))
    }
    /// Descriptions a turn can realize: at most 4 steps, silver steps no more
    /// than gold steps, the empty description (no change) left out.
    fn total(&self) -> i128 {
        let mut t = 0;
        for g in 0..D {
            for s in 0..=g.min(D - 1 - g) {
                t += self.0[g][s];
            }
        }
        t - self.0[0][0]
    }
}

const TRAPS: [(i32, i32); 4] = [(2, 2), (5, 2), (2, 5), (5, 5)];

fn dist(a: (i32, i32), b: (i32, i32)) -> usize {
    ((a.0 - b.0).abs() + (a.1 - b.1).abs()) as usize
}

#[derive(Clone, Copy, PartialEq)]
enum Class {
    Trap,
    NextToTrap,
    Other,
}

fn class(sq: (i32, i32)) -> Class {
    match TRAPS.iter().map(|&t| dist(sq, t)).min().unwrap() {
        0 => Class::Trap,
        1 => Class::NextToTrap,
        _ => Class::Other,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    GoldRabbit,
    GoldOther,
    Silver,
}

/// Coefficientwise maximum, over the squares of a class, of a piece's
/// outcomes by cost: an end square at each distance, plus removal on a trap
/// it isn't next to (its own trap's removals are in the trap polynomials),
/// costing at least the distance to that trap.
fn piece_poly(kind: Kind, cls: Class) -> Poly {
    let mut best = [0i128; D];
    for f in 0..8i32 {
        for r in 0..8i32 {
            if class((f, r)) != cls {
                continue;
            }
            let mut c = [0i128; D];
            for f2 in 0..8i32 {
                for r2 in 0..8i32 {
                    let d = dist((f, r), (f2, r2));
                    if (1..D).contains(&d) && !(kind == Kind::GoldRabbit && r2 < r) {
                        c[d] += 1;
                    }
                }
            }
            let other_trap = TRAPS.iter().map(|&t| dist((f, r), t)).filter(|&d| d >= 2).min().unwrap();
            if other_trap < D {
                c[other_trap] += 1;
            }
            for d in 0..D {
                best[d] = best[d].max(c[d]);
            }
        }
    }
    let mut p = Poly::one();
    for d in 1..D {
        if kind == Kind::Silver {
            p.0[0][d] = best[d];
        } else {
            p.0[d][0] = best[d];
        }
    }
    p
}

struct Polys {
    // [kind][class]
    p: [[Poly; 3]; 3],
}

impl Polys {
    fn get(&self, k: Kind, c: Class) -> &Poly {
        &self.p[k as usize][c as usize]
    }
}

/// What one trap holds at the start, and its neighbors.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TrapUse {
    occupant: Option<Kind>,
    rabbits: usize, // gold rabbit neighbors
    others: usize,  // other gold neighbors
    silver: usize,  // silver neighbors
}

/// The joint polynomial of one color's side of a trap: guards with outcome
/// polys `guards` (cost of a move counted by `unit`), and an occupant of that
/// color if any.
fn trap_side(guards: &[Poly], occupant: Option<&Poly>, unit: (usize, usize)) -> Poly {
    let back = Poly::term(2 * unit.0, 2 * unit.1); // moved away and came back
    let removed = Poly::term(unit.0, unit.1); // removed here, at least one step
    let all = guards.iter().fold(Poly::one(), |a, g| a.mul(g));
    // Every guard moved: changed, came back, or was removed here.
    let moved = |with_removal: bool| {
        guards.iter().fold(Poly::one(), |a, g| {
            let mut m = g.sub(&Poly::one()).add(&back);
            if with_removal {
                m = m.add(&removed);
            }
            a.mul(&m)
        })
    };
    match occupant {
        // Without guards nothing is removed here except pieces walking in
        // from further away, whose own polys count that.
        None if guards.is_empty() => Poly::one(),
        None => all.add(&moved(true).sub(&moved(false))),
        Some(t) => {
            // The occupant's outcomes, plus removal where it stands.
            let t_or_gone = t.add(&Poly::one());
            all.mul(t).add(&moved(true).mul(&t_or_gone).sub(&moved(false).mul(t)))
        }
    }
}

fn trap_poly(u: &TrapUse, ps: &Polys) -> Poly {
    use Class::*;
    let mut gold = vec![*ps.get(Kind::GoldRabbit, NextToTrap); u.rabbits];
    gold.extend(vec![*ps.get(Kind::GoldOther, NextToTrap); u.others]);
    let silver = vec![*ps.get(Kind::Silver, NextToTrap); u.silver];
    let occ = u.occupant.map(|k| *ps.get(k, Trap));
    let gold_occ = occ.filter(|_| u.occupant != Some(Kind::Silver));
    let silver_occ = occ.filter(|_| u.occupant == Some(Kind::Silver));
    trap_side(&gold, gold_occ.as_ref(), (1, 0)).mul(&trap_side(&silver, silver_occ.as_ref(), (0, 1)))
}

fn trap_uses(with_silver: bool, occupied: bool) -> Vec<TrapUse> {
    let mut v = Vec::new();
    let occupants: &[Option<Kind>] = if with_silver {
        &[None, Some(Kind::GoldRabbit), Some(Kind::GoldOther), Some(Kind::Silver)]
    } else {
        &[None, Some(Kind::GoldRabbit), Some(Kind::GoldOther)]
    };
    for &occupant in occupants {
        if occupant.is_some() && !occupied {
            continue;
        }
        for rabbits in 0..=4 {
            for others in 0..=4 - rabbits {
                for silver in 0..=if with_silver { 4 - rabbits - others } else { 0 } {
                    let gold = rabbits + others;
                    let ok = match occupant {
                        None => true,
                        Some(Kind::Silver) => silver > 0,
                        Some(_) => gold > 0,
                    };
                    if ok {
                        v.push(TrapUse { occupant, rabbits, others, silver });
                    }
                }
            }
        }
    }
    v
}

/// The largest bound over every way to use the traps, with at most
/// `rabbits` gold rabbits, `others` other gold pieces and `silver` movable
/// silver pieces (fewer pieces never raise the bound).
fn max_bound(rabbits: usize, others: usize, silver: usize, occupied: bool, ps: &Polys) -> (i128, Vec<TrapUse>) {
    let uses = trap_uses(silver > 0, occupied);
    let polys: Vec<Poly> = uses.iter().map(|u| trap_poly(u, ps)).collect();
    let n = uses.len();
    let mut best = (0, Vec::new());
    let count = |ix: &[usize]| {
        let mut c = (0, 0, 0);
        for &i in ix {
            let u = &uses[i];
            c.0 += u.rabbits + usize::from(u.occupant == Some(Kind::GoldRabbit));
            c.1 += u.others + usize::from(u.occupant == Some(Kind::GoldOther));
            c.2 += u.silver + usize::from(u.occupant == Some(Kind::Silver));
        }
        c
    };
    // The four traps are interchangeable here; take uses in nondecreasing order.
    for a in 0..n {
        for b in a..n {
            let ab = polys[a].mul(&polys[b]);
            for c in b..n {
                let abc = ab.mul(&polys[c]);
                for d in c..n {
                    let ix = [a, b, c, d];
                    let (r, o, s) = count(&ix);
                    if r > rabbits || o > others || s > silver {
                        continue;
                    }
                    let free = ps
                        .get(Kind::GoldRabbit, Class::Other)
                        .pow(rabbits - r)
                        .mul(&ps.get(Kind::GoldOther, Class::Other).pow(others - o))
                        .mul(&ps.get(Kind::Silver, Class::Other).pow(silver - s));
                    let x = abc.mul(&polys[d]).mul(&free).total();
                    if x > best.0 {
                        best = (x, ix.iter().map(|&i| uses[i]).collect());
                    }
                }
            }
        }
    }
    best
}

fn main() {
    use Class::*;
    use Kind::*;
    let mut p = [[Poly::one(); 3]; 3];
    for k in [GoldRabbit, GoldOther, Silver] {
        for c in [Trap, NextToTrap, Other] {
            p[k as usize][c as usize] = piece_poly(k, c);
        }
    }
    let ps = Polys { p };
    for (name, k) in [("gold rabbit", GoldRabbit), ("gold other", GoldOther), ("silver", Silver)] {
        for (cname, c) in [("on trap", Trap), ("next to trap", NextToTrap), ("elsewhere", Other)] {
            let q = ps.get(k, c);
            let co: Vec<i128> = (0..D).map(|d| if k == Silver { q.0[0][d] } else { q.0[d][0] }).collect();
            println!("{name:12} {cname:13} {co:?}");
        }
    }
    // Usage: bound [empty-traps] [silver counts...]
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let occupied = args.first().map(String::as_str) != Some("empty-traps");
    if !occupied {
        args.remove(0);
    }
    let silvers: Vec<usize> =
        if args.is_empty() { vec![0, 1, 2, 15] } else { args.iter().map(|s| s.parse().unwrap()).collect() };
    for silver in silvers {
        let (b, t) = max_bound(8, 8, silver, occupied, &ps);
        let traps = if occupied { "" } else { ", traps empty" };
        println!("gold army + {silver} movable silver{traps}: {b}");
        for u in t {
            println!("    {u:?}");
        }
    }
}
