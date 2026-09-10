//! Lossless `u64`/`i64` <-> JS `BigInt` mapping (Phase H).
//!
//! A JS `Number` is an f64 and cannot represent integers beyond 2^53
//! exactly, so every 64-bit IDL integer (`unsigned long long` / `long
//! long`, keys included) MUST cross the wasm boundary as a JS `BigInt`,
//! never as `Number`. This module owns the portable half of that contract:
//! exact decimal-string conversions shared by the wasm32 `BigInt` exports
//! ([`crate::js`]) and by host tests.
//!
//! Mapping (both directions, total, never panics):
//!
//! - `u64` <-> decimal string <-> `BigInt` (range-checked back;
//!   out-of-range input is a typed error, never wrap-around).
//! - `i64` <-> decimal string <-> `BigInt` (same).
//!
//! On wasm32 the string is only an intermediate representation inside the
//! exported function; JS callers see `bigint` (see `js/index.d.ts`).

use crate::{WasmDdsError, WasmDdsResult};

/// Maximum exactly-representable integer in a JS `Number` (2^53 - 1).
pub const MAX_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;

/// Exact decimal rendering of a `u64` (what `BigInt(s)` parses back).
pub fn u64_to_decimal_str(v: u64) -> String {
    v.to_string()
}

/// Parse a decimal string produced by [`u64_to_decimal_str`] (or by
/// `BigInt.prototype.toString()`) back into a `u64`. Rejects empty
/// strings, signs, whitespace, non-digits, and values above `u64::MAX`
/// with a typed error.
pub fn decimal_str_to_u64(s: &str) -> WasmDdsResult<u64> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WasmDdsError::Serialization(format!(
            "invalid u64 decimal string: {s:?}"
        )));
    }
    // Strip leading zeros so the length check below is sound ("000" is 0).
    let t = s.trim_start_matches('0');
    let t = if t.is_empty() { "0" } else { t };
    // u64::MAX is 20 digits; anything longer is out of range by inspection.
    if t.len() > u64::MAX.to_string().len() {
        return Err(WasmDdsError::Serialization(format!(
            "u64 decimal out of range: {s:?}"
        )));
    }
    t.parse::<u64>()
        .map_err(|_| WasmDdsError::Serialization(format!("u64 decimal out of range: {s:?}")))
}

/// Exact decimal rendering of an `i64`.
pub fn i64_to_decimal_str(v: i64) -> String {
    v.to_string()
}

/// Parse a decimal string back into an `i64`. Accepts one optional
/// leading `-`; rejects everything else with a typed error.
pub fn decimal_str_to_i64(s: &str) -> WasmDdsResult<i64> {
    if s.is_empty() {
        return Err(WasmDdsError::Serialization(
            "invalid i64 decimal string: \"\"".into(),
        ));
    }
    let (neg, digits) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(WasmDdsError::Serialization(format!(
            "invalid i64 decimal string: {s:?}"
        )));
    }
    let t = digits.trim_start_matches('0');
    let t = if t.is_empty() { "0" } else { t };
    let limit = if neg {
        // |i64::MIN| = 9223372036854775808 (20 digits).
        "9223372036854775808"
    } else {
        "9223372036854775807"
    };
    if t.len() > limit.len() || (t.len() == limit.len() && t > limit) {
        return Err(WasmDdsError::Serialization(format!(
            "i64 decimal out of range: {s:?}"
        )));
    }
    s.parse::<i64>()
        .map_err(|_| WasmDdsError::Serialization(format!("i64 decimal out of range: {s:?}")))
}

/// True when a `u64` value would lose precision as a JS `Number`.
/// Values returning true MUST be transported as `BigInt`.
pub fn u64_needs_bigint(v: u64) -> bool {
    v > MAX_SAFE_INTEGER_U64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_extremes_roundtrip_exactly() {
        for v in [
            0u64,
            1,
            MAX_SAFE_INTEGER_U64,
            MAX_SAFE_INTEGER_U64 + 1,
            u64::MAX,
        ] {
            let s = u64_to_decimal_str(v);
            assert_eq!(decimal_str_to_u64(&s).unwrap(), v, "v={v}");
        }
        // u64::MAX decimal is exact (a JS Number would round it to 2^64).
        assert_eq!(u64_to_decimal_str(u64::MAX), "18446744073709551615");
    }

    #[test]
    fn i64_extremes_roundtrip_exactly() {
        for v in [0i64, -1, 1, i64::MIN, i64::MAX] {
            let s = i64_to_decimal_str(v);
            assert_eq!(decimal_str_to_i64(&s).unwrap(), v, "v={v}");
        }
        assert_eq!(i64_to_decimal_str(i64::MIN), "-9223372036854775808");
    }

    #[test]
    fn malformed_and_out_of_range_are_typed_errors() {
        for bad in ["", " 12", "12 ", "+12", "0x10", "1.5", "abc", "-", "--1"] {
            assert!(decimal_str_to_u64(bad).is_err(), "u64 must reject {bad:?}");
            assert!(decimal_str_to_i64(bad).is_err(), "i64 must reject {bad:?}");
        }
        // One past each bound.
        assert!(decimal_str_to_u64("18446744073709551616").is_err());
        assert!(decimal_str_to_i64("9223372036854775808").is_err());
        assert!(decimal_str_to_i64("-9223372036854775809").is_err());
        // Negative is never a u64.
        assert!(decimal_str_to_u64("-1").is_err());
        // Leading zeros are harmless and exact.
        assert_eq!(decimal_str_to_u64("00042").unwrap(), 42);
        assert_eq!(decimal_str_to_i64("-00042").unwrap(), -42);
    }

    #[test]
    fn overlong_digit_strings_rejected_by_length_before_parse() {
        // 21+ digits can never fit u64: rejected by the length guard, and
        // the input is echoed back for diagnosis.
        for bad in ["1000000000000000000000", "99999999999999999999999"] {
            let err = decimal_str_to_u64(bad).unwrap_err().to_string();
            assert!(err.contains("out of range"), "unexpected: {err}");
            assert!(err.contains(bad), "input echoed: {err}");
        }
    }

    #[test]
    fn precision_boundary_matches_js_max_safe_integer() {
        assert!(!u64_needs_bigint(MAX_SAFE_INTEGER_U64));
        assert!(u64_needs_bigint(MAX_SAFE_INTEGER_U64 + 1));
        // i64 values whose magnitude exceeds 2^53 need BigInt too.
        assert!(u64_needs_bigint(i64::MAX as u64));
    }
}
