# Ouril rules (Cape Verde, standard): MVP default

> Plain-language rules for the MVP variant `cv.standard`, with worked examples. **Status:** Accepted. Researched; points no source covers are marked **App decision**.

This is the readable version. The machine-readable spec, with sources and a confidence level for each rule, is [variants/cape-verde/standard.md](variants/cape-verde/standard.md). Other variants are in the [variant catalog](variants/README.md).

## Setup

- The board has two rows of six pits (*casas* / *buracos*). Each player owns the row nearest to them.
- Each pit starts with **4 seeds** (*ouris*), so 48 in total.
- Each player has a store (*depósito*) for captured seeds, which starts empty.
- **First player:** random for the first game. In a series, the **loser of the previous game starts**.

### Board notation

The engine and these docs number the pits counter-clockwise, as South sees the board:

```
            North
   ┌────┬────┬────┬────┬────┬────┐
   │ 11 │ 10 │  9 │  8 │  7 │  6 │
   ├────┼────┼────┼────┼────┼────┤
   │  0 │  1 │  2 │  3 │  4 │  5 │
   └────┴────┴────┴────┴────┴────┘
            South
```

- South owns pits `0–5` and North owns pits `6–11`.
- Sowing always goes to the next index: `0 → 1 → … → 11 → 0`.

## Playing a move (sowing)

1. On your turn, pick one of **your own non-empty pits**.
2. **Single-seed rule:** you may **not** pick a pit holding just **1 seed** while any of your pits holds **2 or more**.
3. Take all of its seeds and drop them one at a time into each following pit, counter-clockwise.
4. **12 or more seeds:** the sowing laps the board, and you **skip the pit you started from**, which stays empty.

## Capturing

- If your **last seed** lands in an **opponent's pit** and leaves it with exactly **2 or 3 seeds**, you capture those seeds.
- **Chain capture:** then check the pit just before it. If it is also on the opponent's side and holds 2 or 3 seeds, capture it too. Keep going backwards until you reach a pit that doesn't hold 2 or 3, or you reach your own side.
- Captures are made whenever possible. A pit with 4 or more seeds is never captured.

## Feeding

- If your opponent has **no seeds** on their side, you **must** make a move that gives them at least one seed.
- If no move can feed them, the game ends and **you collect the seeds left on your side**.
- If the only move that feeds them is from a single-seed pit, feeding takes priority and that move is allowed. **App decision**

## Grand slam

A **grand slam** is a move that captures **every** seed on the opponent's side.

- It is **allowed**, and you keep the seeds.
- You then **play again immediately** (the opponent's turn is skipped), and that extra move **must leave the opponent at least one seed**.
- If the grand slam already gives you **25 or more**, you win at once and there's no extra move.
- If the extra move can't feed the opponent, the game ends and you collect the seeds on your side.

## End of the game

The game ends when:

1. A player has captured **25 or more** seeds. They win at once. Any seeds still on the board go to the player on whose side they are. This only completes the final score; it can't change the winner. **App decision**
2. No move can feed a starving opponent (see above).
3. **Endless cycle:** a few seeds circle without any chance of capture. **Each player collects the seeds on their own side.** In the app, this happens when a position repeats (same pits, same player to move) or after **100 moves in a row without a capture**, whichever comes first. **App decision**

Whoever has captured more seeds wins. **24–24 is a draw.**

**Match play (optional):** in a series, capturing **36 or more** counts as a **2–0** win.

## Worked examples

Board states are written as `South[0..5] | North[6..11]`.

### 1. Simple sowing

Opening position: `4 4 4 4 4 4 | 4 4 4 4 4 4`. South plays pit **2**.

- Pick up 4 and sow into pits 3, 4, 5, 6.
- Result: `4 4 0 5 5 5 | 5 4 4 4 4 4`.
- The last seed landed in pit 6, which now holds 5. No capture.

### 2. Capture with a chain

Before: `4 4 4 4 3 0 | 1 2 4 4 4 4`, stores 5 (South) and 5 (North). South plays pit **4** (3 seeds).

- Sow into 5, 6, 7. Pit 5 → 1, pit 6 → 2, pit 7 → 3.
- The last seed is in pit 7 (opponent's side) with **3** → capture 3.
- Check pit 6: **2** → capture 2.
- Check pit 5: that is South's own side → stop.
- **South captures 5 seeds.** After: `4 4 4 4 0 1 | 0 0 4 4 4 4`, stores 10 and 5.

### 3. Lap with 12+ seeds

Before: `12 0 0 0 0 0 | 9 9 9 9 0 0`, stores 0 and 0. South plays pit **0** (12 seeds).

- Sow 11 seeds into pits 1–11.
- Skip pit 0 (the origin) and drop the 12th seed into pit 1.
- Pit 1 receives 2 seeds in total and pit 0 stays empty. The last seed is on South's side, so there is no capture.
- After: `0 2 1 1 1 1 | 10 10 10 10 1 1`.

### 4. Single-seed rule

South: `1 0 3 1 0 2 | …`

- Pits 0 and 3 hold a single seed, while pits 2 and 5 hold more. **Only pits 2 and 5 are legal.**
- If South had `1 0 0 1 0 1 | …` (no pit with 2 or more), pits 0, 3 and 5 would all be legal.

### 5. Grand slam with extra turn

Before: `0 0 0 1 3 0 | 1 2 0 0 0 0`. South has 18 in store and North has 23. South plays pit **4** (3 seeds).

- Sow into 5, 6, 7. Pit 5 → 1, pit 6 → 2, pit 7 → 3. Capture 3 + 2 = 5, which is **every** North seed: a grand slam.
- South now has 23, which is less than 25, so **South plays again**, and the move must give North a seed.
- Board: `0 0 0 1 0 1 | 0 0 0 0 0 0`. Pits 3 and 5 both hold 1 seed and no pit holds 2+, so the single-seed rule allows both. But pit 3 only reaches pit 4, while pit 5 reaches pit 6. **South must play pit 5.**

### 6. Must feed

Before: North has **no seeds**. South has `pit1=2, pit5=1`, all other pits empty.

- Pit 1 sows into 2 and 3, so it doesn't reach North and is **illegal** here.
- Pit 5 sows into 6, so it feeds North. **South must play pit 5**. The single-seed rule gives way to feeding here (**App decision**).
- If neither move could reach North, the game would end and South would collect their remaining seeds.

## App decisions

No source covers five points, so the app decides them. Each is recorded with its rationale in the [variant spec](variants/cape-verde/standard.md#decisions-without-a-source):

1. Feeding takes priority over the single-seed rule (examples 5 and 6).
2. An endless cycle is a repeated position, or 100 moves without a capture.
3. The first game's starting player is random.
4. When a player reaches 25, the seeds left on the board go to the player on whose side they are.
5. Match scoring (36+ = a 2–0 win) applies only in match mode. Single games just record a win, loss or draw.

If players tell us they play these differently, the spec is updated with `/rule-change`.

## Open questions

Community confirmation is welcome, but nothing here blocks implementation. See the [variant spec's open questions](variants/cape-verde/standard.md#open-questions): whether the grand slam extra turn and mandatory capture apply on every island.
