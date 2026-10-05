# 0022. Forfeit a game, recorded as game record format 2

- **Status:** Accepted
- **Date:** 2026-10-04

## Context
Players want to give up a game against the computer without playing it out. Today a game ends only by the rules, and every finished game is a format 1 record that the server checks by replaying the moves to the engine's own end ([data and sync](../architecture/data-and-sync.md)). A forfeited game stops while the rules still say "playing", so the server would reject it as `illegal_game`. Online multiplayer already plans a `resign` action ([multiplayer](../product/features/multiplayer.md)).

## Decision
- Players can **forfeit** from the in-game menu or the home page's resume card, after a confirmation. **Starting a new game** while one is unfinished also forfeits it, even with no moves played, so a new game can't be used to dodge a loss. The opponent wins, and the game is recorded, synced and counted in stats as a **loss**, like any finished game.
- The record gains one end reason, **`resigned`**, in a new **record format 2**. Only game records use it (`RecordEndReason` in `ouril-sync`); the engine's own `EndReason` and its `game_over` event don't change.
- Clients write format 2 **only** for forfeited games. Games the rules ended stay format 1, so a server that only knows format 1 keeps accepting them; it answers a forfeit `deferred`, and the client sends it again after the server is updated ([versioning](../architecture/versioning.md)).
- The server accepts a forfeit when:
  - the moves replay legally to a game **still in progress**;
  - the stores match that position;
  - `human_player` is present (that side resigned) and the **other side** is the winner.
- Leaving a game without forfeiting is unchanged: the game is saved after every move and resumes from the home screen ("Save and exit" in the menu).

## Alternatives considered
- **Add `Resigned` to the engine's `EndReason`:** the engine would carry a value it never produces, and every `game_over` consumer would have to handle it.
- **Discard a forfeited game:** a free way to avoid losses, and stats would no longer reflect what was played.
- **Keep forfeits local only (never synced):** stats would differ between devices, which data and sync rules out (stats are derived from the same records everywhere).
- **Bump every record to format 2:** older servers would defer every game until updated, not just forfeits.

## Consequences
- `ouril-sync`, the server's record check, the WASM TypeScript types and the web app change together. iOS and Android write format 2 records the same way when they forfeit.
- Readers support formats 1 and 2. Stats need no change: they count outcomes, and a forfeit is a loss.
- Replays of a forfeited game end mid-game. The result shows "You forfeited the game."
