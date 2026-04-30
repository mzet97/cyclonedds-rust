#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import re
from pathlib import Path


DEFAULT_CYCLONEDDS_ROOT = Path("/Users/zeitune/Documents/tese/cyclonedds")
DEFAULT_RUST_ROOT = Path("/Users/zeitune/Documents/tese/cyclonedds-rust/cyclonedds-rust")

PUBLIC_HEADERS = [
    "dds.h",
    "ddsc/dds_public_alloc.h",
    "ddsc/dds_public_dynamic_type.h",
    "ddsc/dds_public_listener.h",
    "ddsc/dds_public_loan_api.h",
    "ddsc/dds_public_qos.h",
    "ddsc/dds_public_qos_provider.h",
    "ddsc/dds_public_status.h",
    "ddsc/dds_statistics.h",
]

ADVANCED_HEADERS = [
    "ddsc/dds_internal_api.h",
    "ddsc/dds_loaned_sample.h",
    "ddsc/dds_psmx.h",
    "ddsc/dds_rhc.h",
    "ddsc/dds_public_error.h",
]


def extract_dds_symbols(text: str) -> list[str]:
    return sorted(set(re.findall(r"\b(dds_[A-Za-z0-9_]+)\s*\(", text)))


def latest_bindings_file(rust_root: Path) -> Path:
    candidates = list(
        rust_root.glob("target/debug/build/cyclonedds-sys-*/out/bindings.rs")
    )
    if not candidates:
        raise FileNotFoundError("No generated cyclonedds-sys bindings.rs found")
    return max(candidates, key=lambda p: p.stat().st_mtime)


def manual_sys_symbols(rust_root: Path) -> set[str]:
    sys_rs = rust_root / "cyclonedds-sys" / "src" / "lib.rs"
    if not sys_rs.exists():
        return set()
    text = sys_rs.read_text(errors="ignore")
    symbols = set(re.findall(r"\bpub(?: const| unsafe)? fn (dds_[A-Za-z0-9_]+)\(", text))
    symbols |= set(re.findall(r"\bpub (?:type|struct|union) (dds_[A-Za-z0-9_]+)\b", text))
    return symbols


def audit_group(headers_root: Path, bound: set[str], header_names: list[str]) -> list[tuple[str, int, list[str]]]:
    rows: list[tuple[str, int, list[str]]] = []
    for header_name in header_names:
        text = (headers_root / header_name).read_text(errors="ignore")
        header_syms = extract_dds_symbols(text)
        missing = [sym for sym in header_syms if sym not in bound and sym != "dds_return_t"]
        rows.append((header_name, len(header_syms), missing))
    return rows


def print_rows(title: str, rows: list[tuple[str, int, list[str]]]) -> None:
    print(f"\n## {title}")
    for header_name, total, missing in rows:
        print(f"- {header_name}: total={total}, missing={len(missing)}")
        if missing:
            print(f"  missing: {', '.join(missing)}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Audit cyclonedds-sys FFI coverage")
    parser.add_argument("--cyclonedds-root", type=Path, default=DEFAULT_CYCLONEDDS_ROOT)
    parser.add_argument("--rust-root", type=Path, default=DEFAULT_RUST_ROOT)
    args = parser.parse_args()

    headers_root = args.cyclonedds_root / "src/core/ddsc/include/dds"
    bindings_path = latest_bindings_file(args.rust_root)
    bindings_text = bindings_path.read_text(errors="ignore")
    bound = set(re.findall(r"pub fn (dds_[A-Za-z0-9_]+)\(", bindings_text))
    bound |= set(re.findall(r"pub (?:type|struct|union) (dds_[A-Za-z0-9_]+)\b", bindings_text))
    bound |= manual_sys_symbols(args.rust_root)

    public_rows = audit_group(headers_root, bound, PUBLIC_HEADERS)
    advanced_rows = audit_group(headers_root, bound, ADVANCED_HEADERS)

    print(f"Bindings file: {bindings_path}")
    print(f"Bound dds_* functions: {len(bound)}")
    print_rows("Public headers", public_rows)
    print_rows("Advanced/internal headers", advanced_rows)

    total_public_missing = sum(len(missing) for _, _, missing in public_rows)
    if total_public_missing:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
