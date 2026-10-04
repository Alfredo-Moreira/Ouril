#!/usr/bin/env python3
"""Mechanical checks for Ouril docs. Stdlib only. Read-only.

Usage: check_docs.py [REPO_ROOT]
Prints one finding per line as `LEVEL path:line: message`, then a summary.
Exit code: 0 = no errors (warnings allowed), 1 = errors found.

Checks
  links     relative Markdown links resolve to a file, and #anchors exist
  header    docs/**.md start with a purpose line containing **Status:** (ADRs and
            variant specs have their own formats and are checked separately)
  openq     docs/**.md end with an "## Open questions" section (same exemptions)
  adr       ADR file names, titles, Status values, Date, and the index table
  catalog   variant specs <-> catalog rows in docs/game/variants/README.md
"""
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
FINDINGS = []

# Docs that are allowed to skip the purpose/Status header or Open questions.
# Keep this list short and justified; every entry is a deliberate exception.
HEADER_EXEMPT = set()
OPENQ_EXEMPT = {"docs/README.md"}  # pure index
ADR_STATUSES = ("Proposed", "Accepted", "Rejected", "Superseded by")


def report(level, path, line, msg):
    FINDINGS.append((level, f"{path}:{line}", msg))


def rel(p):
    return p.relative_to(ROOT).as_posix()


def strip_code(text):
    """Blank out fenced code blocks (keep line count) so links in code are ignored."""
    out, fence = [], None
    for line in text.split("\n"):
        m = re.match(r"^\s*(```+|~~~+)", line)
        if m:
            fence = None if fence and m.group(1).startswith(fence[0]) else (fence or m.group(1))
            out.append("")
            continue
        out.append("" if fence else re.sub(r"`[^`]*`", "", line))
    return "\n".join(out)


def slugify(heading):
    """GitHub-style anchor slug."""
    h = re.sub(r"<[^>]+>", "", heading).strip().lower()
    h = re.sub(r"[*_`]|\[([^\]]*)\]\([^)]*\)", lambda m: m.group(1) or "", h)
    h = "".join(c for c in h if c in " -_" or unicodedata.category(c)[0] in "LN")
    return h.replace(" ", "-")


_anchor_cache = {}


def anchors(path):
    if path not in _anchor_cache:
        seen, result = {}, set()
        for line in strip_code(path.read_text(encoding="utf-8")).split("\n"):
            m = re.match(r"^#{1,6}\s+(.*?)\s*#*\s*$", line)
            if not m:
                continue
            s = slugify(m.group(1))
            n = seen.get(s, 0)
            seen[s] = n + 1
            result.add(s if n == 0 else f"{s}-{n}")
        _anchor_cache[path] = result
    return _anchor_cache[path]


def check_links(md):
    text = strip_code(md.read_text(encoding="utf-8"))
    for i, line in enumerate(text.split("\n"), 1):
        for target in re.findall(r"\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)", line):
            if re.match(r"^[a-z][a-z0-9+.-]*:", target):  # http:, https:, mailto:
                continue
            path_part, _, frag = target.partition("#")
            dest = md if not path_part else (md.parent / path_part).resolve()
            if not dest.exists():
                report("ERROR", rel(md), i, f"broken link: {target}")
            elif frag and dest.suffix == ".md" and frag not in anchors(dest):
                report("ERROR", rel(md), i, f"missing anchor #{frag} in {rel(dest)}")


def body_after_front_matter(text):
    if text.startswith("---\n"):
        end = text.find("\n---\n", 4)
        if end != -1:
            return text[end + 5:], text[4:end]
    return text, None


def check_doc_conventions(md):
    r = rel(md)
    text = md.read_text(encoding="utf-8")
    if r.startswith("docs/decisions/0"):
        return  # ADRs are checked by check_adrs
    body, _ = body_after_front_matter(text)
    first = [l for l in body.split("\n") if l.strip()][:3]
    if r not in HEADER_EXEMPT and not any(l.startswith(">") and "**Status:**" in l for l in first):
        report("WARN", r, 1, "no purpose line with **Status:** right after the title")
    if r not in OPENQ_EXEMPT and not re.search(r"^## Open questions\s*$", body, re.M):
        report("WARN", r, 1, "no '## Open questions' section")


def check_adrs():
    ddir = ROOT / "docs/decisions"
    if not ddir.is_dir():
        return
    index = (ddir / "README.md").read_text(encoding="utf-8")
    rows = dict(re.findall(r"^\|\s*\[(\d{4})\]\([^)]*\)\s*\|[^|]*\|\s*([^|]+?)\s*\|", index, re.M))
    files = sorted(p for p in ddir.glob("*.md") if p.name != "README.md")
    numbers = []
    for f in files:
        m = re.match(r"^(\d{4})-[a-z0-9-]+\.md$", f.name)
        if not m:
            report("ERROR", rel(f), 1, "ADR file name must be NNNN-kebab-title.md")
            continue
        num = m.group(1)
        numbers.append(int(num))
        lines = f.read_text(encoding="utf-8").split("\n")
        if not lines[0].startswith(f"# {num}. "):
            report("ERROR", rel(f), 1, f"title must start with '# {num}. '")
        status = next((l for l in lines[:6] if l.startswith("- **Status:**")), None)
        if not status:
            report("ERROR", rel(f), 2, "missing '- **Status:**' line")
        else:
            value = status.split("**Status:**", 1)[1].strip()
            if not value.startswith(ADR_STATUSES):
                report("ERROR", rel(f), 2, f"unknown status '{value}'")
            idx = rows.get(num)
            if idx is None:
                report("ERROR", "docs/decisions/README.md", 1, f"ADR {num} missing from index table")
            elif value.split()[0].rstrip(",") != idx.split()[0].rstrip(","):
                report("ERROR", "docs/decisions/README.md", 1,
                       f"ADR {num} status mismatch: file '{value}' vs index '{idx}'")
        if not any(re.match(r"^- \*\*Date:\*\* \d{4}-\d{2}-\d{2}$", l) for l in lines[:6]):
            report("ERROR", rel(f), 3, "missing '- **Date:** YYYY-MM-DD' line")
        for sec in ("## Context", "## Decision", "## Alternatives considered", "## Consequences"):
            if sec not in lines:
                report("WARN", rel(f), 1, f"missing section '{sec}'")
    for num in rows:
        if int(num) not in numbers:
            report("ERROR", "docs/decisions/README.md", 1, f"index lists {num} but no file exists")
    if numbers and numbers != list(range(1, max(numbers) + 1)):
        report("WARN", "docs/decisions", 1, f"ADR numbers are not contiguous: {numbers}")


def check_catalog():
    vdir = ROOT / "docs/game/variants"
    readme = vdir / "README.md"
    if not readme.exists():
        return
    text = readme.read_text(encoding="utf-8")
    rows = re.findall(r"^\|[^|]*\|\s*`([a-z]{2,4}\.[a-z0-9-]+)`\s*\|[^|]*\|([^|]*)\|\s*(\w+)\s*\|([^|]*)\|", text, re.M)
    catalog = {vid: (default.strip(), status, spec.strip()) for vid, default, status, spec in rows}
    defaults = {}
    for vid, (default, status, spec) in catalog.items():
        if default:
            defaults.setdefault(vid.split(".")[0], []).append(vid)
        if status in ("researched", "implemented") and not spec:
            report("ERROR", rel(readme), 1, f"{vid} is '{status}' but has no spec link")
        if status == "lead" and spec:
            report("WARN", rel(readme), 1, f"{vid} is 'lead' but links a spec")
    for cc, ids in defaults.items():
        if len(ids) > 1:
            report("ERROR", rel(readme), 1, f"country '{cc}' has {len(ids)} defaults: {ids}")
    for spec in vdir.glob("*/*.md"):
        _, fm = body_after_front_matter(spec.read_text(encoding="utf-8"))
        if fm is None:
            report("ERROR", rel(spec), 1, "variant spec has no YAML front matter")
            continue
        m = re.search(r"^id:\s*([\w.-]+)", fm, re.M)
        vid = m.group(1) if m else None
        if vid not in catalog:
            report("ERROR", rel(spec), 1, f"spec id '{vid}' not in catalog")
            continue
        if spec.relative_to(vdir).as_posix() not in catalog[vid][2]:
            report("ERROR", rel(readme), 1, f"catalog row for {vid} doesn't link {spec.relative_to(vdir)}")
        st = re.search(r"^status:\s*(\w+)", fm, re.M)
        if st and st.group(1) != catalog[vid][1]:
            report("ERROR", rel(spec), 1, f"status '{st.group(1)}' differs from catalog '{catalog[vid][1]}'")
        is_default = re.search(r"^default_for_country:\s*true", fm, re.M) is not None
        if is_default != bool(catalog[vid][0]):
            report("ERROR", rel(spec), 1, "default_for_country disagrees with the catalog star")


def check_index():
    """Every doc except individual ADRs (indexed in decisions/README.md) is linked from docs/README.md."""
    index = ROOT / "docs/README.md"
    if not index.exists():
        return
    linked = set()
    for target in re.findall(r"\]\(([^)#\s]+)", strip_code(index.read_text(encoding="utf-8"))):
        linked.add((index.parent / target).resolve())
    for md in sorted((ROOT / "docs").rglob("*.md")):
        r = rel(md)
        if md == index or r.startswith("docs/decisions/0"):
            continue
        if md.resolve() not in linked:
            report("WARN", "docs/README.md", 1, f"index doesn't link {r}")


def third_party_skills():
    """Skill folders installed with `npx skills` (listed in skills-lock.json). They're
    overwritten on `npx skills update`, so we don't lint them."""
    lock = ROOT / "skills-lock.json"
    if not lock.exists():
        return set()
    import json
    return set(json.loads(lock.read_text(encoding="utf-8")).get("skills", {}))


def main():
    check_index()
    targets = sorted((ROOT / "docs").rglob("*.md")) + [p for p in ROOT.glob("*.md")]
    skills_dir = ROOT / ".claude/skills"
    if skills_dir.is_dir():
        vendored = third_party_skills()
        targets += sorted(p for p in skills_dir.rglob("*.md")
                          if p.relative_to(skills_dir).parts[0] not in vendored)
    for md in targets:
        check_links(md)
        if rel(md).startswith("docs/"):
            check_doc_conventions(md)
    check_adrs()
    check_catalog()
    for level, where, msg in FINDINGS:
        print(f"{level} {where}: {msg}")
    errors = sum(1 for f in FINDINGS if f[0] == "ERROR")
    warns = len(FINDINGS) - errors
    print(f"\n{len(targets)} files checked: {errors} error(s), {warns} warning(s)")
    sys.exit(1 if errors else 0)


if __name__ == "__main__":
    main()
