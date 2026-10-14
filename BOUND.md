# An upper bound on unique moves

`target/release/bound` computes a number that no position's unique move
count can exceed. It is far above the best positions found, so it does
not prove any of them optimal. It shows how much room is left.

## The argument

Fix a position with gold to move. Any turn ends on a board that can be
described by listing the pieces whose state changed, each with its end square
or "removed". The board follows from the description, so there are no more
end boards than there are descriptions that some turn produces.

Costs: every step moves one piece one square, so a piece that ends `d` squares
(Manhattan distance) from its start took at least `d` steps. A removed piece
took at least one step onto a trap, unless it started on a trap. The costs in a
description add up to at most 4. Silver pieces move only in pushes and pulls,
where each silver step comes with a gold step, so silver costs add up to no
more than gold costs.

So the number of descriptions is at most the sum, over all choices of changed
pieces and outcomes with total cost ≤ 4 (silver ≤ gold), of 1. That is the
sum of the coefficients of degree ≤ 4 in a product of one polynomial per
piece: `1 + a1·x + a2·x² + a3·x³ + a4·x⁴`, where `ad` is the number of squares
at distance exactly `d` (on the board; never south of a gold rabbit), plus one
for removal at the nearest trap's distance. Silver pieces use `y` instead of
`x`. Leaving out the constant term (nothing changed) gives the bound.

Taking each coefficient's maximum over all squares (per kind of square: on a
trap, next to a trap, elsewhere) makes the bound hold for every position. More
pieces never lower it, so it's computed for a full army.

### Traps

Without care, removals inflate the count, because a piece next to a trap
could step onto it and either survive or be removed. A piece is removed only
on a trap with no friendly neighbor. Every friendly piece next to the trap at
the start (its guards) must then have moved away first. Each trap and its
guards get a joint polynomial with two branches. In the first, some guard
never moved and nothing of that color is removed there. In the second, every
guard moved (to a new square, there and back again at cost 2, or onto the
trap and removed) and something was removed there. A piece starting on a
trap can be removed at no cost of its own, but only in the second branch.
The program takes the largest result over every way to fill the four traps
and their neighbors.

## Results

| Positions | Bound | Best found |
|---|---|---|
| Gold only, traps empty | 588,575 | 372,846 |
| Gold only | 1,334,950 | 372,846 |
| Legal, 1 movable silver piece, traps empty | 795,919 | 372,739 |

Without any removals at all (no traps), the same product for a gold army
gives 471,734. That is roughly the most this approach can show, because it
ignores the two things that keep real positions far lower:

- **Blocking.** End squares must be empty at the end. Pieces packed close
  enough to interact take away each other's squares.
- **Collisions.** Two pieces moving into the same square is counted as two
  outcomes here, but at most one of them can happen in a single turn.

A proof that a position is optimal would need a bound that accounts for both
and comes down to the record. Searching all placements exhaustively is out of
reach: there are about 3·10^22 ways to place a gold army even before silver
pieces.
