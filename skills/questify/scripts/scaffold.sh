#!/usr/bin/env bash
# Usage: scaffold.sh quest  <quest-id>
#        scaffold.sh module <quest-id> <module-id>
# Creates <root>/quests/<quest-id>/ from templates/quest, or a module crate in
# <root>/quests/<quest-id>/modules/<module-id>/ from templates/module. Either goes last in
# learning order (`order` = last + 1); `questline order` moves it. Never overwrites. Fill in
# the __TITLE__ / __SUMMARY__ placeholders afterwards.
set -euo pipefail
here=$(cd -P "$(dirname "$0")/.." && pwd)
root=$("$here/scripts/questline-root.sh")
id_ok() { [[ $1 =~ ^[a-z0-9]([a-z0-9-]*[a-z0-9])?$ ]] || { echo "'$1' is not a lowercase kebab-case id" >&2; exit 1; }; }
fill() { find "$1" -type f -exec sed -i.bak "${@:2}" {} \; -exec rm -f {}.bak \; ; }
# Last `order` among the given manifests, examples excluded, plus one.
next_order() {
  local order=1 n manifest
  for manifest in "$@"; do
    grep -q '^example = true' "$manifest" && continue
    n=$(sed -n 's/^order *= *\([0-9][0-9]*\).*/\1/p' "$manifest")
    if [[ -n $n ]] && (( n >= order )); then order=$((n + 1)); fi
  done
  echo "$order"
}
shopt -s nullglob

kind=${1:?quest or module}; quest=${2:?quest id}; id_ok "$quest"
qdir=$root/quests/$quest
case $kind in
  quest)
    [[ -e $qdir ]] && { echo "refusing to overwrite $qdir" >&2; exit 1; }
    order=$(next_order "$root"/quests/*/quest.toml)
    cp -R "$here/templates/quest" "$qdir"
    rm -f "$qdir/modules/.gitkeep"
    fill "$qdir" -e "s/__QUEST__/$quest/g" -e "s/__ORDER__/$order/g"
    echo "scaffolded quest $qdir (order $order). Next: scaffold.sh module $quest <module-id>" ;;
  module)
    module=${3:?module id}; id_ok "$module"
    [[ -f $qdir/quest.toml ]] || { echo "no quest at $qdir: scaffold.sh quest $quest first" >&2; exit 1; }
    mdir=$qdir/modules/$module
    [[ -e $mdir ]] && { echo "refusing to overwrite $mdir" >&2; exit 1; }
    order=$(next_order "$qdir"/modules/*/module.toml)
    cp -R "$here/templates/module" "$mdir"
    fill "$mdir" -e "s/__QUEST__/$quest/g" -e "s/__CRATE__/${module//-/_}/g" -e "s/__PACKAGE__/$module/g" -e "s/__ORDER__/$order/g"
    chmod +x "$mdir/scripts/fetch-data.sh"
    echo "scaffolded module $mdir (order $order). Next: write lessons, then cargo run -- all" ;;
  *) echo "usage: scaffold.sh quest <quest-id> | module <quest-id> <module-id>" >&2; exit 1 ;;
esac
