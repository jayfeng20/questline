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

# NYC TLC green taxi trips, January 2024 (1.3 MB, 56,551 rows).
fetch "https://d37ci6vzurychx.cloudfront.net/trip-data/green_tripdata_2024-01.parquet" green_tripdata_2024-01.parquet
if [[ ${1:-} == --large ]]; then
  : # fetch "https://example.org/big.parquet" big.parquet
fi
