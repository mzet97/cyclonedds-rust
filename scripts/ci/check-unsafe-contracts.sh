#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
baseline="$root/scripts/ci/unsafe-inventory.txt"
actual="$(mktemp)"
trap 'rm -f "$actual"' EXIT

cd "$root"
git ls-files -z -- '*.rs' \
  | while IFS= read -r -d '' file; do
      file="${file#./}"
      if grep -qE '\bunsafe\b' "$file"; then
        count="$(grep -nE '\bunsafe\b' "$file" | wc -l)"
        printf '%s %s\n' "$count" "$file"
      fi
    done \
  | sort -k2 > "$actual"

if ! diff -u "$baseline" "$actual"; then
  printf '%s\n' 'unsafe inventory changed; review every new unsafe operation, add a SAFETY contract, and update the inventory intentionally' >&2
  exit 1
fi

printf 'unsafe inventory check: PASS (%s files)\n' "$(wc -l < "$actual")"
