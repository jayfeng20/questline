#!/usr/bin/env bash
# Download fixtures into ./data/ (gitignored). Small by default; --large for realistic sizes.
# Document every file in data/README.md (source URL, license, rows/size, which lessons use it).
set -euo pipefail
cd "$(dirname "$0")/../data"

fetch() {
  local url=$1 out=$2
  if [[ -s $out ]]; then echo "ok    $out (already present)"; return; fi
  echo "fetch $url"
  curl -fsSL --retry 3 -o "$out.part" "$url"
  mv "$out.part" "$out"
  echo "ok    $out ($(wc -c <"$out" | tr -d ' ') bytes)"
}

# fetch "https://example.org/small.parquet" small.parquet
if [[ ${1:-} == --large ]]; then
  : # fetch "https://example.org/big.parquet" big.parquet
fi
