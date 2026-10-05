#!/usr/bin/env python3
"""Static checks for Ouril rule test vectors. Stdlib only. Read-only.

Usage: check_vectors.py [REPO_ROOT] [FILE_OR_DIR ...]
Default target: <REPO_ROOT>/core/test-vectors (skips schema/).

This is NOT a rules engine. It never decides whether a move is legal or what it
captures; that is the engine's job, checked by `cargo test`. It only catches
mistakes a careful human makes when writing vectors by hand:
  - format: required fields, id == path, known event types and error codes
  - variants: every {id, version} has a spec in docs/game/variants/
  - board shape: pits length == 2 * pits_per_side, no negative counts
  - seed conservation: sum(pits) + sum(stores) == 2 * pits_per_side * seeds_per_pit
  - bookkeeping between consecutive explicit states:
      * the moved pit belongs to the side to move and is non-empty
      * number of `sow` events == seeds in the moved pit + seeds picked up by `relay` events
      * sum of `capture` seeds == mover's store gain (ignoring collect_remaining)
      * the moved pit is empty afterwards unless a lap re-filled it
  - expect_legal_moves is sorted, unique and on the mover's side
  - `source` points at an existing doc and anchor
Exit code 0 = clean, 1 = problems found.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
TARGETS = [Path(a).resolve() for a in sys.argv[2:]] or [ROOT / "core/test-vectors"]
EVENTS = {"sow": {"pit"}, "skip_origin": {"pit"}, "capture": {"pit", "seeds"},
          "relay": {"pit", "seeds"},
          "grand_slam": set(), "extra_turn": set(), "collect_remaining": {"player", "seeds"},
          "game_over": {"result", "reason"}}
ERRORS = {"not_own_pit", "empty_pit", "single_seed_rule", "must_feed", "grand_slam_forbidden", "game_over"}
STATUSES = {"playing", "south_wins", "north_wins", "draw"}
PROBLEMS = []


def problem(path, where, msg):
    PROBLEMS.append(f"{path}{' ' + where if where else ''}: {msg}")


def variant_specs():
    """Map variant id -> resolved pits_per_side / seeds_per_pit / version from spec front matter."""
    specs = {}
    for spec in (ROOT / "docs/game/variants").glob("*/*.md"):
        text = spec.read_text(encoding="utf-8")
        if not text.startswith("---\n"):
            continue
        fm = text[4:text.find("\n---\n", 4)]
        get = lambda k: (re.search(rf"^\s*{k}:\s*([\w.-]+)", fm, re.M) or [None, None])[1]
        resolved = fm.split("\nresolved:", 1)[-1]
        rget = lambda k: (re.search(rf"^\s+{k}:\s*(\d+)", resolved, re.M) or [None, None])[1]
        if get("id"):
            specs[get("id")] = {"pits_per_side": int(rget("pits_per_side") or 0),
                                "seeds_per_pit": int(rget("seeds_per_pit") or 0),
                                "version": int(get("version") or 0)}
    return specs


def slug(h):
    h = re.sub(r"[`*_]", "", h).strip().lower()
    return re.sub(r"\s+", "-", re.sub(r"[^\w\s-]", "", h))


def check_source(path, src):
    file_part, _, frag = src.partition("#")
    doc = ROOT / file_part
    if not doc.exists():
        return problem(path, "source", f"file not found: {file_part}")
    if frag:
        heads = {slug(m) for m in re.findall(r"^#{1,6}\s+(.+)$", doc.read_text(encoding="utf-8"), re.M)}
        if frag not in heads:
            problem(path, "source", f"anchor #{frag} not found in {file_part}")


def side_of(pit, n):
    return "south" if pit < n else "north"


def check_state(path, where, st, n, total):
    pits, stores = st.get("pits"), st.get("stores")
    if pits is not None:
        if len(pits) != 2 * n:
            problem(path, where, f"pits has {len(pits)} entries, expected {2 * n}")
        if any((not isinstance(x, int)) or x < 0 for x in pits):
            problem(path, where, "pits must be non-negative integers")
    if stores is not None and (len(stores) != 2 or any(x < 0 for x in stores)):
        problem(path, where, "stores must be [south, north], non-negative")
    if pits is not None and stores is not None and sum(pits) + sum(stores) != total:
        problem(path, where, f"seed conservation: {sum(pits)} + {sum(stores)} != {total}")
    if "to_move" in st and st["to_move"] not in ("south", "north"):
        problem(path, where, f"to_move must be south|north, got {st['to_move']!r}")


def check_vector(path, base):
    try:
        v = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as e:
        return problem(path, "", f"invalid JSON: {e}")
    for field in ("id", "description", "variants", "source", "setup", "steps"):
        if field not in v:
            problem(path, "", f"missing required field '{field}'")
    expected_id = path.relative_to(base).with_suffix("").as_posix()
    if v.get("id") != expected_id:
        problem(path, "id", f"'{v.get('id')}' should equal path '{expected_id}'")
    specs = variant_specs()
    shapes = set()
    for var in v.get("variants", []):
        spec = specs.get(var.get("id"))
        if not spec:
            problem(path, "variants", f"no spec for variant '{var.get('id')}'")
            continue
        if var.get("version", 0) > spec["version"]:
            problem(path, "variants", f"{var['id']}@{var['version']} is newer than spec version {spec['version']}")
        shapes.add((spec["pits_per_side"], spec["seeds_per_pit"]))
    if "source" in v:
        check_source(path, v["source"])
    if len(shapes) != 1:
        if len(shapes) > 1:
            problem(path, "variants", f"variants disagree on board shape {shapes}; split the vector")
        return
    n, seeds = shapes.pop()
    total = 2 * n * seeds
    setup = v.get("setup")
    state = None
    if isinstance(setup, dict) and "pits" in setup:
        check_state(path, "setup", setup, n, total)
        state = dict(setup)
    for i, step in enumerate(v.get("steps", [])):
        where = f"steps[{i}]"
        legal = step.get("expect_legal_moves")
        mover = state.get("to_move") if state else None
        if legal is not None:
            if legal != sorted(set(legal)):
                problem(path, where, "expect_legal_moves must be sorted and unique")
            if mover and any(side_of(p, n) != mover for p in legal):
                problem(path, where, f"expect_legal_moves lists pits not owned by {mover}")
        if "expect" in step and "expect_error" in step:
            problem(path, where, "use either expect or expect_error, not both")
        if "expect_error" in step and step["expect_error"] not in ERRORS:
            problem(path, where, f"unknown error code '{step['expect_error']}'")
        move = step.get("move")
        exp = step.get("expect", {})
        for j, ev in enumerate(exp.get("events", [])):
            t = ev.get("type")
            if t not in EVENTS:
                problem(path, f"{where}.events[{j}]", f"unknown event type '{t}'")
            elif set(ev) - {"type"} != EVENTS[t]:
                problem(path, f"{where}.events[{j}]", f"'{t}' needs fields {sorted(EVENTS[t])}")
        if exp.get("status") and exp["status"] not in STATUSES:
            problem(path, where, f"unknown status '{exp['status']}'")
        check_state(path, where + ".expect", exp, n, total)
        # Bookkeeping between two explicit states.
        if state and move is not None and "expect_error" not in step:
            if not 0 <= move < 2 * n:
                problem(path, where, f"move {move} is off the board")
            elif mover and side_of(move, n) != mover:
                problem(path, where, f"move {move} is not on {mover}'s side")
            elif state["pits"][move] == 0:
                problem(path, where, f"move {move} sows from an empty pit")
            elif "events" in exp:
                evs = exp["events"]
                sown = sum(1 for e in evs if e.get("type") == "sow")
                # Continuous sowing: each `relay` picks up more seeds and sows them too.
                relayed = sum(e.get("seeds", 0) for e in evs if e.get("type") == "relay")
                if sown != state["pits"][move] + relayed:
                    problem(path, where, f"{sown} sow events but pit {move} held {state['pits'][move]} seeds"
                            + (f" (+{relayed} relayed)" if relayed else ""))
                if "stores" in exp and "stores" in state and mover and not any(
                        e.get("type") == "collect_remaining" for e in evs):
                    k = 0 if mover == "south" else 1
                    gain = exp["stores"][k] - state["stores"][k]
                    captured = sum(e.get("seeds", 0) for e in evs if e.get("type") == "capture")
                    if gain != captured:
                        problem(path, where, f"{mover} store grew by {gain} but captures total {captured}")
                if "pits" in exp and exp["pits"][move] != 0 and not any(
                        e.get("type") == "sow" and e.get("pit") == move for e in evs):
                    problem(path, where, f"origin pit {move} is not empty after the move")
        if "pits" in exp and "stores" in exp:
            state = {**exp, "to_move": exp.get("to_move", mover)}
        elif move is not None:
            state = None  # can't follow the position any further


def main():
    files = []
    for t in TARGETS:
        files += [t] if t.is_file() else sorted(p for p in t.rglob("*.json") if "schema" not in p.parts)
    base = ROOT / "core/test-vectors"
    for f in files:
        check_vector(f, base if base in f.parents else f.parent)
    print("\n".join(PROBLEMS))
    print(f"{len(files)} vector file(s), {len(PROBLEMS)} problem(s)")
    sys.exit(1 if PROBLEMS else 0)


if __name__ == "__main__":
    main()
