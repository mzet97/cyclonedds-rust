# FFI Gap Matrix — `cyclonedds-sys` vs CycloneDDS public C surface

Date: 2026-04-20
Workspace: `/Users/zeitune/Documents/tese/cyclonedds-rust/cyclonedds-rust`
CycloneDDS source scanned: `/Users/zeitune/Documents/tese/cyclonedds/src/core/ddsc/include/dds`
Bindings scanned: `target/debug/build/cyclonedds-sys-*/out/bindings.rs`

## Summary
The current `cyclonedds-sys` binding exposes the main public CycloneDDS function groups required by downstream Rust code.

After making `wrapper.h` explicit for the public header set and regenerating bindings, the audited public headers in this document now have **zero remaining public-function gaps**.

## Classification Rules
- **Public-covered**: public exported functions declared in CycloneDDS headers and present in the generated bindings.
- **Public-gap**: public exported functions declared in public headers but missing from generated bindings.
- **Alias/macro false positive**: symbol names appearing in comments/macros or macro aliases rather than standalone exported C functions.
- **Internal/advanced review**: symbols in internal/private/inline-heavy headers that need a deliberate policy decision before inclusion.

## Coverage Table

| Header group | Header | `dds_*` refs scanned | Status | Notes |
| --- | --- | ---: | --- | --- |
| Core DDS API | `dds/dds.h` | 132 | Covered with 1 false positive | `dds_return_t` is a type, not a function |
| Listener API | `dds/ddsc/dds_public_listener.h` | 57 | Covered | Full listener setter/getter raw FFI is present |
| QoS API | `dds/ddsc/dds_public_qos.h` | 73 | Covered | Includes getters/setters/prop APIs |
| QoS provider | `dds/ddsc/dds_public_qos_provider.h` | 4 | Covered | Raw provider functions are present |
| QoS defs convenience surface | `dds/ddsc/dds_public_qosdefs.h` | 24 | Covered except macro aliases | `dds_qset_property` and `dds_qset_type_consistency_enforcements` map to exported functions with different names |
| Status API | `dds/ddsc/dds_public_status.h` | 11 | Covered | Status getters present |
| Loan API | `dds/ddsc/dds_public_loan_api.h` | 6 | Covered | Includes shared-memory availability and loan functions |
| Dynamic type API | `dds/ddsc/dds_public_dynamic_type.h` | 17 | Covered | Raw dynamic type functions present |
| Allocation API | `dds/ddsc/dds_public_alloc.h` | 8 | Covered | Raw alloc/free helpers present |
| Statistics API | `dds/ddsc/dds_statistics.h` | 4 | **Public gap** | Missing from generated bindings |

## Public Gaps Status

At the time of the latest audit, there are **no remaining true public-function gaps** in the audited public headers:
- `dds/dds.h`
- `dds/ddsc/dds_public_alloc.h`
- `dds/ddsc/dds_public_dynamic_type.h`
- `dds/ddsc/dds_public_listener.h`
- `dds/ddsc/dds_public_loan_api.h`
- `dds/ddsc/dds_public_qos.h`
- `dds/ddsc/dds_public_qos_provider.h`
- `dds/ddsc/dds_public_status.h`
- `dds/ddsc/dds_statistics.h`

The earlier statistics gap was closed by explicitly including `dds/ddsc/dds_statistics.h` in `cyclonedds-sys/wrapper.h`.

## Alias / Macro False Positives

### `dds_return_t`
- Reported during text scan from `dds.h` and some internal headers.
- This is a type alias, not a callable symbol.
- No FFI work required.

### `dds_qset_property`
- Appears in `dds_public_qosdefs.h` comments / alias surface.
- Actual exported function in bindings is `dds_qset_prop`.
- Safe-layer ergonomics may still want a Rust alias method later.

### `dds_qset_type_consistency_enforcements`
- Alias surface in `dds_public_qosdefs.h`.
- Actual exported function in bindings is `dds_qset_type_consistency`.
- No raw FFI gap.

### `dds_err_file_id`, `dds_err_line`, `dds_err_nr`
- Declared as macro utilities in `dds_public_error.h`.
- Not standalone exported C functions.
- If desired, expose as Rust helper logic rather than bindgen FFI.

## Internal / Advanced Headers Requiring Policy Decision
These headers expose useful but non-core or internal surfaces. They should be classified explicitly before inclusion in `cyclonedds-sys` public promises.

### `dds/ddsc/dds_internal_api.h`
Symbols found:
- `dds_cdrstream_desc_from_topic_desc`
- `dds_create_participant_guid`
- `dds_create_reader_guid`
- `dds_create_writer_guid`

Assessment:
- Internal/advanced, not first-line safe API.
- Worth evaluating for advanced control/interoperability.

### `dds/ddsc/dds_loaned_sample.h`
Symbols found:
- `dds_loaned_sample_ref`
- `dds_loaned_sample_unref`
- `dds_reader_store_loaned_sample`
- `dds_reader_store_loaned_sample_wr_metadata`

Assessment:
- Advanced loan internals.
- Potentially important for high-performance/PSMX/shared-memory work.

### `dds/ddsc/dds_psmx.h`
Contains PSMX/shared-memory support internals/helpers.

Assessment:
- Relevant to the 100% goal only after a policy decision:
  - either expose as advanced raw FFI,
  - or keep internal and only wrap supported public shared-memory APIs.

### `dds/ddsc/dds_rhc.h`
Contains RHC internals and inline-heavy helpers.

Assessment:
- Likely not part of the stable public promise for the safe crate.
- Needs deliberate inclusion decision.

## Current Wrapper State
`wrapper.h` now explicitly includes the main public header groups instead of relying only on transitive inclusion:
- `dds/dds.h`
- `dds/ddsc/dds_public_alloc.h`
- `dds/ddsc/dds_public_dynamic_type.h`
- `dds/ddsc/dds_public_impl.h`
- `dds/ddsc/dds_public_listener.h`
- `dds/ddsc/dds_public_loan_api.h`
- `dds/ddsc/dds_public_qos.h`
- `dds/ddsc/dds_public_qos_provider.h`
- `dds/ddsc/dds_public_status.h`
- `dds/ddsc/dds_statistics.h`

## Recommendation for Phase P2.2
1. Keep `wrapper.h` explicit for public supported groups.
2. Decide and document policy for:
   - `dds_internal_api.h`
   - `dds_loaned_sample.h`
   - `dds_psmx.h`
   - `dds_rhc.h`
3. Keep a repeatable audit script for header-vs-binding comparison (`tools/audit_ffi_coverage.py`).

## Immediate Conclusion
For the audited public header groups, raw `cyclonedds-sys` coverage is now effectively complete. The remaining FFI decisions are no longer about missing mainstream public DDS functions; they are about whether and how to expose advanced/internal CycloneDDS surfaces such as internal API hooks, PSMX internals, loaned-sample internals, and RHC internals.
