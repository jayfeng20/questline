#!/usr/bin/env bash
# Prints the questline root (the directory holding quests/): $QUESTLINE_ROOT, else the checkout
# this skill lives in (resolving the ~/.claude/skills/learn symlink).
set -euo pipefail
if [[ -n ${QUESTLINE_ROOT:-} ]]; then
  root=$(cd -P "$QUESTLINE_ROOT" && pwd)
else
  root=$(cd -P "$(dirname "$0")/../../.." && pwd)
fi
[[ -d $root/quests ]] || { echo "no quests/ in $root: set QUESTLINE_ROOT to your questline checkout" >&2; exit 1; }
echo "$root"
