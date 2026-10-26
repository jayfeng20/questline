#!/usr/bin/env bash
# Everything /questify autosort needs to place a topic: every registered quest's manifest, its
# shared dependency pins, and its modules.
set -euo pipefail
root=$("$(cd -P "$(dirname "$0")" && pwd)/questline-root.sh")
shopt -s nullglob
for q in "$root"/quests/*/; do
  q=${q%/}
  echo "=== quest $(basename "$q")   ($q)"
  grep -E '^(title|summary|order|tags|example) =' "$q/quest.toml" 2>/dev/null | sed 's/^/  /' || echo "  (no quest.toml)"
  echo "  [workspace.dependencies]"
  sed -n '/^\[workspace.dependencies\]/,/^\[/{/^\[/d;/^#/d;/^$/d;s/^/    /;p;}' "$q/Cargo.toml" 2>/dev/null
  echo "  modules (order, id, title):"
  for m in "$q"/modules/*/; do
    m=${m%/}
    printf '    %-4s %-28s %s\n' "$(sed -n 's/^order *= *\([0-9][0-9]*\).*/\1/p' "$m/module.toml" 2>/dev/null)" \
      "$(basename "$m")" "$(sed -n 's/^title = //p' "$m/module.toml" 2>/dev/null)"
  done | sort -n
done
