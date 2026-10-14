# Results

Counts are from `count` and agree with howdah-arimaa's `TurnBuilder`.

| Board | Variant | Unique moves | Source |
|---|---|---|---|
| `boards/gold-372846.txt` | gold only | 372,846 | legal record without its silver rabbit |
| `boards/legal-372739.txt` | legal | 372,739 | annealing on the 3-step count |
| `boards/gold-371500.txt` | gold only | 371,500 | annealing (reached in 6 runs; a strong attractor) |
| `boards/legal-370473.txt` | legal | 370,473 | annealing |
| `boards/lattice-gold-361584.txt` | gold only | 361,584 | lattice enumeration + polish |
| `boards/aaaa_top.txt` | legal | 359,661 | historical best (aaaa, 2014) |
| `boards/boardH3w.txt` | none | 373,985 | historical; has 9 gold rabbits, so not a valid army |

The best boards have no two pieces orthogonally adjacent. They leave files a
and h and rank 8 mostly empty, and they put rabbits on rank 1, where a rabbit
loses nothing by being unable to step south. Without silver pieces only the
squares and which pieces are rabbits matter; the 371,500 board was found with
the other pieces in different orders. The legal records are the gold-only boards
plus a silver rabbit on the square that costs least: one that no gold piece
reaches in fewer than 3 steps and no stronger piece gets next to in time to
push or pull (h8 for the 372,739 board, which costs 107 moves). Adding silver
pieces to push or pull has always cost more than it gained.

## Annealing on the 3-step count

`search --proxy 3` anneals on the count of boards reachable in up to 3 steps
(about 1.6 ms against 75 ms), then counts the run's 50 best boards exactly
and polishes the best. On every board above the two counts rank the same
way. The first 8 runs found the 372,739 legal board. Twelve low-temperature
runs started from the 372,846 gold-only board all ended there, so it is a
strong local maximum. Seven of the 8 later legal runs from random starts ended
at 370,473 or 370,617; one reached 372,739.

## Lattice enumeration

`lattice` tried every coset of every index 3, 4 and 5 lattice with no two
squares adjacent, fitted to 16 squares by removing, adding or swapping up to
two squares (about 8 million candidates). The best raw candidate counts
335,995, and 361,584 after hill climbing, well below annealing's 371,500 at the time.
Even on the 2-step count, a quick proxy, the annealed board (1,576) beats
every lattice candidate (at most 1,476). The best positions aren't lattice
fragments.

## Upper bound

See BOUND.md: no gold-only position with empty traps exceeds 588,575, and
weaker bounds hold in general. That is far from a proof that any of these
boards is optimal.
