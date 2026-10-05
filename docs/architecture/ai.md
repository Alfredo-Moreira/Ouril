# AI opponent

> Design of `core/ai` (Rust), the computer opponent. **Status:** Accepted

## Approach

Oware has a small branching factor (at most 6 moves), so a classic **minimax search with alpha-beta pruning** using the shared [engine](engine.md) is strong and simple. Written in Rust, it runs natively on iOS and Android and as WebAssembly on the web.

- **Search:** iterative deepening alpha-beta with move ordering (captures first) and a transposition table.
- **Evaluation:** `(my_store − opp_store)`, plus smaller weights for mobility, seeds on my side, and vulnerable pits (opponent pits with 1–2 seeds that I can reach).
- **Time-bounded:** stop at a time budget, not a fixed depth, so it performs well on slow phones.
- **Off the UI thread:** a background task on iOS (Swift concurrency) and Android (coroutines), and a **Web Worker** in the browser.
- Works with any variant config, because the search only calls `legal_moves` and `apply_move`.

## Difficulty levels

| Level | Behaviour |
|---|---|
| **Easy** | Depth 1–2. Picks randomly among moves within a margin of the best, and sometimes makes a random move. |
| **Medium** | Depth ~4–6, with small randomness among near-equal moves. |
| **Hard** | Full time budget (~500 ms), with no randomness. |

Randomness uses a seed passed in by the caller, so a game vs AI can be reproduced exactly for bug reports.

Levels are tuned by playtesting. Track win rates per level and aim for Easy to be beatable by beginners.

## As built

`core/ai` (`ouril-ai`) implements the approach above, with one change: **the search is bounded by a node budget, not a time budget.** Node counts are the same on every platform, so `(variant, state, level, seed)` always gives the same move, which wall-clock time can't guarantee. The budgets below are placeholders until playtesting.

| Level | Max depth | Node budget | Randomness |
|---|---|---|---|
| Easy | 2 | 2,000 | A random legal move 1 time in 5. Otherwise random among moves within 1.5 seeds of the best. |
| Medium | 6 | 50,000 | Random among moves within 0.3 seeds of the best |
| Hard | 64 | 1,000,000 | None. Measured: about 275 ms native and 345 ms as WASM in Node on a laptop |

- **Search:** iterative-deepening negamax with alpha-beta, captures searched first, and a small transposition table that only stores best moves, for move ordering. A grand-slam extra turn (same player to move again) is handled without negating the score.
- **Evaluation:** scores are in hundredths of a seed: `100 × (my store − their store) + 5 × (seeds on my side − seeds on theirs) + 15 × (my threats − their threats)`. A threat is a pit whose last seed would land on an opponent pit holding 1–2 seeds. There's no mobility term yet.
- **API:** `choose_move(v, state, level, seed)` returns `None` only when there's no legal move. `search()` and `SearchParams` are public, so hints reuse them. The web app calls `aiMove` from a module Web Worker, and its hint button runs the same search for the player's side.

## Later

- **Hints and analysis:** reuse the search to suggest a move or rate past moves.
- **Server-side bot** for "play vs bot" while waiting for matchmaking.
- **Endgame tables** for perfect play with few seeds left.

## Open questions

- Node budget per level on low-end Android devices. Hard has to answer in under 1 s on a mid-range phone, and a time cap could be added on top of the node budget, at the cost of reproducibility.
- Should Hard be "strong" or "near-perfect"? Oware can be played very strongly by computers, which may not be fun.
