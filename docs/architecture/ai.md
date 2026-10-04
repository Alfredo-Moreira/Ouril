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

## Later

- **Hints and analysis:** reuse the search to suggest a move or rate past moves.
- **Server-side bot** for "play vs bot" while waiting for matchmaking.
- **Endgame tables** for perfect play with few seeds left.

## Open questions

- Time budget per level on low-end Android devices.
- Should Hard be "strong" or "near-perfect"? Oware can be played very strongly by computers, which may not be fun.
