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

### ~~A6 — ABI probe~~ · closed

All 11 `dds_*_status_t` structs plus `dds_guid_t` were measured in `16d1e4a`. The gap it
left — the `SerdataHeader`/`SerdataOps` structs this crate hand-declares — is closed now
too. `ddsi_serdata.h` is an internal ddsi header bindgen is not pointed at, and reaching it
needed two more include dirs (`src/core/ddsi/include`, `src/core/cdr/include`); with those
the probe measures `ddsi_serdata`'s `ops`/`hash`/`refc` offsets and the three
`ddsi_serdata_ops` vtable slots this crate calls.

They are exactly what the 2.0.4 vtable fix turned on: the version before it hand-computed
byte offsets into that vtable, read one as a `u8`, transmuted the 0..=255 value into a
function pointer and called it. Verified the assertions bite by deleting one vtable slot
from the Rust declaration — the build fails naming `ddsi_serdata_ops.to_ser`.

### ~~D8 — abi/<triple>.rs snapshots~~ · partly closed, and honestly so

`cyclonedds-rust-sys/abi/x86_64-pc-windows-msvc.rs` exists. The other two CI targets do
not, and **cannot be produced from here**: the probe is a C program that answers by
running, so a linux snapshot requires a linux host. That is the property that makes the
snapshots worth anything.

What is in place instead of hand-typing them: `scripts/capture-abi-snapshot.sh` writes the
snapshot for whatever host it runs on, and each CI job now uploads its freshly probed
constants as an artifact — so `x86_64-unknown-linux-gnu` and `aarch64-apple-darwin` can be
committed from a CI run. The same step diffs against a committed snapshot when one exists,
which turns these from dead files into a drift check: an ABI change upstream fails CI on
the platform where it happened.

Note that cross-compilation is the only thing the snapshots serve. Every CI job builds
natively, so none of them exercises the snapshot path today.

### ~~A8 — DdsEntity::entity() is public~~ · decided: stays public

Kept public and documented as an escape hatch, which was the recommendation.
Closing it would push FFI users — passing an entity to a CycloneDDS API this crate does
not wrap, or adopting one made elsewhere — into `unsafe` transmutes to recover a number
the wrapper already holds, and the raw `from_entity`/`from_entities` constructors depend
on it. The doc comment now states the two rules that matter: the handle is valid only
while the wrapper is, and `dds_delete` on it is the wrapper's job.

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

**Second pass** extended the same harness to nested sequences and unions, and found two
more:

4. **Nested sequences took the element subtype from the outer container.**
   `DDS_OP_TYPE_*` names the container and `DDS_OP_SUBTYPE_*` the element; the derive read
   both off the outer one. idlc emits `TYPE_SEQ|SUBTYPE_BSQ` for
   `sequence<sequence<long,4>>` and `TYPE_BSQ|SUBTYPE_SEQ` for `sequence<sequence<long>,8>`;
   the derive had both backwards. Two of four combinations — and the one every existing
   test used, `sequence<sequence<T>>`, is the one that coincides.

   The inline layout suspected here turned out to be **correct**: unlike the composite
   cases, the jump word's high half counts the whole block, so the next member does follow
   it. Suspicion recorded, then disproved by the test.

5. **Unions had never crossed the wire, and `ops()` was wrong four ways at once.** The only
   union coverage drove `clone_out` against a hand-built buffer. The first test that
   actually published one hung the process. The discriminant typecode was in the primary
   type field (so `TYPE_UNI | TYPE_4BY` read as 0x0B, not a union); `alen` was written as 0;
   the `JEQ4` labels carried neither the member type nor its offset; and their jump targets
   were computed at two words per label while four were emitted. Rewritten to the format
   `dds_opcodes.h` documents. 64-bit discriminants are now a compile error — CycloneDDS
   admits only `{1BY,2BY,4BY,BLN}` there.

**Still not covered.** `TYPE_EXT` nesting is verified to one level only. Union cases whose
member is a composite remain unimplemented (now a clear compile error rather than an
`unreachable!()`).

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

### ~~B+++ — SerdeSample::type_name() is the same for every T~~ · decided: named explicitly

`type_name()` used `concat!("SerdeSample<", stringify!(T), ">")`, which in a generic impl
expands to the literal `"T"` — so every `SerdeSample<X>` announced the same DDS type name,
unrelated payloads matched each other on the wire, and each decoded the other's postcard
bytes as its own. `9881b38` made the string honestly type-agnostic without fixing the
matching.

**Decision: the name is supplied, not inferred.** A new `SerdeTypeName` trait carries
`const TYPE_NAME`, and `impl DdsType for SerdeSample<T>` requires it; `serde_type_name!`
is the one-liner for the common case.

Why not the alternatives. A hash of the `postcard` schema would be structural and
automatic, but `postcard`'s schema API is explicitly experimental and it would put a
`Schema` derive bound on every payload. `std::any::type_name` fails both requirements —
crate paths leak in and it carries no stability guarantee across compilations. And a DDS
type name is a *wire contract* between peers, the thing they match a topic on; inferring
it from Rust internals is the wrong shape of solution regardless of which internal is
used. The cost is a breaking change for `serde` users, which is the correct trade against
an API where every payload silently matches every other.

The bound makes a payload without a name a compile error rather than one more type that
matches everything.

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
| ~~D3~~ | ~~Trivy CVE-by-CVE suppression~~ | done: final stage is now `gcr.io/distroless/cc-debian12:nonroot`. Perl, gzip, bsdutils, the shell and the package manager are gone with the base image, and all seven `.trivyignore` entries with them — the file is now empty of CVEs. `libssl3` went too; the CLI has no OpenSSL dependency unless the `security` feature is on. **Not built locally** (no Docker on the machine this was written on); the release workflow builds and scans the image, so a mistake fails the release rather than shipping |
| ~~D4~~ | ~~`._ROADMAP_v5.md` deletion uncommitted~~ | committed. It is an AppleDouble resource fork (`._` prefix), a macOS filesystem artefact, not content |
| ~~D5~~ | ~~8 files in `docs/` never checked against the current API~~ | swept, and **the premise did not hold**: none of the eight contains a single stale constructor, `clone_out` call or `SerdeSample` reference. The three API breaks changed no example in them, because they contain almost no example code — `architecture.md`'s three mentions are Mermaid diagram labels with the arguments elided. One did need fixing, for a different reason: `fuzzing.md` documented a `cargo fuzz run` workflow that could never have worked (see D7) |
| ~~D6~~ | ~~`cyclonedds-bench` never run~~ | done, and the premise did not hold: **no benchmark exercised the async path at all** — `latency`, `throughput`, `cdr` and `config_comparison` are synchronous and `ipc_comparison` mentions async only in a comment, so the claim was not merely unmeasured but unmeasurable. `benches/config_comparison.rs` was also missing its `[[bench]]` entry and had never compiled. Added `benches/async_read.rs`; reintroducing `spawn_blocking` temporarily gives `take/async` **18.38 µs** against **1.016 µs** inline, with `take/sync` at ~820 ns as the control — ~18x, about 17.4 µs per call. `latency` for reference: 1.43 / 1.62 / 3.86 µs at 64b / 1kb / 16kb |
| ~~D7~~ | ~~`fuzz/` never executed~~ | done — and it could not have been: the crate was neither a workspace member nor excluded and had no `[workspace]` table, so every cargo command in it failed before compiling. Fixed. Running libFuzzer still needs a non-Windows host, so the property is also asserted deterministically in `tests/cdr_deserialize_corpus.rs`, which found a live memory-safety defect in `CdrDeserializer` (see the CHANGELOG) |
| ~~D8~~ | ~~`abi/<triple>.rs` snapshots exist for no target~~ | Windows committed; linux/macOS need a native host and are now obtainable as CI artifacts. See A6/D8 above |

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

### E3 — participant creation fails on domains 149-165, 200-201, 229 · Low · environmental

`DomainParticipant::new` returns `DDS_RETCODE_ERROR` for a contiguous block of domain ids
on this Windows machine, which fails 17 `test_dynamic_*` tests in
`cyclonedds/tests/integration_test.rs` — they are the ones that pick ids in that range.
Domain 0 (39 tests) and 139/141/146/169-174 are unaffected.

**Not a regression.** Verified by running the same suite from a worktree at `456ef1a`, the
commit before this work started: identical 17 failures. It also passed earlier the same
day, so something on the host changed underneath it.

The failure is inside `dds_create_participant`, at the socket layer, and reproduces in
0.00s with a single test in isolation. A probe over `0..=240` shows the same block plus
233-240, and for those CycloneDDS explains itself: *"resulting port number (67400) is out
of range"* — `7400 + 250 * domain` exceeds 65535 past domain 232. The 149-165 block
computes to 44650-48650, which is in range and had no listener when checked
(`Get-NetUDPEndpoint`), so that part is still unexplained.

**Action.** Do not treat the 17 failures as a code defect. Worth pinning the test domains
to a range known to work, or having them fall back when creation fails, so a host quirk
stops looking like a regression. CI has never shown this.

### E2 — async-stream will not expand on local stable · Low

`async_stream::stream! { yield 1; }` fails in a two-line crate on rustc 1.95 and works on
1.85.0 and nightly. Ruled out: `proc-macro2`/`syn`/`quote` versions, incremental cache,
target dir, sandbox. CI is unaffected.

**Action.** Local environment only, worked around with `+1.85.0`. If it becomes annoying:
`rustup update` and retest; if it persists it is a bug to report upstream. Worth the effort
only if development moves to stable.

---

## F. Needs the maintainer

### ~~F1 — The READMEs you updated~~ · resolved by proceeding

They never appeared. Asked twice across the session and explicitly delegated back, so the
working assumption is that they are not in this repository — if they were in another
`Z:	ese` project, there is no conflict to resolve.

Acted accordingly: the pending example fixes in the root README, `cyclonedds/README.md`
and five files under `docs/` — all of them the mechanical `Publisher::new(dp.entity())` →
`Publisher::new(&dp)` change forced by the owned-parents break — are committed rather than
left dangling in the working tree, and D5's sweep went ahead.

**If a different version does exist**, these are ordinary text changes in git history and
reconciling them is a diff, not a rescue.

### ~~F2 — Typed Publisher / Subscriber / WaitSet?~~ · answered by A1

They take `&DomainParticipant` now. A1 absorbed the question, as expected.

### F3 — Two CycloneDDS copies, two versions · decided: made visible, not silently aligned

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

**Decided.** None of the three options was taken, because each is worse than it
looks. `vendor/cyclonedds` is a *submodule*; `cyclonedds-src` is 1505 checked-in
files. Preferring `vendor/` would make the build depend on whether someone cloned
with `--recursive` — non-reproducible, and it would break crates.io users
outright. Deleting `vendor/` loses a newer reference for no gain. Bumping
`cyclonedds-src` to 11.0.1 is the *right* end state but it is a publish, which is
the owner's call and not something to slip into a soundness pass.

So the current precedence stays — it is correct — and the trap is closed a
different way: `cyclonedds-rust-sys`'s build script now prints which source and
version it compiled, and warns explicitly when the submodule is a different
release from the one being linked. The mismatch can no longer be discovered the
hard way.

**Still open for the owner:** aligning the two, by bumping `cyclonedds-src` to
11.0.1 as part of a release. Until then the warning is the safeguard.

---

## Execution order

Each phase ends verifiable and committable. Phases 1, 2 and 6 are done.

1. ~~**ops() differential test**~~ — done, `683ae33`. Found three defects, two of them
   silent data loss.
2. ~~**Nested composites and the remaining panics**~~ — done, `683ae33` and `955bdbc`.
3. ~~**Union `JEQ4` and nested sequences**~~ — done. Two more defects: the nested-sequence
   element subtype, and a union `ops()` wrong in four independent ways that meant no union
   had ever been published. Only `TYPE_EXT` beyond one level is left unexamined.
4. **SerdeSample type naming** — needs a design decision on what a stable name is, not just
   code. `B+++` · ~0.5d
5. ~~**ABI snapshots**~~ — done. The probe reaches `ddsi_serdata.h` now, and the Windows
   snapshot is committed; linux/macOS come from a CI artifact, since they cannot be
   produced anywhere but on those hosts.
6. **Release 3.0.0** — consolidate `[Unreleased]`, write the 2.x → 3.0 migration guide,
   bump `-sys`, decide A8 and F3, tag and publish. `C, A8, F3` · ~0.5d
7. ~~**Debt and measurement**~~ — D2, D6 and D7 done; D3 (Trivy) and D8 (snapshots) open,
   D5 still blocked on F1.

**Publishing 2.0.4 (C) is orthogonal** and can happen at any point — it builds from the tag,
not from `main`. **E1, F1 and F3 sit outside the sequence:** E1 is watchfulness, act only if
it recurs; F1 blocks D5 and any further documentation work; F3 blocks nothing today but
makes every future C citation suspect until it is settled. All three depend on the
maintainer.
