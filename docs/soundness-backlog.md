# Soundness backlog

Everything still open, with the fix for each item. Nothing curated out.

Baseline: `main @ HEAD` · version `3.0.0-alpha.1` · CI 6/6 green ·
316 tests on each of linux/windows/macos, 17 more under the ASan job.

`v2.0.4` is tagged at `f2ef9e6` and **not** published.

Estimated ~4 days remaining. The API breaks are done; what is left is verification and debt.

---

## A. Soundness still open

A1 was the root and is closed; A2 and A7 went with it and A3 is half closed. What remains
here is small.

### A3 — WaitSet::wait_async is not cancellable · Low · ~0.5d · half closed

The safety half went with A1: the stream's `WaitSet` now holds an `Arc` of the reader, so
a `spawn_blocking` wait can no longer sit on an entity someone else deleted.

What remains is cancellation itself. `spawn_blocking` tasks cannot be cancelled, so
dropping the future still leaves the thread waiting until the waitset triggers or the
timeout expires. It is now merely wasteful rather than dangerous.

**Fix.** Expose `dds_waitset_set_trigger` and call it when the stream is dropped, so the
wait wakes immediately instead of hanging on to a thread.

### A5 — Remaining panics · Low/Medium · ~0.5d · downgraded

A4 (fallible `clone_out`) landed in `62b1afd` and closed the union-discriminator panic,
which was the remotely reachable one. What A5 listed alongside it did **not** ride along,
and on re-reading, most of it is weaker than the original entry claimed:

| Site | Original claim | What holds today |
|---|---|---|
| `sequence.rs:85` `DdsSequence::clone_from_raw` | reachable via `clone_out` | `from_slice` only fails on `dds_alloc` returning null or a `len × size` overflow. `ddsrt_malloc` aborts on OOM, so the null branch is dead. **Effectively unreachable.** |
| `string.rs:48` `Default` | reachable | same `dds_alloc` argument. **Effectively unreachable.** |
| `string.rs:58` `Clone` | reachable | `dds_string_dup` → `ddsrt_malloc`. **Effectively unreachable.** |
| `sequence.rs:224` `DdsBoundedSequence::clone_from_raw` | not listed | `from_slice` returns `BadParameter` when `len > N`, and `len` comes from the native sample. This is the one with a live argument — **argued, not demonstrated**: CycloneDDS enforces the bound during deserialization via the `BSQ` ops, so it may not be reachable in practice. |
| `dynamic_type.rs` (7 sites) | API misuse on the user's thread | unchanged, outside any trampoline. Still the honest ones to convert. |

**Fix.** Make `dynamic_type.rs`'s builder return `DdsResult` — an API misuse should be
`Err`, not a panic. For the bounded sequence, either demonstrate reachability with a peer
that oversends, or record that the C side enforces it and leave the `expect`.

**Do not** repeat the original framing. It said "reachable via `clone_out`" for three sites
whose failure branch cannot be taken.

### A6 — ABI probe · Medium · mostly closed in `16d1e4a` · ~0.5d left

Done: all 11 `dds_*_status_t` structs that `entity.rs` reads by value, plus `dds_guid_t`,
are measured by the probe and asserted in `sys/src/lib.rs`. Verified by deliberately
corrupting a probe constant — the build fails naming the type.

Still open: the `SerdataHeader`/`SerdataOps` structs this crate hand-declares. Reaching
`ddsi_serdata.h` from the probe pulls in a chain of internal ddsi headers
(`dds/cdr/dds_cdrstream.h` onward) the probe's include set does not resolve. They remain
pinned only by the vendored header they were read from — and they are exactly what the
2.0.4 vtable fix depended on.

Also still open: the `abi/<triple>.rs` snapshots, which exist for no target (D8).

### A8 — DdsEntity::entity() is public · Low · decision, not code

The review suggested `pub(crate)`. Not done: it is required for FFI interop and the
`from_entities` constructors depend on it.

**Fix.** Keep it public, document it as an escape hatch and mark the raw constructors
`#[doc(hidden)]` — or do nothing and record the decision. Recommend the latter: closing this
pushes FFI users into `unsafe` for no real gain.

### Closed since the last revision of this document

- **A1, A2, A7 — owned parents**. Every entity holds an `Arc<OwnedEntity>` for each
  ancestor, so a parent cannot be deleted while a child is alive and declaration order no
  longer matters. `Publisher`/`Subscriber`/`WaitSet`/`GuardCondition` take
  `&DomainParticipant`; `ReadCondition`/`QueryCondition` take `&DataReader<T>` (A7, and F2
  answers itself). `parent_ownership.rs` reproduced the defect first: four entities that
  escape their participant's scope returned `PreconditionNotMet` before the change.

  **Severity corrected while fixing it.** A1 claimed a recycled handle destroys the wrong
  entity. `dds_handle_create` (`dds_handles.c:116`) draws handles uniformly at random from
  ~2.1e9 values and resolves them through a hash table, so a stale handle is almost always
  absent and returns an error; hitting a live entity needs that exact value redrawn, ~1 in
  2.1e9 per creation. The routine defect was the silent error returns, not corruption.
- **A4 — fallible `clone_out`** · `62b1afd`. `DdsType::clone_out` returns `DdsResult<Self>`.
  Writing its first test uncovered a second defect: `DdsUnionDerive` interpolated a
  macro-time flag into a runtime `if`, so the union derive had never compiled for any
  non-String case — an advertised README feature with zero coverage.
- **A6 (status structs)** · `16d1e4a`, above.
- **D1 — the `Qos` Send/Sync justification** · `16d1e4a`. It claimed the type is
  "immutable after construction"; `set_property` exists. The impl is sound for a different
  reason (every mutation takes `&mut self`).

---

## B. Audit — complete

All nine modules have been read. Six defects, all with before/after proof except where
noted. Recording what each module produced, because "not audited" and "audited, clean" are
very different states to hand over.

| Module | Lines | Outcome |
|---|---:|---|
| `qos.rs` | 1,631 | Clean beyond the `data_representation` leak fixed earlier. All 9 allocating getters re-checked against the C. |
| `xtypes.rs` | 1,170 | **2 defects.** `TopicDescriptor` double-free on `Clone` (`c191bee`); `adr_step` dropped members in `parse_type` (`0a3db00`). |
| `dynamic_value.rs` | 1,186 | Nothing found. |
| `dynamic_type.rs` | 1,071 | 7 panic sites, see A5. No memory defect. |
| `type_discovery.rs` | 976 | **3 defects** (`a2bfb2c`): three more copies of the bad width table; `write_value_to_native` naming fields by word position; native buffer leaked on an early return. |
| `request_reply.rs` | 265 | Nothing found. |
| `security.rs` | 205 | Nothing found. |
| `participant_pool.rs` | 184 | Nothing found. 8 `lock().unwrap()` remain — poisoning propagates a panic, which is the standard trade-off, not a defect. |
| `serde_sample.rs` | 138 | **1 defect** (`9881b38`): `Native = Self` handed a Rust `Vec` to C as a `dds_sequence_t`. Reproduced as `STATUS_STACK_BUFFER_OVERRUN`. |

> The five "nothing found" rows come from the working session, not from a commit — unlike
> `qos.rs` and `xtypes.rs`, whose results were written up in `174e908`. If you want that
> level of record for them too, it has to be written; do not assume it exists.

### B+ — ops() bytecode beyond the scanner · High · ~1d · open

The derive's scanner is gone and the five copies of the width table are down to one, but
the rest of the generation was **not** audited: `OP_KOF` key patching, union `JEQ4` entries,
and `TYPE_EXT` offsets nested more than one level deep. It is the least verifiable part of
the project and an error here corrupts serialization for any type.

**Fix.** Differential test against the C `idlc`: compile the same IDL through both paths and
compare the ops arrays word by word. The only way to verify this without relying on human
reading — and the width-table episode is the argument for why reading is not enough.

### B++ — Vec\<Composite\> with nested heap fields · Medium · ~0.5d · open

Gap the code itself admits at `cyclonedds/src/writer.rs:296-306`: the derive uses the inner
composite type directly as the `DdsSequence` element instead of `<Inner as DdsType>::Native`.
Correct only while `Inner` has no `String`/`Vec` of its own.

**Fix.** Apply the `Native` translation recursively in the derive. Test:
`struct Outer { items: Vec<Inner> }` with `Inner { name: String }`, under ASan.

### B+++ — SerdeSample::type_name() is the same for every T · Medium · open

`type_name()` used `concat!("SerdeSample<", stringify!(T), ">")`, which expands to the
literal `"T"` — so every `SerdeSample<X>` announced the same DDS type name and unrelated
payload types matched each other on the wire. `9881b38` changed the string to plain
`"SerdeSample"`, which is honest about being type-agnostic but does not fix the matching.

Left open deliberately rather than guessed at. A correct fix needs a name **stable across
peers and across compilations**; `std::any::type_name` is neither (no stability guarantee,
and it differs by crate path). Candidates: a hash of the `postcard` schema, or an explicit
name supplied by the user at construction.

---

## C. Release on hold

| Item | State | Action |
|---|---|---|
| tag `v2.0.4` | **created**, at `f2ef9e6` | done |
| publish 2.0.4 to crates.io | not done, 9 crates | build from the **tag**, not `main` (which is 3.0.0-alpha.1). Order: `-src` → `-sys` → `-derive` → `cyclonedds` → `-build` → `-idlc` → `-cli` → `cargo-cyclonedds` → `-wasm`. **Irreversible** — needs explicit approval |
| `cyclonedds-src` | 1.0.1 | check whether the submodule moved since 1.0.1; bump if so |
| `cyclonedds-rust-sys` | 1.1.1 on `main` | the A6 probe changed `sys/src/lib.rs`; needs a bump before the 3.0.0 publish |
| `[Unreleased]` | 3.0.0-alpha.1, three breaking changes | becomes 3.0.0 once A1 lands |

---

## D. Quality and debt

None of this is soundness. All of it is cheap and reduces recurring friction.

| # | What | Action |
|---|---|---|
| ~~D1~~ | ~~`Qos` safety comment~~ | done in `16d1e4a` |
| D2 | ASan job is still `continue-on-error` (`.github/workflows/ci.yml:70`); its output has never been read | read the log; promote to blocking if stable |
| D3 | Trivy: CHANGELOG 2.0.2 records that CVE-by-CVE suppression is unsustainable | purge Perl/gzip from the final stage or move to distroless |
| D4 | `._ROADMAP_v5.md` deletion uncommitted, predates this work | commit or restore — owner's call |
| D5 | 8 files in `docs/` never checked against the current API: `qos-reference`, `security-guide`, `benchmarks`, `fuzzing`, `faq`, `async-patterns`, `security-production`, `architecture`. Now overdue three times: the typed-constructor break, fallible `clone_out`, and owned parents all changed example code | same sweep already done on the other six — but see F1 first |
| D6 | `cyclonedds-bench` never run; dropping `spawn_blocking` should have cut latency and it was not measured | run before/after and record |
| D7 | `fuzz/` never executed | run the existing targets; consider a new one for `clone_out` |
| D8 | `abi/<triple>.rs` snapshots exist for no target — cross-compilation fails by design | generate for the supported targets |

---

## E. Unexplained

Two observed behaviours with no explanation. Recorded so they do not become folklore.

### E1 — SIGSEGV in tests/qos.rs under coverage · Medium

Happened once, on `a1be1ca9`, under `cargo llvm-cov`. Did not reproduce across 15 isolated
runs, the local suite, or any CI commit since — the Code Coverage job has been green on
every commit through `16d1e4a`. The test exercises `qos.data_representation()` — exactly
what was changed — but the crash came before any test reported.

**Action.** Do not treat as resolved. If it returns, run `tests/qos.rs` under ASan in
isolation and inspect `dds_qget_data_representation`. A `slice` stays alive (though unused)
past the `dds_free` in that function — a dangling reference, technically UB, and suspect
number one.

### E2 — async-stream will not expand on local stable · Low

`async_stream::stream! { yield 1; }` fails in a two-line crate on rustc 1.95 and works on
1.85.0 and nightly. Ruled out: `proc-macro2`/`syn`/`quote` versions, incremental cache,
target dir, sandbox. CI is unaffected.

**Action.** Local environment only, worked around with `+1.85.0`. If it becomes annoying:
`rustup update` and retest; if it persists it is a bug to report upstream. Worth the effort
only if development moves to stable.

---

## F. Needs the maintainer

### F1 — The READMEs you updated · High · still blocking

They do not appear in this repository. The root README and 5 files in `docs/` had their
examples rewritten for the typed-constructor break, and `clone_out` returning
`DdsResult<Self>` has since invalidated more example code. If your version lives elsewhere
the conflict is now larger than it was.

**Need to know.** Where they are. If they were in another `Z:\tese` project there is no
conflict. If they were here, say so before documentation is touched again. D5 waits on this.

### ~~F2 — Typed Publisher / Subscriber / WaitSet?~~ · answered by A1

They take `&DomainParticipant` now. A1 absorbed the question, as expected.

---

## Execution order

Each phase ends verifiable and committable.

1. **ops() differential test** — the largest unverified surface left, and the one where
   reading has already proven insufficient. `B+` · ~1d
2. **Nested composites and the remaining panics** — `B++` and A5's `dynamic_type.rs`
   builder. `B++, A5, A3` · ~1.5d
3. **SerdeSample type naming** — needs a design decision on what a stable name is, not just
   code. `B+++` · ~0.5d
4. **ABI snapshots** — the `SerdataHeader`/`SerdataOps` probe gap and the cross-compile
   snapshots. `A6, D8` · ~1d
5. **Release 3.0.0** — consolidate `[Unreleased]`, write the 2.x → 3.0 migration guide,
   bump `-sys`, decide A8, tag and publish. `C, A8` · ~0.5d
6. **Debt and measurement** — D2 through D8. Blocks nothing, but D2 and D3 cut recurring
   friction and D6 measures a gain that is currently only a claim. `D2–D8` · ~1.5d

**Publishing 2.0.4 (C) is orthogonal** and can happen at any point — it builds from the tag,
not from `main`. **E1 and F1 sit outside the sequence:** E1 is watchfulness, act only if it
recurs; F1 blocks D5 and any further documentation work, and depends entirely on the
maintainer.
