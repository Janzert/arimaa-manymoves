# manymoves

Searching for the Arimaa position with the most unique moves: the most
distinct boards that one turn (1 to 4 steps, pushes and pulls included) can
end on, not counting the starting board. Repetition is ignored. Gold is the
side to move.

Two variants:

- **legal**: a position that can arise in a game with gold to move. Both
  sides have a rabbit, no rabbit stands on its goal, and no trap holds an
  unguarded piece.
- **gold**: gold pieces only (at most a normal army).

## Tools

    cargo build --release
    target/release/count [--ref] BOARD...    # unique moves of board files
    target/release/crosscheck [N] [SEED]     # fast count vs howdah-arimaa on random crowded boards
    target/release/search legal|gold [THREADS] [ITERS] [T_START] [T_END] [START_BOARD]
    target/release/bound [empty-traps] [SILVER...]   # upper bound over all positions (BOUND.md)
    target/release/lattice [THREADS] [SWAPS]  # lattice-based candidates (RESULTS.md)

`count` is a bitboard DFS over single steps, pushes and pulls with
deduplication of intermediate positions. With `--ref` it also enumerates every
legal step sequence through howdah-arimaa's `TurnBuilder`, which applies the
full rules. The two agree on the historical boards and on 23,000 random
crowded positions, and they match pyrimaa's `get_moves` on `aaaa_top`
(359,661).

`search` runs simulated annealing (moves, swaps, adds and removes pieces) and
then hill-climbs until no single change helps. It writes new bests to
`results/<variant>/<count>.txt` and each run's local maximum to
`results/<variant>/local-<count>.txt`. Keep it to 8 threads or fewer.

Board files use the usual diagram (`8| . . r ... |`), as in `boards/`.
