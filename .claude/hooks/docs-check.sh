#!/bin/bash
# PostToolUse hook (matcher: Edit|Write). After Claude edits a Markdown doc, run the
# docs checker and feed back only the errors that concern that file (exit 2 shows
# stderr to Claude). Pre-existing errors elsewhere don't block unrelated edits.
# Requires: jq, python3.
input=$(cat)
file=$(jq -r '.tool_input.file_path // empty' <<<"$input")
root="${CLAUDE_PROJECT_DIR:-$(pwd)}"
case "$file" in
  "$root"/docs/*.md|"$root"/*.md|"$root"/.claude/skills/*.md|"$root"/.claude/skills/*/*.md) ;;
  *) exit 0 ;;
esac
relpath="${file#"$root"/}"

# Errors are reported against these exact paths.
targets="$relpath"
case "$relpath" in
  docs/decisions/*)     targets="$targets docs/decisions/README.md" ;;
  docs/game/variants/*) targets="$targets docs/game/variants/README.md" ;;
esac

# Checker output lines look like: "ERROR path:line: message".
errors=$(python3 "$root/.claude/skills/docs-audit/scripts/check_docs.py" "$root" 2>&1 \
  | awk -v targets="$targets" '
      BEGIN { n = split(targets, t, " "); for (i = 1; i <= n; i++) want[t[i]] = 1 }
      $1 == "ERROR" { split($2, a, ":"); if (a[1] in want) print }')
[ -z "$errors" ] && exit 0
{ echo "Docs check found problems after editing $relpath:"; echo "$errors"; } >&2
exit 2
