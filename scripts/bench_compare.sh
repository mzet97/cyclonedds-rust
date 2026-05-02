#!/usr/bin/env bash
# ---------------------------------------------------------------------------
# cyclonedds-rust benchmark runner
# ---------------------------------------------------------------------------
# Runs the built-in criterion benchmarks and prints a summary.
# For cross-impl comparisons (FastDDS / OpenDDS), external harnesses
# must be built and invoked manually — see docs/benchmarks.md.
# ---------------------------------------------------------------------------

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/target/benchmark_results"

mkdir -p "${TARGET_DIR}"

echo "=== cyclonedds-rust Benchmark Suite ==="
echo "Results will be saved to: ${TARGET_DIR}"
echo ""

cd "${PROJECT_ROOT}"

# 1. CDR serialization benchmarks
echo "[1/4] Running CDR serialization benchmarks..."
cargo bench -p cyclonedds-bench --bench cdr -- --noplot > "${TARGET_DIR}/cdr.txt" 2>&1 || true
echo "  -> ${TARGET_DIR}/cdr.txt"

# 2. Latency benchmarks (64B, 1KB, 16KB)
echo "[2/4] Running latency benchmarks..."
cargo bench -p cyclonedds-bench --bench latency -- --noplot > "${TARGET_DIR}/latency.txt" 2>&1 || true
echo "  -> ${TARGET_DIR}/latency.txt"

# 3. Throughput benchmarks
echo "[3/4] Running throughput benchmarks..."
cargo bench -p cyclonedds-bench --bench throughput -- --noplot > "${TARGET_DIR}/throughput.txt" 2>&1 || true
echo "  -> ${TARGET_DIR}/throughput.txt"

# 4. IPC comparison (DDS vs std channel)
echo "[4/4] Running IPC comparison benchmarks..."
cargo bench -p cyclonedds-bench --bench ipc_comparison -- --noplot > "${TARGET_DIR}/ipc_comparison.txt" 2>&1 || true
echo "  -> ${TARGET_DIR}/ipc_comparison.txt"

echo ""
echo "=== Benchmark Summary ==="
echo ""

# Extract and print key numbers
grep -E "(time: |throughput|latency)" "${TARGET_DIR}/latency.txt" | head -20 || true
echo ""
grep -E "(time: |throughput)" "${TARGET_DIR}/throughput.txt" | head -20 || true
echo ""
grep -E "(time: )" "${TARGET_DIR}/ipc_comparison.txt" | head -10 || true

echo ""
echo "For detailed criterion reports, open:"
echo "  ${PROJECT_ROOT}/target/criterion/report/index.html"
echo ""
echo "To compare with FastDDS or OpenDDS, build the external harnesses"
echo "described in docs/benchmarks.md and run them before this script."
