#!/usr/bin/env bash
# Everything /learn autosort needs to place a topic: every registered quest's manifest, its
# shared dependency pins, and its modules.
set -euo pipefail
root=$("$(cd -P "$(dirname "$0")" && pwd)/questline-root.sh")
shopt -s nullglob
for q in "$root"/quests/*/; do
  q=${q%/}
  echo "=== quest $(basename "$q")   ($q)"
  grep -E '^(title|summary|tags|example) =' "$q/quest.toml" 2>/dev/null | sed 's/^/  /' || echo "  (no quest.toml)"
  echo "  [workspace.dependencies]"
  sed -n '/^\[workspace.dependencies\]/,/^\[/{/^\[/d;/^#/d;/^$/d;s/^/    /;p;}' "$q/Cargo.toml" 2>/dev/null
  echo "  modules:"
  for m in "$q"/modules/*/; do
    m=${m%/}
    printf '    %-28s %s\n' "$(basename "$m")" "$(sed -n 's/^title = //p' "$m/module.toml" 2>/dev/null)"
  done
done
