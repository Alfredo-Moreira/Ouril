#!/usr/bin/env python3
"""Static checks that the game's animation is the same on every platform (animation-parity skill).

The web's 3D board is the reference (docs/architecture/board-motion.md). Without building
anything, this checks that:

1. the motion vectors in shared/motion/vectors are there, well formed, and cover the frame kinds;
2. the frame times, the 3D pace, reduced motion, the computer's pause and every tone agree between
   the web, each other platform and timings.json;
3. each ported motion file keeps its web original's numbers (a quick drift check: the
   platforms' tests, which replay the vectors, are the real proof);
4. the board-motion doc lists every ported pair.

Usage: check_motion.py [repo root]. Exit status 1 when something disagrees.
"""
import json
import re
import sys
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
VECTORS = ROOT / "shared/motion/vectors"
WEB = ROOT / "apps/web/src"
ANDROID = ROOT / "apps/android/app/src/main/kotlin/org/moreiralabs/ouril"

# The web's motion modules and their ports (keep in step with board-motion.md's table).
PAIRS = {
    "android": [
        ("board3d/choreography.ts", "ui/board3d/Choreography.kt"),
        ("board3d/cameraRig.ts", "ui/board3d/CameraRig.kt"),
        ("board3d/rings.ts", "ui/board3d/Rings.kt"),
        ("board3d/geometry.ts", "ui/board3d/Geometry.kt"),
        ("board3d/seeds.ts", "ui/board3d/Seeds.kt"),
        ("game/animation.ts", "game/Frames.kt"),
        ("game/sound.ts", "game/Sounds.kt"),
    ],
}
PLATFORM_ROOTS = {"android": ANDROID}

# Numbers a port needs that its web original gets from a library or keeps in another file. Listed
# with the reason, so only new differences show. Values as written in the port.
EXPECTED = {
    "ui/board3d/Choreography.kt": {
        "three.js's Color: sRGB to linear, hex parsing, HSL": [0.04045, 0.0773993808, 0.9478672986, 0.0521327014, 2.4, 16, 8, 255, 4],
    },
    "ui/board3d/CameraRig.kt": {"degrees to radians (three.js's MathUtils.degToRad)": [180]},
    "ui/board3d/Geometry.kt": {"mulberry32 (the web's `rng` is in board3d/textures.ts)": [15, 61, 4294967296]},
    "game/Frames.kt": {"PACE_3D (the web's is in board3d/pace.ts)": [2.2]},
    "game/Sounds.kt": {"the WAV file's header (the web plays tones through Web Audio)": [4, 8, 16, 24, 36]},
}
FRAME_KINDS = ["pickup", "sow", "skip", "capture", "grand_slam", "extra_turn", "collect", "end"]
# Frame kinds no playable variant produces yet (board-motion.md, Open questions).
NOT_YET = {"grand_slam", "extra_turn"}

findings = []


def report(level, where, msg):
    findings.append((level, where, msg))


# --- 1. The vectors ------------------------------------------------------------------------------

def check_vectors():
    if not VECTORS.is_dir():
        report("ERROR", "shared/motion/vectors", "missing: run `just motion-vectors`")
        return None
    timings = None
    for name in ("timings.json", "seed-looks.json"):
        path = VECTORS / name
        if not path.exists():
            report("ERROR", f"shared/motion/vectors/{name}", "missing: run `just motion-vectors`")
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        if name == "timings.json":
            timings = data
    scenarios = sorted(p for p in VECTORS.glob("*.json") if p.name not in ("timings.json", "seed-looks.json"))
    if len(scenarios) < 7:
        report("ERROR", "shared/motion/vectors", f"{len(scenarios)} scenarios; expected at least 7")
    covered = set()
    for path in scenarios:
        r = path.relative_to(ROOT).as_posix()
        try:
            v = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            report("ERROR", r, f"not JSON: {e}")
            continue
        for key in ("comment", "name", "moves", "inputs", "samples", "final", "aspect", "frameSpeed", "boardSpeed", "tick", "every", "until"):
            if key not in v:
                report("ERROR", r, f"no '{key}'")
        if v.get("name") != path.stem:
            report("ERROR", r, f"name '{v.get('name')}' isn't the file's")
        if "Don't edit" not in v.get("comment", ""):
            report("WARN", r, "the comment doesn't say it's generated")
        for x in v.get("inputs", []):
            h = x.get("highlight")
            covered.add(h["kind"] if h else "end")
        if not v.get("samples"):
            report("ERROR", r, "no samples")
    missing = [k for k in FRAME_KINDS if k not in covered and k not in NOT_YET]
    if missing:
        report("ERROR", "shared/motion/vectors", f"no scenario shows these frames: {', '.join(missing)}")
    print(f"vectors: {len(scenarios)} scenarios covering {', '.join(k for k in FRAME_KINDS if k in covered)}")
    return timings


# --- 2. Timings and tones ------------------------------------------------------------------------

def num(s):
    return float(s.rstrip("fFLd"))


def web_timings():
    anim = (WEB / "game/animation.ts").read_text(encoding="utf-8")
    table = re.search(r"FRAME_BASE_MS[^=]*=\s*\{(.*?)\};", anim, re.S).group(1)
    base = {k: num(v) for k, v in re.findall(r"(\w+):\s*([\d.]+)", table)}
    pace = num(re.search(r"PACE_3D\s*=\s*([\d.]+)", (WEB / "board3d/pace.ts").read_text(encoding="utf-8")).group(1))
    reduced = num(re.search(r"REDUCED_MOTION_SPEED\s*=\s*([\d.]+)", anim).group(1))
    pause = num(re.search(r"AI_PAUSE_MS\s*=\s*([\d.]+)", anim).group(1))
    sound = (WEB / "game/sound.ts").read_text(encoding="utf-8")
    tones = {}
    for kind, body in re.findall(r"(\w+):\s*\{\s*(freq:[^}]*)\}", sound):
        fields = dict(re.findall(r"(\w+):\s*'?([\w.]+)'?", body))
        tones[kind] = {"freq": num(fields["freq"]), "jitter": num(fields["jitter"]), "ms": num(fields["ms"]),
                       "wave": fields["wave"], "gain": num(fields["gain"])}
    return base, pace, reduced, pause, tones


def android_timings():
    frames = (ANDROID / "game/Frames.kt").read_text(encoding="utf-8")
    base = {k.lower(): num(v) for k, v in re.findall(r"FrameKind\.(\w+) to (\d+)", frames)}
    pace = num(re.search(r"PACE_3D\s*=\s*([\d.]+)", frames).group(1))
    reduced = num(re.search(r"REDUCED_MOTION_SPEED\s*=\s*([\d.]+)", frames).group(1))
    pause = num(re.search(r"AI_PAUSE_MS\s*=\s*([\d.]+)", frames).group(1))
    sounds = (ANDROID / "game/Sounds.kt").read_text(encoding="utf-8")
    tones = {}
    for key, args in re.findall(r"(FrameKind\.\w+|true|false) to Tone\(([^)]*)\)", sounds):
        name = {"true": "win", "false": "loss"}.get(key, key.replace("FrameKind.", "").lower())
        positional = [a.strip() for a in args.split(",") if "=" not in a]
        named = dict((k.strip(), v.strip()) for k, v in (a.split("=") for a in args.split(",") if "=" in a))
        wave = next((p for p in positional if p.startswith("Wave.")), "Wave.SINE").split(".")[1].lower()
        numbers = [p for p in positional if not p.startswith("Wave.")]
        gain = named.get("gain", numbers[2] if len(numbers) > 2 else "0.08")
        tones[name] = {"freq": num(numbers[0]), "jitter": num(named.get("jitter", "0")), "ms": num(numbers[1]),
                       "wave": wave, "gain": num(gain)}
    return base, pace, reduced, pause, tones


def check_timings(recorded):
    web = web_timings()
    platforms = {"web": web, "android": android_timings()}
    names = ("frame times", "3D pace", "reduced-motion speed", "computer's pause", "tones")
    for platform, values in platforms.items():
        for i, name in enumerate(names):
            if values[i] != web[i]:
                report("ERROR", platform, f"{name}: {values[i]} (the web: {web[i]})")
    if recorded:
        rec = (
            {k: float(v) for k, v in recorded["frameBaseMs"].items()},
            float(recorded["pace3d"]),
            float(recorded["reducedMotionSpeed"]),
            float(recorded["aiPauseMs"]),
            {**{k: {**t, "freq": float(t["freq"]), "jitter": float(t["jitter"]), "ms": float(t["ms"]), "gain": float(t["gain"])}
                for k, t in recorded["frameTones"].items()},
             **{k: {**t, "freq": float(t["freq"]), "jitter": float(t["jitter"]), "ms": float(t["ms"]), "gain": float(t["gain"])}
                for k, t in recorded["endTones"].items()}},
        )
        for i, name in enumerate(names):
            if rec[i] != web[i]:
                report("ERROR", "shared/motion/vectors/timings.json", f"{name} differ from the web's: regenerate with `just motion-vectors`")
    print("timings and tones: " + ", ".join(platforms))


# --- 3. The ported files' numbers ----------------------------------------------------------------

def numbers(path):
    """The numeric literals of a source file, without comments or strings, as a multiset."""
    text = path.read_text(encoding="utf-8")
    text = re.sub(r"/\*.*?\*/", " ", text, flags=re.S)
    text = re.sub(r"//[^\n]*", " ", text)
    text = re.sub(r"'[^'\n]*'|\"[^\"\n]*\"|`[^`]*`", " ", text)
    found = {}
    for m in re.finditer(r"(?<![\w.])(\d+\.\d+|\d+)(?![\w.])", text):
        value = round(float(m.group(1)), 9)
        # 0, 1 and 2 are everywhere (indices, halves, doubles): they say nothing about drift.
        if value in (0.0, 1.0, 2.0):
            continue
        found[value] = found.get(value, 0) + 1
    return found


def check_ports():
    for platform, pairs in PAIRS.items():
        root = PLATFORM_ROOTS[platform]
        for web_file, port in pairs:
            w = WEB / web_file
            p = root / port
            if not w.exists():
                report("ERROR", f"apps/web/src/{web_file}", "missing")
                continue
            if not p.exists():
                report("ERROR", f"{platform}: {port}", f"missing: the port of {web_file}")
                continue
            wn = numbers(w)
            pn = numbers(p)
            known = {round(float(v), 9) for values in EXPECTED.get(port, {}).values() for v in values}
            only_web = sorted(v for v in wn if v not in pn)
            only_port = sorted(v for v in pn if v not in wn and v not in known)
            if only_web:
                report("WARN", f"{platform}: {port}", f"numbers only in {web_file}: {', '.join(f'{v:g}' for v in only_web)}")
            if only_port:
                report("WARN", f"{platform}: {port}", f"numbers only in the port: {', '.join(f'{v:g}' for v in only_port)}")
    print("ported files: " + ", ".join(f"{p} ({len(v)})" for p, v in PAIRS.items()))


# --- 4. The docs ---------------------------------------------------------------------------------

def check_docs():
    doc = ROOT / "docs/architecture/board-motion.md"
    if not doc.exists():
        report("ERROR", "docs/architecture/board-motion.md", "missing")
        return
    text = doc.read_text(encoding="utf-8")
    for platform, pairs in PAIRS.items():
        for web_file, port in pairs:
            for f in (web_file, port):
                if f"`{f}`" not in text and f"`{f.split('/')[-1]}`" not in text:
                    report("WARN", "docs/architecture/board-motion.md", f"the table doesn't name `{f}`")


def main():
    timings = check_vectors()
    check_timings(timings)
    check_ports()
    check_docs()
    for level, where, msg in findings:
        print(f"{level} {where}: {msg}")
    errors = sum(1 for f in findings if f[0] == "ERROR")
    print(f"\n{errors} error(s), {len(findings) - errors} warning(s)")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
