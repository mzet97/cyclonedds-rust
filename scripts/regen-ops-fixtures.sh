#!/usr/bin/env bash
#
# Print the ops arrays the C idlc generates for tests/idl/ops_reference.idl.
#
# The differential tests in cyclonedds-test-suite assert DdsType::ops() against
# these arrays, transcribed by hand so the expectations stay readable and can be
# expressed in terms of offset_of!/size_of! rather than frozen numbers. Run this
# after changing the reference IDL, or when a new CycloneDDS release lands, and
# reconcile any diff with tests/ops_vs_idlc.rs.
#
# idlc is not built by the normal `cargo build`: cyclonedds-rust-sys configures
# the vendored CycloneDDS with -DBUILD_IDLC=OFF because nothing in the crate
# needs it at run time. This script builds it out of tree.
#
# Usage:
#   scripts/regen-ops-fixtures.sh [build-dir]
#
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
build_dir="${1:-${TMPDIR:-/tmp}/cyclonedds-idlc-build}"
idl_file="$repo_root/cyclonedds-test-suite/tests/idl/ops_reference.idl"
out_dir="$build_dir/generated"

if [ ! -f "$idl_file" ]; then
    echo "reference IDL not found: $idl_file" >&2
    exit 1
fi

echo "==> configuring idlc in $build_dir"
cmake -S "$repo_root/vendor/cyclonedds" -B "$build_dir" \
    -DBUILD_IDLC=ON \
    -DBUILD_TESTING=OFF \
    -DBUILD_EXAMPLES=OFF \
    -DENABLE_SECURITY=OFF \
    -DENABLE_SSL=OFF \
    -DCMAKE_BUILD_TYPE=Release \
    >/dev/null

# libidlc is the C backend idlc dlopen()s; building only the `idlc` target
# produces a binary that cannot generate anything.
echo "==> building idlc and libidlc"
cmake --build "$build_dir" --config Release --target idlc libidlc >/dev/null

idlc_bin="$(find "$build_dir" -name 'idlc' -o -name 'idlc.exe' | grep -v CMakeFiles | head -1)"
if [ -z "$idlc_bin" ]; then
    echo "idlc binary not found under $build_dir" >&2
    exit 1
fi
idlc_dir="$(dirname "$idlc_bin")"

mkdir -p "$out_dir"
cp "$idl_file" "$out_dir/"

echo "==> generating C from $(basename "$idl_file")"
(
    cd "$out_dir"
    # The generator plugin is looked up through the loader path.
    PATH="$idlc_dir:$PATH" LD_LIBRARY_PATH="$idlc_dir:${LD_LIBRARY_PATH:-}" \
        DYLD_LIBRARY_PATH="$idlc_dir:${DYLD_LIBRARY_PATH:-}" \
        "$idlc_bin" "$(basename "$idl_file")"
)

echo
echo "==> idlc $("$idlc_bin" -v 2>&1 | tail -1)"
echo "==> ops arrays"
echo
sed -n '/_ops \[\]/,/^};/p' "$out_dir"/*.c
