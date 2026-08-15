# Soundness backlog

Everything still open, with the fix for each item. Nothing curated out.

Baseline: `main @ HEAD` · version `3.0.0-alpha.1` · 356 tests on each of
linux/windows/macos, 25 under the ASan job (now blocking).

`v2.0.4` is tagged at `f2ef9e6` and **not** published.

The API breaks are done. B+, B++, A3, A5, D2, D6 and D7 are closed; what is left
is the rest of B+ (unions, nested sequences), release work, and three things
that need the maintainer.

## Read this before trusting any C citation in this file

`cyclonedds-rust-sys` builds **`cyclonedds-src` (CycloneDDS 11.0.0)**, not
`vendor/cyclonedds` (**11.0.1**) — `build.rs:241-248` prefers the crate over the
vendor directory. The two differ observably (`dds_stream_normalize` returns
`bool` in 11.0.0 and an enum in 11.0.1), so a line number read out of `vendor/`
does not necessarily describe the library that is linked. Reconciling them is
F3 below.

---

## A. Soundness still open

A1 was the root and is closed; A2, A7 and A3 went with it, and A5 closed as hardening.
What remains here is A6's probe gap and one decision.

### ~~A3 — WaitSet::wait_async is not cancellable~~ · closed

Closed, but **the entry above it was wrong** and is corrected rather than repeated. It
said dropping the future "leaves the thread waiting until the waitset triggers or the
timeout expires". That did not reproduce: deleting a waitset runs
`dds_waitset_interrupt`, which broadcasts the wait condition, and the wait loop rechecks
`dds_handle_is_closed` — so the wait returned as soon as the stream dropped its waitset.
Measured at 0.35s for a stream with a 30-second timeout.

What was actually open was the handle race: the blocking task captured only the raw
`dds_entity_t`. It now holds an `Arc`. Doing *only* that created the hang the entry had
predicted — holding the `Arc` prevents the deletion that was doing the interrupting, and
the same measurement went to 29.7s. So `dds_waitset_set_trigger` was needed after all,
just not for the stated reason: streams attach their waitset to itself and trigger it on
drop. `tests/async_wait_cancellation.rs` measures both halves through runtime shutdown.

### ~~A5 — Remaining panics~~ · closed, as hardening

`dynamic_type.rs`'s seven panic sites now return `DdsResult`. **No failing test was
written and none was possible:** every one is unreachable through the public API —
`DynamicTypeBuilder::new` is private, each constructor whose kind needs a sub-type takes
it as an argument, and the setters take values rather than `Option`s. Recorded as
hardening, not as a fix. `every_public_constructor_builds_without_panicking` pins the
unreachability claim.

Writing that test found a real defect it was not looking for: `DDS_RETCODE_UNSUPPORTED`
(`-2`) was mapped to `DdsError::OutOfMemory`, for which `is_transient()` answers `true`,
so every permanently unsupported operation looked worth retrying. `-12` and `-13` were
also wrong. Fixed with `tests/error_retcode_mapping.rs`.

<details>
<summary>Original A5 entry, kept for the reasoning about the other sites</summary>

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

</details>

Still genuinely open from that table: `DdsBoundedSequence::clone_from_raw` panics when the
native sample's length exceeds the bound, and that length comes off the wire. Argued, not
demonstrated — CycloneDDS enforces the bound during deserialization via the `BSQ` ops, and
`CdrDeserializer` now normalizes before reading, which closes the path this crate controls.
Either demonstrate reachability with a peer that oversends, or record that the C enforces
it and leave the `expect`.

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

### ~~B+ — ops() bytecode beyond the scanner~~ · closed · `683ae33`

The differential test against the C `idlc` was written and it found three defects, none
of which inspection had found. `tests/ops_vs_idlc.rs`, fixtures regenerated by
`scripts/regen-ops-fixtures.sh` from `tests/idl/ops_reference.idl`.

1. **The member after a composite one was never serialized.** `sequence<Struct>`,
   `sequence<Struct, N>` and `Struct[N]` emitted the element's sub-ops inline and wrote a
   constant jump word. Both halves of that word are positions: the high half is the
   distance to the next member's instruction, the low half to the sub-ops. idlc emits
   `(4<<16)+7` where the derive hardcoded `(4<<16)+5`, so "next member" pointed at an
   `RTS`. `struct { long h; sequence<P> items; long tail; }` round-tripped with `tail`
   zeroed; with a key or more members it crashed the process. The one shape that worked is
   a composite member declared *last* — the shape every existing test used.
2. **`#[key]` after a composite member got the wrong ops index.** `keys()` counted the
   inlined child block, so the `KOF` operand pointed into the sub-ops. idlc emits
   `KOF|1, 3` where the derive emitted `9`.
3. A redundant trailing `RTS` on every nested block — unreachable, but it inflated
   `m_nops` and diverged from idlc.

Two differences from idlc are documented rather than matched: idlc appends a `KOF` chain
to `m_ops` where this crate builds one in `Topic::new` from `keys()`, and it shares one
sub-ops block between members of the same element type where the derive emits one each.
Both are valid encodings.

**Not covered.** Union `JEQ4` entries were in the original scope and are still unverified —
`derive_union_impl` builds them with its own arithmetic, which this test does not reach.
`TYPE_EXT` nesting was verified to one level. `nested_sequence_info` (sequence of
sequence, `SUBTYPE_SEQ`) has the same inline shape the four composite cases had and was
**not** examined; it is the most likely place for a fourth instance of defect 1.

### ~~B++ — Vec\<Composite\> with nested heap fields~~ · closed · `683ae33`

Wider than recorded. The derive used the inner Rust type as both the `DdsSequence` element
type and the element stride; for `Inner { name: String, v: i32 }` that stride is 32 where
the wire layout is 16. The same held for a directly nested member, `DdsSequence<Inner>`,
`DdsBoundedSequence<Inner, N>` and `[Inner; N]`, none of which the entry named — in each
the generated native struct kept the Rust type while the sub-ops addressed the member
through `Inner::Native`.

Four of the five reproduced as `STATUS_ACCESS_VIOLATION` before the fix and one as a size
assertion, verified by stashing the fix and running each by name. The `Native` translation
is now recursive, via a new `DdsNativeValue` trait. `tests/native_layout_recursive.rs`.

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
| ~~D2~~ | ~~ASan job non-blocking~~ | done. The evidence had to come from the *step*, not the job: `continue-on-error` made the job report `success` either way, so ten green job conclusions meant nothing. Per-step conclusions are `success` on each of the last ten runs and the log shows no `ERROR: AddressSanitizer`. Now blocking, with the three new suites added |
| D3 | Trivy: CHANGELOG 2.0.2 records that CVE-by-CVE suppression is unsustainable | purge Perl/gzip from the final stage or move to distroless |
| D4 | `._ROADMAP_v5.md` deletion uncommitted, predates this work | commit or restore — owner's call |
| D5 | 8 files in `docs/` never checked against the current API: `qos-reference`, `security-guide`, `benchmarks`, `fuzzing`, `faq`, `async-patterns`, `security-production`, `architecture`. Now overdue three times: the typed-constructor break, fallible `clone_out`, and owned parents all changed example code | same sweep already done on the other six — but see F1 first |
| ~~D6~~ | ~~`cyclonedds-bench` never run~~ | done, and the premise did not hold: **no benchmark exercised the async path at all** — `latency`, `throughput`, `cdr` and `config_comparison` are synchronous and `ipc_comparison` mentions async only in a comment, so the claim was not merely unmeasured but unmeasurable. `benches/config_comparison.rs` was also missing its `[[bench]]` entry and had never compiled. Added `benches/async_read.rs`; reintroducing `spawn_blocking` temporarily gives `take/async` **18.38 µs** against **1.016 µs** inline, with `take/sync` at ~820 ns as the control — ~18x, about 17.4 µs per call. `latency` for reference: 1.43 / 1.62 / 3.86 µs at 64b / 1kb / 16kb |
| ~~D7~~ | ~~`fuzz/` never executed~~ | done — and it could not have been: the crate was neither a workspace member nor excluded and had no `[workspace]` table, so every cargo command in it failed before compiling. Fixed. Running libFuzzer still needs a non-Windows host, so the property is also asserted deterministically in `tests/cdr_deserialize_corpus.rs`, which found a live memory-safety defect in `CdrDeserializer` (see the CHANGELOG) |
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

### F3 — Two CycloneDDS copies, two versions · High · new

`cyclonedds-rust-sys` resolves its source as: `CYCLONEDDS_SRC`, then the
`cyclonedds-src` crate, then `vendor/cyclonedds` (`build.rs:228-256`). The crate
wins, and the two are different releases — **11.0.0** in `cyclonedds-src`,
**11.0.1** in `vendor/`. The difference is observable in the API:
`dds_stream_normalize` returns `bool` in one and
`enum dds_stream_normalize_result` in the other.

So the tree that gets read is not the tree that gets linked. Every "went to the C
source" claim in this backlog and in the commit history was made against
`vendor/`, and the library in the binary is the other one. Nothing found so far
turned on the difference — the ops encoding is identical across the patch
release, verified by round-trip — but the next one might.

**Need a decision.** Either bump `cyclonedds-src` to 11.0.1 (it is a *published*
crate, so that is a release of its own), or delete `vendor/` and read the crate,
or make the build prefer `vendor/` when it exists. Recommend the first: the
vendored tree is what everyone reads, and having it be the stale one is the
trap.

---

## Execution order

Each phase ends verifiable and committable. Phases 1, 2 and 6 are done.

1. ~~**ops() differential test**~~ — done, `683ae33`. Found three defects, two of them
   silent data loss. See B+ for what it did *not* reach: union `JEQ4`, `TYPE_EXT` beyond
   one level, and `nested_sequence_info`.
2. ~~**Nested composites and the remaining panics**~~ — done, `683ae33` and `955bdbc`.
3. **Union `JEQ4` and nested sequences** — the rest of B+, now that the harness exists.
   `derive_union_impl` builds its jump table with its own arithmetic and no test compares
   it to idlc; `nested_sequence_info` still emits sub-ops inline, which is the shape that
   was wrong in all four composite cases. Add the IDL to `ops_reference.idl` and extend
   `ops_vs_idlc.rs`. `B+` · ~0.5d
4. **SerdeSample type naming** — needs a design decision on what a stable name is, not just
   code. `B+++` · ~0.5d
5. **ABI snapshots** — the `SerdataHeader`/`SerdataOps` probe gap and the cross-compile
   snapshots. `A6, D8` · ~1d
6. **Release 3.0.0** — consolidate `[Unreleased]`, write the 2.x → 3.0 migration guide,
   bump `-sys`, decide A8 and F3, tag and publish. `C, A8, F3` · ~0.5d
7. ~~**Debt and measurement**~~ — D2, D6 and D7 done; D3 (Trivy) and D8 (snapshots) open,
   D5 still blocked on F1.

**Publishing 2.0.4 (C) is orthogonal** and can happen at any point — it builds from the tag,
not from `main`. **E1, F1 and F3 sit outside the sequence:** E1 is watchfulness, act only if
it recurs; F1 blocks D5 and any further documentation work; F3 blocks nothing today but
makes every future C citation suspect until it is settled. All three depend on the
maintainer.
