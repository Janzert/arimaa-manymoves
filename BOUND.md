# An upper bound on unique moves

`target/release/bound` computes a number that no position's unique move
count can exceed. It is far above the best positions found, so it does
not prove any of them optimal. It shows how much room is left.

```bash
target/release/bound               # gold pieces only
target/release/bound empty-traps   # gold pieces only, nothing on the traps
target/release/bound legal         # every legal position (about a minute and 1.5 GB)
```

## Results

| Positions | Bound | Best found |
|---|---|---|
| Legal | 649,394 | 372,739 |
| Gold only | 626,365 | 372,846 |
| Gold only, traps empty | 539,328 | 372,846 |

An earlier version of the argument gave 1,334,950 for gold-only positions
and 588,575 with the traps empty. It covered legal positions only with at
most one silver piece able to move (795,919).

## The argument

Fix a position with gold to move. Any turn ends on a board that can be
described by listing the pieces whose state changed, each with its end square
or "removed". The board follows from the description, so there are no more
end boards than there are descriptions that some turn produces.

Costs: every step moves one piece one square, so a piece that ends `d` squares
(Manhattan distance) from its start took at least `d` steps. A removed piece
took at least one step onto a trap, unless it started on a trap. The costs in a
description add up to at most 4.

So the number of descriptions is at most the sum of the coefficients of degree
1 to 4 in a product of one polynomial per piece, `1 + a1·t + a2·t² + a3·t³ +
a4·t⁴`, where `ad` is the number of squares at distance exactly `d` (on the
board, never south of a gold rabbit), plus one for removal at the nearest
trap's distance. Taking each coefficient's maximum over the squares a piece can
stand on makes the bound hold for every position. More pieces never lower it,
so it is computed for a full army.

### Traps

A piece is removed only on a trap with no friendly neighbor, and the trap is
checked after every step. Two consequences:

- **Removal needs the guards gone.** Every friendly piece next to the trap at
  the start (its guards) must have moved away first. A guard that ends where it
  started but had to leave stepped away and back (2 steps). An occupant removed
  on its trap waits for its guards to leave, so a guard that ends on the trap
  went elsewhere first: by parity, 3 steps.
- **A piece left on a trap has a neighbor at the end.** An occupant that stays
  while every guard leaves survives only if some gold piece ends next to the
  trap: a guard that moved to another side, or a *witness* from elsewhere. The
  same holds for a guard that steps onto an empty trap: alone, it is removed.

Each trap's group (its occupant and guards) is counted exactly instead of by a
product: every combination of the members' end states within 4 steps, with
distinct end squares (a member can end on another's square only if that one
moved), the costs above, and the rules above. Rabbits never step south, so a
rabbit north of a trap can't step onto it. States that leave a piece on the trap
with no member next to it are counted separately and multiplied by the ways a
witness can arrive: a free piece stepping next to the trap (summed over every
square a free piece can start on), or a member of another trap's group (an
extra term for each such pair of traps; each costs at least 2 steps, so at most
two occur). The program takes the largest result over every way to fill the
four traps and their neighbors, and over every placement of the guards.

### Silver pieces

Silver pieces move only in pushes and pulls by stronger gold pieces. Call a
non-rabbit gold piece G and a silver piece S at most 3 squares apart an
*incidence*. A turn either has no push or pull, or its first push or pull is
some incidence's G on S.

**Turns without a push or pull** end on boards where no gold piece is on a
silver square. So for each incidence, the gold-only model's descriptions with G
ending on S's square are not needed. These include G ending there (cost `d`)
with every state of G's group-mates. Counting only states with no group-mate on
another silver square keeps these sets disjoint, and they all lie within the
group's own count, so at each number of steps they total at most the group's
count there.

**Turns with a push or pull** are counted by their first one, G on S. G and S
end in one of the pair's joint end states. Those are enumerated with the two
pieces alone on the board, at their fewest steps, allowing either piece to
live or die on a trap. If G ends away from its trap, its group-mates' state is
also valid with G on S's square, so it is among the ones just removed. If G
ends next to its trap, the group-mates may rely on it, and removing the
occupant instead gives such a state (so those count twice). On top of that:

- another gold piece may end on S's vacated square (at most 3, 8 and 12 pieces
  at distance 1 to 3), with its own group's extra states;
- G may become another trap's witness, or another trap's member may;
- up to two more pushes or pulls can follow a first one at distance 1;
- a silver piece on a trap can live or die depending on the order of two
  pushes or pulls (at most 1,296 extra boards per trap).

So each incidence adds `(gains − t^d) × L`, where `L` is its group-mates' states
(1 for a free piece), at least 1 and at most its unblocked count. The total is
a product over free pieces and trap groups, linear in each `L` and each number
of incidences. Its maximum is at the corners of their ranges, and the program
searches all of them, trap by trap. It keeps only configurations that aren't
dominated, merges ones within 1% of each other into their coefficientwise
maximum (still an upper bound), and drops those that can't reach the best
configuration without silver. The witness terms between traps (47,064 at
most) and the silver-on-trap order terms (5,184) are added as constants.

The best configuration without silver gives 581,511 before those constants;
silver adds about 16,000 on top. The search's best configuration has an empty
trap with four non-rabbit guards surrounded by silver.

Tests check every rule against exact counts. They take random positions (one
trap group, groups on all four traps, and groups with weaker silver pieces
around them, including silver on a trap) and confirm that the model for each
position never counts fewer boards than the position has, at each number of
steps from 1 to 4. Without the push and pull terms the silver test fails.

## What's left

Without any removals at all (no traps), the product for a gold army gives
471,734, and the best positions reach 372,846. The bounds ignore two things
that keep real positions far lower:

- **Blocking.** End squares must be empty at the end. Pieces packed close
  enough to interact take away each other's squares.
- **Collisions.** Two pieces moving into the same square is counted as two
  outcomes here, but at most one of them can happen in a single turn.

The traps still add the most: the gold-only bound is highest with a rabbit on
each trap and one guard. The witness count assumes every square around a
trap's neighbors can hold a free piece, and doesn't charge those pieces for
crowding the guards.

A proof that a position is optimal would need a bound that accounts for both
and comes down to the record. Searching all placements exhaustively is out of
reach: there are about 3·10^22 ways to place a gold army even before silver
pieces.
