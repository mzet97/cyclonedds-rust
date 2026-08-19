#!/usr/bin/env bash
set -euo pipefail

root="$(git rev-parse --show-toplevel)"
workflows="$root/.github/workflows"
security="$workflows/security.yml"

fail() {
  printf 'security workflow check: %s\n' "$1" >&2
  exit 1
}

test -f "$security" || fail "missing .github/workflows/security.yml"

while IFS= read -r use; do
  ref="${use##*@}"
  [[ "$ref" =~ ^[0-9a-f]{40}$ ]] || fail "mutable action reference: $use"
done < <(grep -RhoE 'uses:[[:space:]]+[^[:space:]]+@[^[:space:]]+' "$workflows" | sed 's/^uses:[[:space:]]*//')

cache_uses="$(grep -RhoE 'uses:[[:space:]]+Swatinem/rust-cache@' "$workflows" | wc -l)"
uncached_targets="$(grep -RhoE 'cache-targets:[[:space:]]+false' "$workflows" | wc -l)"
[[ "$cache_uses" -eq "$uncached_targets" ]] || fail "every Rust cache must exclude native target artifacts"

grep -Eq '^permissions:[[:space:]]*$' "$security" || fail "missing top-level permissions"
grep -Eq 'contents:[[:space:]]+read' "$security" || fail "contents permission is not read-only"

required=(
  'cargo fmt --all -- --check'
  'cargo clippy --workspace --all-targets --all-features --locked -- -D warnings'
  'cargo test --workspace --all-features --locked'
  'cargo test --doc --workspace --exclude cyclonedds-rust-sys --all-features --locked'
  'cargo check -p cyclonedds --no-default-features --features no_std --locked'
  'cargo \+1\.85\.0'
  'cargo audit'
  'cargo deny check'
  'cargo-audit@0\.22\.2,cargo-deny@0\.20\.2'
  'check-unsafe-contracts.sh'
  'miri-strict-provenance'
  'miri-tree-borrows'
  'miri-disable-isolation'
  'dynamic_type_builder'
  'dynamic_cdr_roundtrip'
  'dynamic_schema_publish'
  'loan_heap_fields'
  'listener_panic_barrier'
  'x86_64-pc-windows-msvc'
  "hashFiles\('Cargo\.lock'\)"
  'cache-targets:[[:space:]]+false'
)

for pattern in "${required[@]}"; do
  grep -Eq -- "$pattern" "$security" || fail "missing required gate: $pattern"
done

grep -Rqs 'fail_ci_if_error:[[:space:]]*true' "$workflows" || fail "Codecov is not blocking"
! grep -Rqs 'continue-on-error:[[:space:]]*true' "$workflows" || fail "security checks may not continue on error"

printf 'security workflow check: PASS\n'
