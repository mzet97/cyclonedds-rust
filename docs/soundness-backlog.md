# Soundness backlog

Everything left open after 2.0.4, with the fix for each item. Nothing curated out.

Baseline: `main @ a9cdd0b` · 304 tests · CI 6/6 green · verified on `+1.85.0` (MSRV).

Estimated total ~11 days, of which ~4.5 are the two API breaks that make up 3.0.0.

---

## A. Soundness still open

A1 is the root: A2, A3 and A7 only close once it does.

### A1 — Drop order with unowned parents · High · breaks API · ~2d

Struct fields drop in declaration order. In `struct App { dp, sub, reader }` the participant
dies first, CycloneDDS deletes its children recursively, and the later `Drop`s call
`dds_delete` on handles that are already gone. Harmless **except** when the handle was
recycled in between, where it destroys the wrong entity — silently and
non-deterministically.

**Fix.** Each entity owns its parent: `Arc<ParticipantInner>` in `Publisher`/`Subscriber`/
`Topic`, `Arc<SubscriberInner>` in `DataReader`, and so on. The parent's `Drop` only runs
once the last child is gone, which makes ordering irrelevant by construction. Today
`entity::delete_entity` merely *logs* the failure.

**Verification.** Test with a struct in hostile field order, under ASan.

### A2 — DataReader can outlive its Topic · High · breaks API · depends on A1

The typed constructors closed type confusion, but the `&Topic<T>` reference only has to live
for the call — the returned reader does not retain it.
`DataReader::new(&sub, &Topic::new(&dp,"x")?)` still leaves the reader on a dead topic.

**Fix.** Falls out of A1: the reader holds an `Arc` of the topic. Cheaper alternative if A1
is deferred: a lifetime parameter (`DataReader<'a, T>`) — works, but is viral across the
whole API.

### A3 — WaitSet::wait_async is not cancellable · Medium · breaks API · depends on A1 · ~0.5d

`spawn_blocking` tasks cannot be cancelled. Dropping the future leaves the thread waiting
until the waitset triggers; if the `WaitSet` dies in the meantime, the wait sits on a
deleted entity. Documented on the method, not closed.

**Fix.** `WaitSet` under `Arc`, cloned into the blocking task so the entity outlives the
wait. Complement: expose `dds_waitset_set_trigger` to wake the wait on drop, making
cancellation effective rather than merely safe.

### A4 — clone_out is infallible · High · breaks API · ~1d

The generated `clone_out` for a union without `#[dds_default]` panics on an unknown
discriminator, and the discriminator comes off the wire. The `catch_unwind` barrier stops
the process abort, but a peer built from a different IDL revision still makes
`reader.take()` panic on the user's thread.

**Fix.** `unsafe fn clone_out(ptr) -> DdsResult<Self>`. Ripple: the trait, the derive (3
templates), and every call site — `reader.rs`, `sample.rs`, `async.rs`, `serialization.rs`,
`content_filtered_topic.rs`. An undecodable sample becomes a discarded error instead of a
downed call.

**Verification.** Test with a union discriminator outside the declared range.

### A5 — 13 panics on reachable paths · Medium · couples to A4 · ~0.5d

In `dynamic_type.rs`, `dynamic_value.rs`, `sequence.rs` and `string.rs`. The
`dynamic_type.rs` ones sit in `to_schema`, on the user's thread, outside any trampoline —
checked. But `sequence.rs:83` (`expect` in `clone_from_raw`), `string.rs:47` (`Default` with
`expect`) and `string.rs:57` (`assert!` in `Clone`) **are** reachable via `clone_out`.

**Fix.** The three reachable ones ride along with A4 as propagated errors. The
`dynamic_type.rs` ones become `DdsResult` on the builder — an API misuse should be `Err`,
not a panic.

### A6 — ABI probe covers little · Medium · ~0.5d

Checks `dds_sample_info` plus 6 scalar typedefs. Does not cover the ~11 status structs read
by value in `entity.rs:107-155`, nor `dds_guid_t`, nor the `SerdataHeader`/`SerdataOps`
structs hand-declared in `-sys` — precisely the ones that replaced magic offsets and that a
new CycloneDDS release could invalidate.

**Fix.** Extend `ABI_PROBE_C` with `sizeof`/`offsetof` for those types and add the matching
`const assert`s in `sys/src/lib.rs`. Mechanical. Regenerate the `abi/<triple>.rs` snapshots —
which, incidentally, do not exist for any target yet (see D8).

### A7 — Publisher / Subscriber / WaitSet still take raw handles · Low · absorbed by A1

Deliberate: they carry no type parameter, so the confusion that motivated the typed
constructors does not apply. They do still accept a temporary's handle.

**Fix.** Free with A1. Standalone it is a signature change to `&DomainParticipant` plus call
site migration — the same pattern already applied to `Topic`/`DataReader`/`DataWriter`
(~2h).

### A8 — DdsEntity::entity() is public · Low · decision, not code

The review suggested `pub(crate)`. Not done: it is required for FFI interop and the
`from_entities` constructors depend on it.

**Fix.** Keep it public, document it as an escape hatch and mark the raw constructors
`#[doc(hidden)]` — or do nothing and record the decision. Recommend the latter: closing this
pushes FFI users into `unsafe` for no real gain.

---

## B. Never audited — 6,710 lines

Grep only. Not claiming these are correct; claiming they were not read. Ordered by risk
(`unsafe` density × exposed surface), not size.

| Module | Why it matters | Lines |
|---|---|---:|
| `xtypes.rs` | Highest `unsafe` density in the crate (63 blocks). Only the 4 `from_raw_parts` were checked. | 1,078 |
| `qos.rs` | Only the 2 `unsafe impl`s and 3 `qget` functions — and one of those held a leak. The other ~20 unexamined. | 1,624 |
| `dynamic_value.rs` | Builds and reads dynamic values through pointers; not a line read. | 1,186 |
| `dynamic_type.rs` | Only the `expect`s. The rest of the builder and type registration unseen. | 1,071 |
| `type_discovery.rs` | Converts CDR ↔ dynamic data over raw buffers; not a line read. | 986 |
| `request_reply.rs` | Correlation IDs and timeouts over pub/sub; not a line read. | 265 |
| `security.rs` | X.509 certificates and hot-reload. Security surface, zero audit. | 205 |
| `participant_pool.rs` | `lock().unwrap()` throughout; poisoning propagates panics. | 184 |
| `serde_sample.rs` | `postcard` wrapper; lowest risk, still never read. | 111 |

### B+ — ops() bytecode beyond the scanner · High · ~1d

The scanner is gone, but the rest of the generation was **not** audited: `OP_KOF` key
patching, union `JEQ4` entries, and `TYPE_EXT` offsets nested more than one level deep. It
is the least verifiable part of the project and an error here corrupts serialization for any
type.

**Fix.** Differential test against the C `idlc`: compile the same IDL through both paths and
compare the ops arrays word by word. The only way to verify this without relying on human
reading.

### B++ — Vec\<Composite\> with nested heap fields · Medium · ~0.5d

Gap the code itself admits at `writer.rs:271-277`: the derive uses the inner composite type
directly as the `DdsSequence` element instead of `<Inner as DdsType>::Native`. Correct only
while `Inner` has no `String`/`Vec` of its own.

**Fix.** Apply the `Native` translation recursively in the derive. Test:
`struct Outer { items: Vec<Inner> }` with `Inner { name: String }`, under ASan.

---

## C. Release on hold

2.0.4 is committed with CI 6/6 green three commits back. The further `main` advances with
breaking changes on top, the more expensive extracting that release becomes.

| Item | State | Action |
|---|---|---|
| tag `v2.0.4` | not created | `git tag -a v2.0.4` on commit `5c3e515`, **not** on current HEAD |
| publish crates.io | not done, 9 crates | order: `-src` → `-sys` → `-derive` → `cyclonedds` → `-build` → `-idlc` → `-cli` → `cargo-` → `-wasm`. **Irreversible** — needs explicit approval |
| `cyclonedds-src` | at 1.0.1, not bumped | check whether the submodule moved since 1.0.1; bump if so |
| `[Unreleased]` | accumulating breaking changes | becomes 3.0.0 once phase 5 lands |

---

## D. Quality and debt

None of this is soundness. All of it is cheap and reduces recurring friction.

| # | What | Action |
|---|---|---|
| D1 | `Qos` safety comment says "immutable after construction" — false, `set_property` exists | rewrite: the real argument is `&mut self` |
| D2 | ASan job ran once, is `continue-on-error`, nobody read the output | read the log; promote to blocking if stable |
| D3 | Trivy: CHANGELOG 2.0.2 records that CVE-by-CVE suppression is unsustainable | purge Perl/gzip from the final stage or move to distroless |
| D4 | `._ROADMAP_v5.md` deletion uncommitted, predates this work | commit or restore — owner's call |
| D5 | 8 files in `docs/` never checked against the current API: `qos-reference`, `security-guide`, `benchmarks`, `fuzzing`, `faq`, `async-patterns`, `security-production`, `architecture` | same sweep already done on the other six |
| D6 | `cyclonedds-bench` never run; dropping `spawn_blocking` should have cut latency and it was not measured | run before/after and record |
| D7 | `fuzz/` never executed | run the existing targets; consider a new one for `clone_out` |
| D8 | `abi/<triple>.rs` snapshots exist for no target — cross-compilation fails by design | generate for the supported targets |

---

## E. Unexplained

Two observed behaviours with no explanation. Recorded so they do not become folklore.

### E1 — SIGSEGV in tests/qos.rs under coverage · Medium

Happened once, on `a1be1ca9`, under `cargo llvm-cov`. Did not reproduce across 15 isolated
runs, the 304-test local suite, or the two following CI commits. The test exercises
`qos.data_representation()` — exactly what was changed — but the crash came before any test
reported.

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

### F1 — The READMEs you updated · High

They do not appear in this repository — `git status` clean, last commit is mine. And the
root README and 5 files in `docs/` had their examples rewritten for the typed-constructor
API break. If your version lives elsewhere there is a real conflict to reconcile.

**Need to know.** Where they are. If they were in another `Z:\tese` project there is no
conflict. If they were here, say so before documentation is touched again.

### F2 — Typed Publisher / Subscriber / WaitSet? · Low

Left with raw handles, judging that consistency does not justify churn with no defect behind
it (see A7). Quick to change if a uniform API is preferred. If A1 goes ahead the question
disappears — it absorbs all three.

---

## Execution order

Each phase ends verifiable and committable.

1. **Close 2.0.4** — tag and publish. `C` · ~1h · *needs explicit approval before publish*
2. **Audit before redesigning** — read B in risk order; differential `ops()` test. Comes
   first because it may change what the redesign has to cover. `B, B+, B++` · ~3d
3. **Fallible clone_out** — independent of A1 and cheaper. Closes A4 and the three reachable
   panics of A5 at once. `A4, A5` · ~1.5d · breaks API
4. **Ownership: owned parents** — the central redesign. `Arc` parents close A1, A2, A3 and
   A7 together; attempting any in isolation is rework. `A1, A2, A3, A7` · ~3d · breaks API
5. **ABI probe and snapshots** — extend the probe and generate cross-compile snapshots in one
   pass; same file, same reasoning. `A6, D8` · ~1d
6. **Release 3.0.0** — consolidate `[Unreleased]`, write the 2.x → 3.0 migration guide,
   decide A8, tag and publish. `C, A8` · ~0.5d
7. **Debt and measurement** — D1 through D7. Blocks nothing, but D2 and D3 cut recurring
   friction and D6 measures a gain that is currently only a claim. `D1–D7` · ~1.5d

**E1 and F1 sit outside the sequence.** E1 is watchfulness — act only if it recurs. F1 blocks
any further documentation work and depends entirely on the maintainer.
