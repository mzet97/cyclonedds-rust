# CycloneDDS-Rust Implementation Plan

## Overview

Create a production-ready Rust binding for Eclipse CycloneDDS C (v0.11.0), following the architectural patterns from CycloneDDS.NET. The project is a workspace with two crates:

- **`cyclonedds-sys`**: Raw FFI bindings (sys crate)
- **`cyclonedds`**: Safe, idiomatic Rust wrapper

### Key Paths

| Resource | Path |
|----------|------|
| CycloneDDS C headers | `/Users/zeitune/Documents/tese/cyclonedds/src/core/ddsc/include/` |
| CycloneDDS ddsrt headers | `/Users/zeitune/Documents/tese/cyclonedds/src/ddsrt/include/` |
| Generated headers (version.h, export.h, config.h, features.h) | `/Users/zeitune/Documents/tese/cyclonedds/build-mac/src/ddsrt/include/` and `build-mac/src/core/include/` |
| Compiled library | `/Users/zeitune/Documents/tese/cyclonedds/build-mac/lib/libddsc.dylib` |
| Rust workspace | `/Users/zeitune/Documents/tese/cyclonedds-rust/cyclonedds-rust/` |
| .NET reference | `/Users/zeitune/Documents/tese/CycloneDds.NET/src/CycloneDDS.Runtime/` |

### Key C API Surface (~70 functions needed)

From `dds.h` (~2600 lines):
- Entity lifecycle: `dds_create_participant`, `dds_create_topic_sertype`, `dds_create_publisher`, `dds_create_subscriber`, `dds_create_writer`, `dds_create_reader`, `dds_delete`
- Read/Write: `dds_write`, `dds_writecdr`, `dds_read`, `dds_readcdr`, `dds_take`, `dds_takecdr`, `dds_return_loan`
- QoS: `dds_create_qos`, `dds_delete_qos`, `dds_qset_*` (~30 functions), `dds_qget_*`
- Listeners: `dds_create_listener`, `dds_delete_listener`, `dds_lset_*` (~15 functions)
- WaitSet: `dds_create_waitset`, `dds_waitset_attach`, `dds_waitset_detach`, `dds_waitset_wait`, `dds_waitset_set_trigger`
- Conditions: `dds_create_readcondition`, `dds_create_querycondition`, `dds_create_guardcondition`
- Instance: `dds_register_instance`, `dds_unregister_instance`, `dds_lookup_instance`, `dds_instance_get_key`
- Status: `dds_get_status_changes`, `dds_get_enabled_status`, `dds_set_enabled_status`
- Serdata: `ddsi_serdata_from_ser_iov`, `ddsi_serdata_to_ser`, `ddsi_serdata_ref`, `ddsi_serdata_unref`
- Topic descriptor: `dds_topic_descriptor_t` (size, alignment, flags, keys, ops, metadata)

---

## Phase 1: Build System & FFI Foundation

**Goal**: `cargo build` succeeds with auto-generated bindings that link to `libddsc.dylib`.

### Steps

1. **Add `bindgen` as build dependency** to `cyclonedds-sys/Cargo.toml`
   - `bindgen = "0.71"` under `[build-dependencies]`
   - Add feature flag `use-bindgen` (default on)

2. **Rewrite `build.rs`** to:
   - Locate `dds.h` and all required include paths
   - Run `bindgen::Builder` to generate `bindings.rs` from `dds/dds.h`
   - Emit correct `cargo:rustc-link-search` for `/Users/zeitune/Documents/tese/cyclonedds/build-mac/lib/`
   - Emit `cargo:rustc-link-lib=dylib=ddsc`
   - Use `CYCLONEDDS_PATH` env var with fallback to hardcoded path

3. **Replace manual bindings** in `cyclonedds-sys/src/lib.rs`:
   - `include!(concat!(env!("OUT_DIR"), "/bindings.rs"));`
   - Keep manual additions only for types bindgen can't handle (if any)

4. **Environment variable support**:
   - `CYCLONEDDS_INCLUDE` — override header search path
   - `CYCLONEDDS_LIB` — override library search path
   - `CYCLONEDDS_STATIC` — link statically (optional)

5. **Install `llvm` via homebrew** if `libclang` not found (bindgen requirement)

### Verification
- `cargo build -p cyclonedds-sys` succeeds
- Generated `bindings.rs` contains `dds_create_participant`, `dds_write`, `dds_read`, `dds_topic_descriptor_t`, etc.

---

## Phase 2: Core Safe Wrapper — Entities & Lifecycle

**Goal**: RAII wrappers for all DDS entities compile and Drop correctly.

### Steps

1. **Update entity types** to use bindgen-generated types (`dds_entity_t` is `i32` in C, not `u32`)
2. **Fix `Topic<T>` creation** — current code calls `dds_create_topic()` with type_name as arg, but the real API uses `dds_create_topic_sertype()` with a `dds_topic_descriptor_t` built from the type
3. **Proper `dds_return_loan`** in `DataReader::read()` and `DataReader::take()` — currently missing, causes memory leaks
4. **Add `dds_create_topic_descriptor`** wrapper that builds the descriptor from Rust type info (preliminary — real serialization comes in Phase 3)
5. **Entity base trait** — `DdsEntity` trait with `fn as_entity(&self) -> dds_entity_t`

### Verification
- `cargo build` succeeds for both crates
- Basic pub/sub example compiles (even if runtime fails due to missing serialization)

---

## Phase 3: Type System & CDR Serialization (Critical Path)

**Goal**: Users can define a `struct` with `#[derive(Serialize, Deserialize)]`, register it as a DDS topic, and read/write real data.

This is the hardest phase. CycloneDDS uses a topic descriptor with an ops array (instruction-based CDR serializer). The .NET binding (`DdsParticipant.MarshalDescriptor<T>()`, lines 293-399) shows the pattern.

### Approach: Serdata Plugin

1. **Define `DdsType` trait** in `cyclonedds` crate:
   ```rust
   pub trait DdsType: Sized + Send + 'static {
       fn type_name() -> &'static str;
       fn topic_descriptor() -> TopicDescriptor;
       fn serialize(&self) -> Vec<u8>;      // to CDR bytes
       fn deserialize(data: &[u8]) -> Self;  // from CDR bytes
       fn key_count() -> usize { 0 }
       fn key_offsets() -> Vec<usize> { vec![] }
   }
   ```

2. **Build `dds_topic_descriptor_t`** from `DdsType`:
   - Ops array construction from type layout (mirroring .NET's `MarshalDescriptor<T>`)
   - Size, alignment, flags from `std::mem::size_of::<T>()` and `std::mem::align_of::<T>()`

3. **CDR serialization**:
   - Option A: Use `cdr` crate for CDR encode/decode
   - Option B: Manual CDR serialization matching CycloneDDS ops format
   - Recommend Option A (`cdr` crate) for simplicity

4. **Serdata plugin implementation**:
   - Implement `ddsi_serdata_ops` callbacks: `to_ser`, `to_ser_iov`, `from_ser`, `from_ser_iov`, `from_keyhash`, `get_size`, `eqkey`, `free`, `print`
   - This bridges between DDS CDR wire format and Rust types

5. **`dds_create_topic_sertype()`** instead of `dds_create_topic()`:
   - Creates topic with custom serialization plugin
   - Requires `ddsi_sertype` struct with ops pointer

### Verification
- Write a `HelloWorld { id: i32, message: String }` example
- Publisher sends, subscriber receives, data matches
- Test with different types: primitives, structs with strings, arrays

---

## Phase 4: QoS Policies

**Goal**: Full QoS builder pattern.

### Steps

1. **`QosBuilder`** struct with builder pattern
2. **Wrap all `dds_qset_*` functions**: reliability, durability, deadline, lifespan, history, resource_limits, transport_priority, ownership, etc.
3. **`Qos` RAII wrapper** — `dds_create_qos()` / `dds_delete_qos()`
4. **Apply QoS** to participant, topic, publisher, subscriber, reader, writer

### Verification
- Create writer with reliable + transient-local QoS
- Create reader with matching QoS
- Verify data persists across late-joining readers

---

## Phase 5: Listeners & Async

**Goal**: Safe listener callbacks and async read/write.

### Steps

1. **Listener wrapper** — `dds_listener_t` with safe Rust closures
2. **Callback types**: `on_data_available`, `on_publication_matched`, `on_subscription_matched`, `on_liveliness_changed`, etc.
3. **WaitSet → tokio bridge**:
   - `WaitSet` wraps `dds_create_waitset`
   - `wait()` returns `async fn` using tokio::task::spawn_blocking
4. **`DataReader::read_async()`** and **`DataReader::take_async()`**

### Verification
- Async subscriber receives data using tokio
- Listener callback fires on data available

---

## Phase 6: Instance Management & Advanced

**Goal**: Full DDS feature coverage.

### Steps

1. **Keyed topics** — `dds_register_instance`, `dds_unregister_instance`, `dds_instance_get_key`
2. **ReadCondition** — `dds_create_readcondition` with sample/view/instance state masks
3. **QueryCondition** — `dds_create_querycondition` with SQL-like filter expressions
4. **Content filter** — `dds_create_contentfiltered_topic`
5. **GuardCondition** — `dds_create_guardcondition`
6. **Loan API** — `dds_loan_sample`, `dds_return_loan` for zero-copy

### Verification
- Keyed topic with instance lifecycle management
- QueryCondition filters correctly
- Loan-based zero-copy read

---

## Phase 7: Idiomatic Rust Ergonomics

**Goal**: Polish for public use.

### Steps

1. **`#[derive(DdsType)]` proc macro** — auto-generate `DdsType` trait impl from struct definition
2. **`Loan<T>` guard type** — RAII wrapper for loaned samples with auto-return
3. **`Sample<T>`** — wraps data + `SampleInfo` with metadata accessors
4. **Documentation** — rustdoc for all public types
5. **Error improvements** — `std::error::Error` impl, more specific error variants
6. **CI/Tests** — integration tests with CycloneDDS installed

### Verification
- `cargo doc` generates clean docs
- `cargo test` passes all unit + integration tests
- Example compiles and runs end-to-end

---

## Risk Assessment

| Phase | Risk | Mitigation |
|-------|------|------------|
| Phase 1 | bindgen can't resolve headers | Use wrapper header with explicit includes; fall back to manual bindings |
| Phase 3 | Serdata plugin is complex C interop | Follow .NET reference closely; test incrementally |
| Phase 3 | CDR serialization mismatches | Use `cdr` crate for wire format; validate with Wireshark |
| Phase 5 | Listener closures require `'static` | Use `Box<dyn Fn>` + `unsafe` trampoline; same pattern as .NET delegate marshalling |
| Phase 7 | Proc macro complexity | Start with `DdsType` trait hand-implementation; add macro later |

## Dependencies (per phase)

```
Phase 1 → Phase 2 → Phase 3 (critical path)
                    ↘ Phase 4 (can parallel with Phase 3)
Phase 2 → Phase 5
Phase 3 → Phase 6
Phase 6 → Phase 7
```
