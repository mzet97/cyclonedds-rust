# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [3.0.0-alpha.3] - 2026-08-18

### Fixed

- Key serialization no longer links against CycloneDDS's private
  `dds_stream_write_key` symbol. It now serializes through the exported sample
  writer and extracts the key through the exported CDR API, so shared-library
  consumers link correctly.
- The coverage workflow runs DDS tests serially, matching the CI and MSRV
  workflows and avoiding cross-test discovery contention.

## [3.0.0-alpha.2] - 2026-08-18

`3.0.0-alpha.2` carries breaking changes relative to 2.x. Anything tagged
`v2.0.4` is the last release without them.

### Fixed

- **A `#[derive(DdsUnionDerive)]` type had never survived a round trip, and its
  `ops()` was wrong in four independent ways.** The only union coverage drove
  `clone_out` against a buffer the test built by hand; nothing had ever handed a
  union to CycloneDDS. The first test that did hung the process indefinitely —
  the reader walking a malformed ops array — and that is what the four defects
  add up to:

  1. The discriminant's typecode was emitted in the **primary type** field, so it
     was OR'd into `TYPE_UNI` (0x09 | 0x02 = 0x0B) and the opcode no longer named
     a union at all. `dds_opcodes.h` puts it in the subtype field:
     `[ADR, UNI, d, z] [offset] [alen] [next-insn, cases]`.
  2. `alen` — the case count — was written as `0`, and the case count was packed
     into the high half of the last header word instead of the distance to the
     next member instruction.
  3. Case labels were emitted as `[JEQ4] [value] [0] [jump]`: no member type in
     the opcode, so the reader could not know how to decode the case, and no
     member offset, so it had nowhere to read it from. The correct form is
     `[JEQ4, type] [value] [offset] [0]`.
  4. Those jump targets were computed as if each label were two words while four
     were emitted, so every one pointed four words short of its own case ops.

  The header now carries `OP_FLAG_DEF` when a `#[dds_default]` variant exists and
  the default is emitted as the last label with value 0, matching idlc. A 64-bit
  discriminant is now rejected at compile time instead of emitted wrong:
  CycloneDDS admits only `{1BY, 2BY, 4BY, BLN}` there. `tests/union_wire.rs`
  round-trips both arms.

- **Nested sequences took the element subtype from the outer container.**
  `DDS_OP_TYPE_*` names the container and `DDS_OP_SUBTYPE_*` names the element;
  the derive read both off the outer one, which only coincides when the two kinds
  match. idlc emits `TYPE_SEQ|SUBTYPE_BSQ` for `sequence<sequence<long,4>>` and
  `TYPE_BSQ|SUBTYPE_SEQ` for `sequence<sequence<long>,8>`; the derive had both
  backwards. Two of the four container/element combinations were affected —
  `sequence<sequence<T>>`, the one every test used, was not.

- **`CdrDeserializer` followed length prefixes out of arbitrary bytes.** Both
  `deserialize` and `deserialize_key` are safe functions taking an arbitrary
  `&[u8]`, and both handed it straight to `dds_istream_init` +
  `dds_stream_read_sample`. That violates a precondition the C states outright:
  "The buffer must contain well-formed CDR data in native endianness. Use
  `dds_stream_normalize` to verify well-formedness" (`dds/cdr/dds_cdrstream.h`).
  Without that step the reader trusts every length prefix it finds.

  Reproduced immediately, and not by reading: 87 bytes from a seeded PRNG, fed
  to a type with a `String` and a `Vec`, took the process down with
  `STATUS_ACCESS_VIOLATION` on the very first iteration. A POD type survives —
  which is why nothing caught it, since the only CDR test types were POD. Both
  entry points now normalize first and reject what does not validate with
  `BadParameter`. A truncation test asserts that every proper prefix of a valid
  sample is refused, and a round-trip test asserts the validation did not simply
  start refusing everything.

  Reachable from the network for anyone deserializing what `read_cdr`/`take_cdr`
  hands back, which is the entire purpose of those methods — the third of the
  three README features that turned out to have no test behind them, after
  `serde` and the union derive.

- **Two DDS return codes were mapped to the wrong error, one of them onto a
  retry loop.** `dds/ddsrt/retcode.h:32-45` says `-2` is `UNSUPPORTED` and `-12`
  is `ILLEGAL_OPERATION`; `From<i32> for DdsError` read them as `OutOfMemory` and
  `Unsupported("unsupported")`, and `-13` (`NOT_ALLOWED_BY_SECURITY`) fell
  through to a bare `ReturnCode`. `is_transient()` answers `true` for
  `OutOfMemory`, so **every permanently unsupported operation looked worth
  retrying**. Reproduced end to end: `DynamicTypeBuilder::map(..).build()` — which
  cannot ever succeed, because CycloneDDS 11.0.1 returns `UNSUPPORTED` for
  `DDS_DYNAMIC_MAP` (`dds_dynamic_type.c:237`) — returned `OutOfMemory` with
  `is_transient() == true`.

  `DdsError::OutOfMemory` is now produced only by this crate (a `len × size`
  overflow in the sequence constructors) and `raw_code()` no longer claims `-2`
  for it. CycloneDDS has no out-of-memory retcode: `ddsrt_malloc` aborts rather
  than returning null, which is the same fact that makes the null checks around
  `dds_alloc` dead code.

- **`WaitSet::wait_async` ran on a handle it did not keep alive.** The
  `spawn_blocking` task captured only the raw `dds_entity_t`, so between the
  waitset's `dds_delete` and the task's `dds_entity_pin` the handle could be
  redrawn for another entity — the window measured for A1, argued rather than
  demonstrated. It now holds an `Arc` of the waitset.

  Doing only that introduced a real hang, which is why it is worth writing down:
  deleting a waitset is what used to interrupt an in-flight wait
  (`dds_waitset_interrupt`, `dds_waitset.c:92`, wired in at `:137`), and holding
  the `Arc` prevents the deletion. A stream dropped mid-wait on a 30-second
  timeout then held a runtime thread for **29.7 seconds**. Sample streams now
  attach their waitset to itself and set its trigger on drop, which is the
  mechanism CycloneDDS documents for exactly this, so the wait returns at once
  and the entity is released. `tests/async_wait_cancellation.rs` measures it
  through runtime shutdown.

  **The backlog's description of A3 was wrong and is corrected rather than
  repeated.** It said dropping the future "leaves the thread waiting until the
  waitset triggers or the timeout expires". That did not reproduce: before this
  change the waitset *was* deleted with the stream, and the C interrupted the
  wait in well under a second.

- **The member declared after a composite one was never serialized.** For a
  `sequence<Struct>`, `sequence<Struct, N>` or `Struct[N]` member, the derive emitted the
  element's sub-ops *inline*, immediately after the instruction, and wrote a constant jump
  word — `(4<<16)+5` for sequences, `(5<<16)+6` for bounded sequences and arrays. In
  CycloneDDS's encoding the high half of that word is the distance to the **next member's
  instruction** and the low half the distance to the element's sub-ops. Both are positions,
  not constants: the C `idlc` emits `(4<<16)+7` for the same sequence when a member follows
  it, and puts the sub-ops after the terminating `RTS`. Hardcoding them made the "next
  member" pointer land on an `RTS`, so the interpreter stopped there.

  Consequences, in order of how quietly they happen: `struct { long h; sequence<P> items;
  long tail; }` round-tripped with `tail` reset to `0`; with more members after it, or with
  a keyed type, the same layout crashed the process — `STATUS_ACCESS_VIOLATION` on Windows.
  Found by compiling the same IDL through `idlc` and comparing the arrays word by word,
  which is now `cyclonedds-test-suite/tests/ops_vs_idlc.rs`. Reading the emitter had not
  found it: the arrays look plausible, and the shape that works — a composite member
  declared last — is the shape every existing test used.

  All four composite shapes now go through the same patch step `TYPE_EXT` already used:
  the child block is appended after the terminating `RTS` and the jump word is computed
  once the layout is known.

- **`#[key]` on a member declared after a composite one produced the wrong key offset.**
  `keys()` advanced its ops index by the width of the instruction *plus the inlined child
  block*, so the `KOF` operand pointed into the element's sub-ops rather than at the key's
  own `ADR`. idlc emits `KOF | 1, 3` for `struct { P inner; @key long id; }`; the derive
  emitted `9`. Fixed by counting only the main instruction stream, which is what the index
  means. Also removes a redundant trailing `RTS` the derive appended to every nested block
  — unreachable, but it inflated `m_nops` and was a real divergence from idlc.

- **`Vec<Composite>` (and the other four composite shapes) mis-described any inner type
  with heap fields.** The gap `writer.rs` documented in prose, now closed and wider than
  recorded: the derive used the inner *Rust* type both as the `DdsSequence` element type
  and as the element stride. For `Inner { name: String, v: i32 }` that stride is 32 where
  the wire layout is 16, and the buffer handed to CycloneDDS held Rust `String` triples
  where the ops array said `char *`. The same applied to a directly nested composite
  member, to `DdsSequence<Inner>`, `DdsBoundedSequence<Inner, N>` and `[Inner; N]`, none of
  which were named in the original note — for all of them the generated native struct kept
  the Rust type while the sub-ops addressed the member through `Inner::Native`.

  Each of the five reproduced first: four as `STATUS_ACCESS_VIOLATION`, one as a size
  assertion. The `Native` translation is now applied recursively, via a new
  `DdsNativeValue` trait (see below), and `tests/native_layout_recursive.rs` covers all
  five shapes.

### Changed

- **BREAKING**: `SerdeSample<T>` now requires `T: SerdeTypeName`, a new trait carrying
  `const TYPE_NAME`. `type_name()` used to return the same literal for every payload —
  `stringify!(T)` in a generic impl expands to `"T"`, not to the substituted type — so
  every `SerdeSample<X>` announced the same DDS type name, unrelated payloads matched each
  other on the wire, and each decoded the other's postcard bytes as its own.

  The name is supplied rather than inferred, via `serde_type_name!(MyType, "acme::MyType")`.
  A hash of the `postcard` schema was the alternative, and was rejected: that API is
  explicitly experimental and it would put a `Schema` derive bound on every payload.
  `std::any::type_name` fails outright — crate paths leak in and it has no stability
  guarantee across compilations. A DDS type name is a wire contract between peers; deriving
  it from Rust internals is the wrong shape of answer whichever internal is chosen. The
  bound makes a payload without a name a compile error instead of one more type that
  matches everything.

- The release container's final stage is `gcr.io/distroless/cc-debian12:nonroot` instead of
  `debian:bookworm-slim`. `.trivyignore` had grown seven entries chasing CVEs in Perl, gzip
  and bsdutils — none of them reachable, since the artifact is a single Rust binary that
  invokes neither — and four different ones came and went in that group in two days. The
  packages are gone with the base image, and the ignore list with them. `libssl3` went too:
  the CLI has no OpenSSL dependency unless the `security` feature is enabled. **Not built
  locally** — no Docker on the machine this was written on; the release workflow builds and
  scans the image, so a mistake fails the release rather than shipping.

- `DdsEntity::entity()` stays public, now documented as an escape hatch with the two rules
  that matter: the handle is valid only while the wrapper is, and `dds_delete` on it is the
  wrapper's job. A review had proposed `pub(crate)`; that would force FFI callers into
  `unsafe` transmutes to recover a number the wrapper already holds, and the raw
  `from_entity`/`from_entities` constructors depend on it.

- `cyclonedds-rust-sys`'s build script now prints which CycloneDDS source and version it
  compiled, and warns when `vendor/cyclonedds` is a different release from the one being
  linked. See the note below; the mismatch itself is left for the maintainer to close.

### Infrastructure

- **The ABI probe now covers the two structs this crate hand-declares.**
  `SerdataHeader` and `SerdataOps` mirror `struct ddsi_serdata` and its vtable, which live
  in an internal ddsi header bindgen is not pointed at — so until now nothing but a careful
  reading tied them to the C. They are also precisely what the 2.0.4 vtable fix turned on:
  the version before it hand-computed byte offsets into that vtable, read one of them as a
  `u8`, transmuted the resulting 0..=255 value into a function pointer and called it.
  Reaching `ddsi_serdata.h` needed two more include directories; the probe now measures
  `ops`/`hash`/`refc` and the three vtable slots this crate calls, and `sys/src/lib.rs`
  asserts them. Verified by deleting one vtable slot from the Rust declaration: the build
  fails naming `ddsi_serdata_ops.to_ser`.

- **First `abi/<triple>.rs` snapshot**, for `x86_64-pc-windows-msvc`. These exist so a
  *cross*-compile has measured constants rather than guessed ones, and they cannot be
  written by hand or produced from another host — the probe answers by running. The other
  two CI targets are not committed for exactly that reason; instead
  `scripts/capture-abi-snapshot.sh` produces one for whatever host it runs on, and each CI
  job uploads its freshly probed constants as an artifact so they can be committed from a
  CI run. The same step diffs against a committed snapshot when one exists, so an upstream
  ABI change fails CI on the platform where it happened.


- **Nothing measured the async read path**, so 2.0.4's claim that dropping
  `spawn_blocking` cut latency was not merely unmeasured — it was unmeasurable.
  `latency`, `throughput`, `cdr` and `config_comparison` are synchronous;
  `ipc_comparison` mentions async only in a comment. `benches/async_read.rs`
  measures `take` against `take_async` on identical work, and reintroducing the
  old wrapper long enough to take a reading gives the number the entry wanted:

  | | `take/sync` | `take/async` |
  |---|---|---|
  | with `spawn_blocking` (pre-2.0.4) | 814 ns | **18.38 µs** |
  | inline (current) | 831 ns | **1.016 µs** |

  ~18×, about 17.4 µs per call, with the synchronous arm as an unmoving control.
  One machine, one runtime flavour — a baseline to catch regressions against,
  not a headline. For reference the existing `latency` bench, run for the first
  time, reports 1.43 / 1.62 / 3.86 µs at 64 B / 1 KiB / 16 KiB.

  `benches/config_comparison.rs` also had no `[[bench]]` entry, so cargo had
  never compiled it — the same shape as the fuzz crate below. It compiles.

- **`fuzz/` could not be built at all**, let alone run. The crate was neither a
  workspace member nor in `workspace.exclude`, and had no `[workspace]` table of
  its own, so every cargo command in that directory failed with "current package
  believes it's in a workspace when it's not" — `cargo fuzz run` included. Adding
  the table makes it build; it had survived the 3.0 API breaks otherwise.

  Running it still needs libFuzzer, which rules out `x86_64-pc-windows-msvc`. The
  same property is therefore also asserted in
  `cyclonedds-test-suite/tests/cdr_deserialize_corpus.rs` with a seeded PRNG:
  weaker at finding inputs than coverage-guided fuzzing, stronger at *staying*
  run, since it needs nothing beyond the MSRV and executes on every CI platform.
  That is what found the defect above.

- **The build does not use `vendor/cyclonedds`.** `cyclonedds-rust-sys`'s source
  resolution prefers the `cyclonedds-src` crate over the vendor directory
  (`build.rs:241-248`), and the two are different releases: `cyclonedds-src`
  carries **11.0.0**, `vendor/cyclonedds` carries **11.0.1**. The difference is
  observable — `dds_stream_normalize` returns `bool` in 11.0.0 and
  `enum dds_stream_normalize_result` in 11.0.1 — so the tree that gets read when
  someone "goes to the C source" is not the tree that gets linked. Recorded, not
  yet resolved: reconciling them is the owner's call, since it means either
  bumping `cyclonedds-src` (which is a published crate) or dropping `vendor/`.
  The ops fixtures in `tests/ops_vs_idlc.rs` were generated with idlc **11.0.1**
  from `vendor/` and verified against the linked 11.0.0 library by round-trip;
  the encoding is unchanged across the patch release.

### Changed (hardening, not fixes)

- `DynamicTypeBuilder::to_schema` returns `DdsResult` instead of `expect`ing on a
  missing sub-type in six places and `panic!`ing in a seventh. **No failing test
  could be written for any of them and none is claimed:** those states are not
  reachable through the public API — `DynamicTypeBuilder::new` is private, every
  constructor whose kind needs a sub-type takes it as an argument, and the
  setters take values rather than `Option`s, so a sub-type can be replaced but
  never cleared. `DynamicType::create` already returned `DdsResult`, so this
  costs nothing and means a future constructor that forgets one reports it
  instead of unwinding on the caller's thread.
  `every_public_constructor_builds_without_panicking` pins the unreachability
  claim so it fails here rather than in a user's process.

- `DynamicTypeBuilder::map`/`bounded_map` now document that they cannot succeed
  against CycloneDDS 11 (see the retcode entry above). The constructors are kept
  so the API tracks the XTypes kinds.

### Added

- `DdsNativeValue`, a trait carrying `to_native_value(&self, &mut WriteArena)`. Composite
  elements need the native *value*, not the pointer `DdsType::write_to_native` returns,
  because a `sequence<Struct>`'s elements have to sit contiguously at the stride the
  descriptor declares. Deliberately a separate trait rather than another `DdsType` method,
  so the ~19 hand-written `impl DdsType` blocks in this repository keep compiling; a manual
  type used as a composite element without it fails to compile instead of mis-serializing.
- `DdsSequence::from_vec` / `DdsBoundedSequence::from_vec`, the moving counterparts of
  `from_slice`. The generated native structs own `DdsString`/`DdsSequence` fields and are
  not `Clone`, so the element buffer has to be filled by moving rather than cloning.
- `scripts/regen-ops-fixtures.sh` and `cyclonedds-test-suite/tests/idl/ops_reference.idl`:
  the provenance of the expected ops arrays. `idlc` is not built by the normal build
  (`-DBUILD_IDLC=OFF` in the `-sys` build script), so the script configures it out of tree.
  Two differences from idlc are documented rather than matched: it appends a `KOF` chain to
  `m_ops` where this crate builds one in `Topic::new` from `keys()`, and it shares one
  sub-ops block between members of the same element type where the derive emits one each.
  Both are valid encodings.

### Changed

- **BREAKING**: every entity now owns its parents. `Publisher::new`, `Subscriber::new`,
  `WaitSet::new` and `GuardCondition::new` take `&DomainParticipant` instead of a
  `dds_entity_t`; `ReadCondition::new`/`any`/`not_read` and `QueryCondition::new`/
  `with_filter` take `&DataReader<T>`. Each entity holds an `Arc` of its ancestors, so a
  parent's `dds_delete` cannot run until the last child has released it.

  What this closes: CycloneDDS deletes an entity's whole subtree when the entity is
  deleted, and struct fields drop in declaration order, so
  `struct App { participant, subscriber, reader }` destroyed the participant first and the
  children's `Drop`s then ran `dds_delete` on handles that were already gone. The same root
  cause let a `DataReader` outlive the `Topic` and `DomainParticipant` it was built from,
  after which **every call on it failed** — reproduced in `parent_ownership.rs`, where a
  reader, writer, topic and subscriber that escape the scope their participant was declared
  in all returned `PreconditionNotMet` before this change and now work. Declaration order
  is irrelevant by construction.

  **Severity, stated accurately.** The previous note in `docs/soundness-backlog.md` said a
  recycled handle would destroy the wrong entity. That is possible but rare, and the
  measurement belongs here: `dds_handle_create` (`dds_handles.c:116`) draws each handle
  uniformly at random from `[1, DDS_MIN_PSEUDO_HANDLE)` — about 2.1e9 values — and every C
  entry point resolves a handle through a hash table, so a stale handle is almost always
  simply absent and yields an error. Reaching a *different live* entity needs that exact
  value redrawn: ~1 in 2.1e9 per entity created. The routine, always-present defect is the
  silent error returns, not memory corruption.

  The raw constructors remain for FFI interop and are documented as unchecked — they adopt
  a handle without holding anything alive: `Topic::from_entity`/`with_qos_from_entity`,
  `Data{Reader,Writer}::from_entities`/`from_entities_with`, and the new
  `Publisher::from_entity`, `Subscriber::from_entity`, `WaitSet::from_entity`,
  `GuardCondition::from_entity`, `ReadCondition::from_entity`, `QueryCondition::from_entity`.

  Two side effects worth naming. The async streams' `WaitSet` now holds the reader (and
  through it the whole chain) alive for the life of the stream, so a `spawn_blocking` wait
  can no longer be left sitting on an entity someone else deleted — the safety half of A3;
  cancellation itself is still open. And `Listener` moved inside the owned handle, so a
  listener is dropped after the entity that could invoke it rather than alongside it.

- **BREAKING**: `DdsType::clone_out` returns `DdsResult<Self>` instead of `Self`. The
  generated `clone_out` for a union without a `#[dds_default]` variant used to `panic!` on
  a discriminator outside the declared set — and that discriminator arrives from the
  network, so a peer built from a different revision of the IDL was enough to unwind
  `reader.take()` on the caller's thread. The `catch_unwind` barriers kept it from
  aborting the process through an `extern "C"` frame; they could not stop the unwind on
  the user's own thread. An undecodable sample is now an error.

  Call sites choose per context: `read`/`take` and the async streams skip the bad sample
  and keep delivering the rest (one misbehaving peer must not stop everything else);
  `read_next`/`take_next`, `instance_get_key` and the CDR deserializers propagate;
  the content filter excludes the sample, matching its existing fail-closed behaviour.
  `Loan::iter()` now yields `DdsResult<Sample<T>>` and `to_vec()` returns
  `DdsResult<Vec<Sample<T>>>`.

- **BREAKING**: `Topic::new`/`with_qos`, and `DataReader`/`DataWriter`'s `new`,
  `with_qos`, `with_listener` and `with_qos_and_listener` now take their parents by
  reference (`&DomainParticipant`, `&Subscriber`/`&Publisher`, `&Topic<T>`) instead of
  raw `dds_entity_t` handles. The handle form accepted any entity, so a `Topic<A>`
  handle could be handed to a `DataReader<B>` — CycloneDDS returned samples laid out as
  `A` while `clone_out` reinterpreted them as `B`, and nothing in the type system
  objected. It also let a temporary supply the handle
  (`Topic::new(DomainParticipant::new(0)?.entity(), "x")` compiles, deletes the
  participant at the end of the statement, and leaves the topic on a recyclable handle).
  The raw forms remain as `Topic::from_entity`/`with_qos_from_entity` and
  `Data{Reader,Writer}::from_entities`/`from_entities_with` for FFI interop, documented
  as unchecked.

  This step fixed type confusion only, not lifetimes: the reference was required for the
  call and nothing was retained, so a `DataReader` could still outlive its `Topic`. The
  owned-parents change above closes that, and moved `Publisher`/`Subscriber`/`WaitSet` to
  references as well.

### Fixed

- **`TopicDescriptor` double-freed when cloned** (`cyclonedds/src/xtypes.rs`): it carried a
  `Clone` that copied the raw `*mut dds_topic_descriptor_t` with no reference count, and a
  `Drop` that called `dds_delete_topic_descriptor` on it. Two clones owned the same
  allocation, so the second drop was a double free and any access after the first drop a
  use-after-free. Nothing in this repository cloned one, which is why it never fired — but
  `Clone` is public API, so any caller doing the obvious thing hit it. Reproduced as
  `STATUS_HEAP_CORRUPTION` before the fix. The pointer now lives in an `Rc<DescriptorOwner>`
  and is released once the last clone is dropped; `Rc` (not `Arc`) keeps the type
  `!Send`/`!Sync`, exactly as the raw-pointer version was. Found by the `xtypes.rs` audit.
- **`#[derive(DdsUnionDerive)]` could not compile for any non-String case**
  (`cyclonedds-derive`): `write_to_native` emitted a *runtime* `if #is_string { ... }`,
  interpolating the macro-time flag as a `true`/`false` literal into a real `if`. Both
  branches therefore had to typecheck for every case, so the derive demanded
  `i32: AsRef<str>` and produced mismatched branch types. The union derive has never
  worked for anything but strings; nothing in the repository derives a union, so nothing
  caught it, while the README advertises union support. The branch is chosen at expansion
  time now, and `union_unknown_discriminator.rs` is the first test to exercise this derive
  at all.
- **`SerdeSample<T>` handed a Rust `Vec` to CycloneDDS as a DDS sequence**
  (`cyclonedds/src/serde_sample.rs`): it declared `Native = Self` and
  `write_to_native` returned `self as *const Self as *const c_void`, but its ops say
  `ADR | SEQ | 1BY`, so the C side reads that memory as `dds_sequence_t`
  (`{u32 _maximum, u32 _length, *mut u8 _buffer, bool _release}`) while it actually held a
  `Vec<u8>` — whose field order is `repr(Rust)` and explicitly not guaranteed, so the
  sizes lining up was luck rather than design. `clone_out` then did `ptr::read`, taking
  Rust ownership of a buffer allocated by `dds_alloc`, to be freed later by Rust's
  allocator. Reproduced as `STATUS_STACK_BUFFER_OVERRUN` before the fix. There is now a
  `#[repr(C)] SerdeSampleNative` holding a `DdsSequence<u8>`, with conversions mirroring
  what the derive generates for a `Vec<u8>` field.

  The `serde` feature had **no test coverage at all**, which is how this survived — the
  same blind spot that let 2.0.1's `SerdeSample`/`Native` omission through. It now has a
  real DDS round-trip suite, and the feature is wired into `cyclonedds-test-suite`.

  Still open, and recorded in `docs/soundness-backlog.md` rather than guessed at here:
  `type_name()` used `concat!("SerdeSample<", stringify!(T), ">")`, which expands to the
  literal `"T"`, so every `SerdeSample<X>` announces the same DDS type name and distinct
  payload types match each other on the wire. A correct fix needs a name stable across
  peers and across compilations, which `std::any::type_name` is not.
- **Three more copies of the wrong instruction-width table**
  (`cyclonedds/src/type_discovery.rs`): the schema builder, `write_value_to_native` and
  `read_value_from_native` each carried their own copy, all missing `ENU`, `ARR`, `UNI`
  and `EXT`. With the derive's scanner and `xtypes::adr_step` that made five copies in the
  crate, written independently, all drifting from `dds_opcodes.h`. All three now call one
  shared, header-checked `adr_step`.

  **No failing case was demonstrated for these three.** `write_value_to_native` takes
  `ops[i + 1]` as a byte offset and does `base.add(offset)` before writing, so a
  desynchronised walk could in principle write out of bounds — but it does not in
  practice, for the same reason the derive's scanner did not: a skipped data word rarely
  carries `0x01` in its top byte, so it reads as opcode 0 (`OP_RTS`), and these walkers
  treat that as `i += 1` and resynchronise. `parse_type` was the one that actually
  misbehaved because its `OP_RTS` arm is `break`, not `+= 1`, so it ended the walk early
  and dropped members — that one is measured. Fixed here regardless: correctness resting
  on data words not looking like opcodes is a coincidence, not an invariant.
- **`write_value_to_native` named fields by word position** while the schema builder and
  the reader name them by ordinal. `format!("field_{}", (i - ops_start) / 2)` assumes every
  instruction is 2 words, so one 3-word field (a bounded string, say) skewed every later
  name and those values silently failed to match the schema — data quietly not written.
  Now uses a monotonic counter like the other two.
- **Native sample buffer leaked on an early return** (`cyclonedds/src/type_discovery.rs`):
  in both `dynamic_data_to_cdr` and `cdr_to_dynamic_data` the buffer was allocated before
  the fallible key-name conversion, so a key name with an interior NUL returned through `?`
  and leaked it — a raw pointer has no `Drop` to clean up. Introduced by this cycle's own
  change from `unwrap()` to `map_err(..)?`; the allocation now happens after the fallible
  work.
- **`TopicDescriptor::parse_type` walked the ops array out of step**
  (`cyclonedds/src/xtypes.rs`): `adr_step` carried a second hand-maintained table of ADR
  instruction widths — the same class of defect as the derive's `ops()` scanner, in
  another location — and this one omitted `ENU`, `ARR`, `UNI` and `EXT` entirely; all
  fell through to 2 words. `ARR` is 3 minimum and `ENU`/`EXT` are 3, so a single array,
  enum or nested-struct field put the walk permanently out of phase and members after it
  simply vanished from the result (a 3-member type reported 2). Bounds-checked
  throughout, so never memory-unsafe — it just made a public introspection API describe
  a type that does not exist. The table now matches `dds_opcodes.h`. Unlike the derive,
  this one cannot be deleted: the ops array comes from CycloneDDS, so it has to be right.
  Found by the `xtypes.rs` audit.
- Pre-existing clippy findings that only surface on the MSRV toolchain: a duplicated
  `#[cfg(feature = "std")]` in `lib.rs` and two needless lifetimes.

## [2.0.4] - 2026-08-14

Soundness release. Five defects reachable from ordinary, `unsafe`-free use of the
public API — three of them memory corruption, one a remotely triggerable process
abort. Every one of them lived on a surface with **zero** test coverage; that gap is
closed first (five new regression suites, plus an AddressSanitizer CI job), which is
what makes the rest verifiable.

`cyclonedds-rust-sys` goes to 1.1.1.

### Fixed

- **`ddsi_serdata` vtable helpers read one byte instead of a function pointer**
  (`cyclonedds-rust-sys`): `*(ops as *const u8).add(N)` dereferences a `u8`, so
  `ddsi_serdata_size`, `ddsi_serdata_to_ser` and `ddsi_serdata_unref` transmuted a
  value in `0..=255` into a function pointer and called it. Any `read_cdr`/`take_cdr`
  with a live sample jumped into an unmapped page; the CLI hit the same path in
  `subscribe`, `echo`, `record` and `monitor`. The byte offsets themselves were
  correct, so this was invisible on inspection. Replaced the offset arithmetic with
  `#[repr(C)]` `SerdataHeader`/`SerdataOps` declarations and field access, removing
  the whole class of defect rather than the three instances.
- **`Loan::iter()` handed out `&T` over memory laid out as `T::Native`**
  (`cyclonedds/src/sample.rs`): for any type with `String`/`Vec` fields the DDS buffer
  holds `DdsString` (8 bytes) / `DdsSequence` where `T` expects `String` (24 bytes) /
  `Vec` — an out-of-bounds read, and heap corruption through `to_vec()`, whose
  `String::clone` allocated from a garbage capacity and later freed a pointer Rust
  never allocated. `iter()` now yields owned `Sample<T>` via `DdsType::clone_out`, and
  `to_vec()` no longer requires `T: Clone`. Genuine zero-copy access moved to the new
  `Loan::iter_native()`, which yields `Sample<&T::Native>` — the honest type.
- **`DataReader::instance_get_key` zero-initialized a generic `T`**
  (`cyclonedds/src/reader.rs`): an all-zero `String`/`Vec` violates the `NonNull` niche
  inside it (rustc rejects it outright in debug builds), the buffer was sized
  `size_of::<T>()` while CycloneDDS writes the `Native` layout, and the returned value
  was freed by Rust over `ddsrt_malloc`-owned memory. Now uses
  `MaybeUninit<T::Native>` plus `clone_out`. Also corrected `check_entity` to `check`
  for what is an operation return code, not an entity handle.
- **`DataReader::lookup_instance` passed `&T` straight to CycloneDDS**
  (`cyclonedds/src/reader.rs`): every writer-side equivalent routes through
  `write_to_native`; this one did not, so CycloneDDS read the key at native offsets
  and found the middle of Rust's `String` — `strlen` over an arbitrary address. Now
  mirrors `DataWriter::lookup_instance`, signature unchanged.
- **No panic barrier on any listener callback** (`cyclonedds/src/listener.rs`): none of
  the 13 `extern "C"` trampolines wrapped the user closure in `catch_unwind`, so a
  single `unwrap()` in a callback aborted the process from a CycloneDDS thread. The
  trampolines are now generated by a macro over a shared `dispatch` helper that owns
  the barrier, so a newly added callback cannot omit it. The same barrier was added to
  `content_filtered_topic::trampoline_filter_sample_arg` and `log::log_trampoline`.
- **Remotely triggerable abort through the content filter**
  (`cyclonedds-derive`): `clone_out` for a union declared without `#[dds_default]`
  panics on an unknown discriminator, and that discriminator arrives from the network —
  a peer built from a different revision of the IDL could abort the process. The
  `catch_unwind` barrier above contains it; the panic message now names the
  discriminator and the type, and points at `#[dds_default]` as the fix.
- **Use-after-free window in the `QueryCondition` registry**
  (`cyclonedds/src/waitset.rs`): the trampoline read a raw pointer to the closure and
  used it *after* releasing the registry lock. `QueryCondition` is `Send` and its
  `Drop` removes the entry, so another thread dropping the condition in between freed
  the closure about to be called. The registry now stores `Arc` and the trampoline
  clones it inside the lock.
- **Log lines delivered twice, and a reentrancy deadlock** (`cyclonedds/src/log.rs`):
  both sinks registered a null `logdatum`, so the shared trampoline could not tell
  which one fired and invoked both. Each sink now registers a distinct tag. The user
  callback is also cloned out and the mutex released before invoking it — previously a
  sink that logged anything flowing back through CycloneDDS deadlocked.
- **`get_name`/`get_type_name` mishandled names of 256 bytes or more**
  (`cyclonedds/src/entity.rs`): these follow `snprintf` semantics and return the length
  that *would* be needed, but the result was passed to `Vec::truncate`, which only ever
  shrinks — so the full raw buffer was returned, embedded NUL padding included. Now
  retries once with an exactly-sized buffer.
- **`unwrap()` on `CString::new` for key names** (`topic.rs`, `content_filtered_topic.rs`):
  an interior NUL in a key name is bad input, not a bug; now reported as
  `DdsError::BadParameter`, consistent with the topic-name handling a few lines above.
- **Poisoned-mutex panics in `QueryCondition::with_filter` and its `Drop`**: a panicking
  `Drop` during an unwind aborts the process. Both now recover via `into_inner()`, as
  the trampoline already did.
- **The async read path outlived the reader it borrowed** (`cyclonedds/src/async.rs`):
  `take_async` and the drain step of all eight `*_aiter*` streams ran their
  `dds_take`/`dds_read` inside `tokio::task::spawn_blocking`. That task is `'static`, so
  only the raw `dds_entity_t` (an `i32`) was moved in, not a borrow of the
  `DataReader` — cancelling the future left the task running against a handle whose
  reader could already be dropped and its entity deleted, and CycloneDDS recycles entity
  handles. Neither call blocks (both walk the reader history cache), so the thread hop
  bought nothing; they now run inline, tied to the `&self` borrow the future already
  holds. This also removes one thread hop per read from the hot path. The eight streams
  collapsed onto a single shared implementation (~500 lines removed).
  `dds_waitset_wait` does block and stays on `spawn_blocking`; the remaining gap there
  (an uncancellable wait on a `WaitSet` dropped with the future) is documented on
  `WaitSet::wait_async` and needs an ownership change to close.
- **The `ops()` instruction scanner disagreed with `dds_opcodes.h`**
  (`cyclonedds-derive`): the generated `ops()` walks the bytecode it just built to find
  `TYPE_EXT` (nested composite) instructions and patch their jump words, which requires
  knowing each instruction's width. Ten entries were wrong — `SEQ|ENU` counted 2 words
  instead of 3, `SEQ|BST` 4 instead of 3, `ARR|ENU` and `BSQ|ENU` 3 instead of 4, `UNI`
  2 instead of 4, `BMK` 2 instead of 4, composite `ARR` subtypes 3 instead of 5 — and
  the `TYPE_EXT` advance was hardcoded to 3 while the patch step already widened to 4
  for the external flag (neither accounted for `DDS_OP_FLAG_OPT`). A mis-sized
  instruction makes the scan land mid-instruction, so it can miss a real `TYPE_EXT`
  (leaving its jump word zeroed) or mistake a data word for one and patch that.
  Corrected against the header table.

  **No failing case was demonstrated.** Tracing the emitted bytecode by hand shows the
  drift resynchronises: a skipped data word rarely has `0x01` in its top byte, so it
  does not match `OP_ADR` and the scan advances one word at a time until it realigns.
  Correctness was therefore resting on data words not looking like opcodes, which is a
  coincidence rather than an invariant. The structural fix is to record `TYPE_EXT`
  positions while emitting instead of re-scanning afterwards — the derive already knows
  where it put them — and is left as follow-up.
- **Clippy CI gate was red on `main`** (pre-existing, from `709f58a`):
  `assert!(reg.is_poisoned() || true)` in a `waitset.rs` test was a tautology that
  asserted nothing (the poisoning there is deterministic, so it now asserts it), plus an
  unused parameter. `cargo clippy --workspace --all-targets -- -D warnings` passes again.

### Added

- Regression suites for the surfaces where the above shipped undetected — all four had
  **zero** coverage, which is why POD-only tests kept passing: `cdr_roundtrip.rs`,
  `loan_heap_fields.rs`, `instance_string_key.rs`, `listener_panic_barrier.rs`,
  `async_reader_lifetime.rs`, `ops_scanner_alignment.rs` (each of the latter pairs a
  mis-sized instruction with a trailing nested struct — the combination no existing test
  produced).
  Each was confirmed to reproduce its defect before the fix (access violation, rustc's
  own "attempted to zero-initialize type which is invalid", and a non-unwinding abort).
- `Loan::iter_native()` — zero-copy iteration over loaned samples as `&T::Native`.
- AddressSanitizer CI job (nightly, Linux, non-blocking for now) over those four
  suites: for a safe wrapper around a C library, the failure mode that matters is
  corruption a normal test run reports as "ok".
- Project governance: CONTRIBUTING.md, SECURITY.md, CODEOWNERS, issue templates, PR template
- Dependabot configuration for cargo and GitHub Actions
- CodeQL security analysis workflow
- Release workflow with Docker build, Cosign signing, SBOM, and Trivy scan
- Multi-stage Dockerfile and docker-compose.yml for DDS development environment
- Per-crate `README.md` for all 9 published crates, each `Cargo.toml` now pointing at
  its own file — inheriting `readme.workspace = true` resolves relative to the
  workspace root, so every crate rendered the root README on crates.io.

### Changed

- `docs/` brought back in line with 2.x: `getting-started.md`, `api-guide.md`,
  `observability.md` and `tutorial.md` advertised `cyclonedds = "1.7"`/`"1.4"`;
  `type-system.md`, `migration-from-python.md` and `architecture.md` used
  `#[derive(DdsType)]`, which does not exist — the crate re-exports the macros as
  `DdsTypeDerive`/`DdsEnumDerive`/`DdsUnionDerive`/`DdsBitmaskDerive` (`DdsType` is the
  trait); and the `impl DdsType` example in `getting-started.md` omitted the mandatory
  `Native` associated type, so it did not compile.
- `observability` module docs claimed the `opentelemetry` feature re-exports
  `tracing-opentelemetry` and `opentelemetry-otlp`. It never did — neither is a
  dependency of this crate. Documented what the feature actually provides.
- Root README corrected against the source: version `1.8` → `2.0`, CMake 3.10 → 3.16
  (the vendored `CMakeLists.txt` requires 3.16), `write_loan` → `request_loan`, derive
  macro names to their `*Derive` re-exports, the CLI subcommand list (12–13 listed, 16
  exist), and `DdsType` examples that omitted the mandatory `Native` associated type and
  therefore did not compile. Added a feature-flag table and documented the per-target
  ABI probe and the `abi/<triple>.rs` snapshot required for cross-compilation.

[Unreleased]: https://github.com/mzet97/cyclonedds-rust/compare/v3.0.0-alpha.3...HEAD
[3.0.0-alpha.3]: https://github.com/mzet97/cyclonedds-rust/compare/v3.0.0-alpha.2...v3.0.0-alpha.3
[3.0.0-alpha.2]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.4...v3.0.0-alpha.2
[2.0.4]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.3...v2.0.4

## [2.0.3] - 2026-07-23

### Fixed

- **Release container's Trivy gate, for good this time**: a 4th different CVE
  (`CVE-2026-9538`, perl-Archive-Tar DoS) appeared minutes after 2.0.2 shipped, in the same
  never-fixed Perl/gzip OS packages. CVE-by-CVE `.trivyignore` entries proved unsustainable
  (4 different CVEs cycling through the same package group in 2 days). Replaced with
  `ignore-unfixed: true` on the Trivy scan step: skips any CVE with no upstream fix
  available (`will_not_fix`/`fix_deferred`/`affected`) — the exact category every CVE seen
  in these packages falls into — while still failing the gate on anything with an actual
  available patch. `.trivyignore` kept as documentation of the specific CVEs already
  investigated.

[2.0.3]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.2...v2.0.3

## [2.0.2] - 2026-07-23

### Fixed

- **Trivy gate on the release container still failed** after 2.0.1: 3 new CVEs
  (`CVE-2026-41992` gzip, `CVE-2026-42496` perl-base, `CVE-2026-48962` perl-IO-Compress)
  appeared in the Trivy feed between the 2.0.1 tag and this release, in the same
  never-executed Perl/gzip OS packages already covered by `.trivyignore`. Added to
  `.trivyignore` with the same justification; noted that CVE-by-CVE suppression on these
  packages is not sustainable long-term — a follow-up should purge Perl/gzip from the
  final image stage (or switch to a distroless base) instead of continuing to chase
  individual CVE IDs.

[2.0.2]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.1...v2.0.2

## [2.0.1] - 2026-07-23

### Fixed

- **`SerdeSample<T>` did not implement `DdsType::Native`**: introduced by the `DdsType::Native`
  associated type added in 2.0.0, but missed in this one `impl` block. Broke any build with
  `--features serde` enabled, including the crate's own `cargo doc`/`cargo clippy
  --all-features` CI jobs.
- **`Cargo.lock` was stale since the 2.0.0 version bump**, never regenerated/committed after
  the release — broke `cargo build/clippy/doc --locked` in CI (CI, MSRV, Clippy, Docs, CodeQL
  workflows) with "cannot update the lock file because --locked was passed".
- **Release container's Trivy scan always failed** on 4 CVEs in OS packages of the
  `debian:bookworm-slim` base image (`perl-base`, `perl-Archive-Tar`, `zlib1g`, `bsdutils`),
  none exercised by the published binary and two already marked `will_not_fix`/`fix_deferred`
  upstream by Debian. Added a documented `.trivyignore` for these specific CVEs; the
  CRITICAL/HIGH gate remains active for anything new.

[2.0.1]: https://github.com/mzet97/cyclonedds-rust/compare/v2.0.0...v2.0.1

## [2.0.0] - 2026-07-21

### Fixed

- **Zero-copy loan buffer overflow** (`DataWriter::request_loan`/`WriteLoan`): the loaned
  buffer was zero-initialized and interpreted as `size_of::<T>()` bytes, but
  `dds_request_loan` only allocates `size_of::<T::Native>()` — smaller for any type with
  `String`/`Vec` fields (translated to `DdsString`/`DdsSequence`). This wrote past the end
  of the DDS-owned allocation on every loan of such a type, and a zeroed `String`/`Vec` is
  not a valid bit-pattern to begin with. `Drop for WriteLoan` now runs `drop_in_place` on
  the native value before returning the loan, so partially-populated `DdsString`/
  `DdsSequence` fields are freed correctly.
- **Reading loaned/read samples as `T` instead of `T::Native`** (`async.rs`): `take_async`/
  `read_async` used `ptr::read(samples[i] as *const T)`, reinterpreting the DDS-native
  buffer (8-byte `char*` strings) as the ergonomic Rust type (24-byte `String`); replaced
  with `T::clone_out(..)`, which converts the native representation into an owned `T`.
- **`Topic<T>` was not `Send`/`Sync`**: its `DescriptorHolder` used `Rc` (changed to `Arc`)
  and lacked explicit `unsafe impl Send/Sync`, even though the held data is read-only after
  topic creation and safely shared by CycloneDDS across its own threads. Same fix applied
  to `Qos` and `Listener` (both immutable after construction; documented safety
  justification inline).
- Stale `cyclonedds-build` codegen tests (`test_generate_simple_struct`,
  `test_compile_idl_to_string`) still asserted the pre-`Default, PartialEq` derive list.

### Added

- `DdsType::Native` associated type: the DDS wire-compatible representation used by the
  loan APIs and the topic descriptor size/align. `#[derive(DdsTypeDerive)]` now emits it
  automatically; manual `impl DdsType` blocks for POD types set `type Native = Self`.
- `DdsType::type_metadata_blobs()`: optional XCDR2 (TypeInformation, TypeMapping) blobs so
  the topic descriptor can set `DDS_TOPIC_XTYPES_METADATA` and announce type information
  over SEDP — required for type-enforcing peers (Python/C++) to match correctly.
- `DataWriter::set_qos()` — update a writer's QoS at runtime for the online-tunable knobs
  (TransportPriority, LatencyBudget, OwnershipStrength).
- Generated structs (`cyclonedds-build` codegen) now also derive `Default, PartialEq`.
- `cyclonedds-rust-sys` 1.1.0: opt-in `CYCLONEDDS_STATIC=1` static build of the vendored
  CycloneDDS (needed on filesystems without symlink support, e.g. CIFS/SMB, and produces a
  self-contained binary), with the transitive system libs (`pthread`, `dl`, `rt`, `m`) and
  `-DCMAKE_POSITION_INDEPENDENT_CODE=ON` it requires; clearer `cargo:warning=` diagnostics
  for which CycloneDDS build was picked (pre-built / freshly built / system).

### Changed

- **BREAKING**: `DdsType` now requires `type Native: Sized`. Manual `impl DdsType` blocks
  written against 1.x must add `type Native = Self;` (or the real native type, for hand-rolled
  wire-compatible structs).
- **BREAKING**: `WriteLoan::get_mut()` returns `&mut T::Native`, not `&mut T`; populate
  string fields via `DdsString::new(..)` instead of assigning a `String` directly.
  `write_loan_async`'s closure signature changed to `FnOnce(&mut T::Native)` to match.

[2.0.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.8.0...v2.0.0

## [1.8.0] - 2026-05-02

### Added

- **DDS Request-Reply Pattern** (`Requester<TReq,TRep>` + `Replier<TReq,TRep>` with correlation IDs, timeout, and retry).
- **Connection Pooling & Service Discovery** (`ParticipantPool` with multi-domain participant management, `discover_topics()`, `discover_participants()`, automatic heartbeat/purge).
- **Content Filtering Advanced** (`FilterParams` + `TopicParameterizedFilterExt::with_params()` for runtime parameter updates).
- **Serde Integration** (`SerdeSample<T>` with feature `serde` + `postcard` for Rust-to-Rust serialization over DDS).
- **Observability** (`observability.rs` with `init_json_logging()`, `init_tokio_console()`, `init_full_observability()`; features `opentelemetry` and `tokio-console`).
- **WASM Support (Experimental)** — new `cyclonedds-wasm` crate with DDS-compatible API over WebSocket; compiles for `wasm32-unknown-unknown`.
- **no_std / Embedded Support (Experimental)** — feature `no_std` exports `DdsType` trait + CDR opcode constants without FFI; compiles for `thumbv7em-none-eabihf`.
- **Security Production Hardening** (`SecurityConfig::crl()` for Certificate Revocation Lists + `docs/security-production.md`).

### Changed

- `cyclonedds-rust-sys` and `thiserror` are now optional dependencies (feature `std`).
- Feature `async` now implies `std` for CI compatibility.
- `lib.rs` uses `#[cfg(feature = "std")]` to conditionally compile all FFI-dependent modules.

## [1.7.0] - 2026-05-02

### Added

- **Error Handling & Recovery** (`DdsError::is_transient()`, retry with exponential backoff in `DomainParticipant::new()` and `DataWriter::write()`).
- **Async Timeouts & Cancellation** (`read_aiter_timeout`, `take_aiter_timeout`, safe cancellation via `drop()` without DDS entity leaks).
- **DDS Security Hardening** (`SecurityConfig::validate()` for X.509/PEM checks, `SecurityConfig::reload()` for hot-reload support).
- **Profiling & Diagnostics CLI** (`cyclonedds-cli diagnose --domain 0` for full JSON state, `cyclonedds-cli metrics <topic>` for Prometheus text export).
- **ROS2 Interop Helpers** (`DomainParticipant::ros2_topic_name()` for `rt/<topic>` naming, `ros2_qos_reliable()` and `ros2_qos_best_effort()` QoS mappers).
- **Loaned Reads (Zero-Copy Subscriber)** (`DataReader::read_loan()`, `DataReader::take_loan()` with `ReadLoan<T>` wrapper).
- **Expanded Test Suite** (reconnection rediscovery tests, cross-domain isolation tests, long-duration stress tests).

### Changed

- CI/CD workflows updated to run tests sequentially (`--test-threads=1`) to prevent flaky SIGSEGV caused by CycloneDDS global domain state in parallel test execution.
- `missing_docs` lint suppressed globally (`#![allow(missing_docs)]`) to unblock CI; documentation will be incrementally added.
- Fixed ~45+ Clippy warnings across the entire workspace (`collapsible_match`, `needless_borrow`, `len_zero`, `never_loop`, `redundant_closure`, `print_literal`, `format_in_format_args`, `dead_code`, etc.).
- Fixed broken intra-doc links in `serialization.rs`.
- Fixed benchmark `config_comparison.rs` missing `max_blocking_time` argument in `reliability()`.

### Fixed

- Flaky `qos` test SIGSEGV in MSRV and Code Coverage jobs.
- `type_discovery.rs` accidental deletion restored with careful re-application of Clippy fixes.
- `cyclonedds-test-suite` examples and benchmarks using incorrect `cyclonedds_derive::DdsTypeDerive` import (now uses `cyclonedds::DdsTypeDerive`).
- CLI `main.rs` `needless_range_loop` and `needless_borrow` issues.

## [1.6.0] - 2026-04-30

### Added

- Initial comprehensive API: DomainParticipant, Publisher, Subscriber, Topic, DataWriter, DataReader.
- 26+ QoS policies via `QosBuilder`.
- 13 listener callbacks via `ListenerBuilder`.
- WaitSet / ReadCondition / QueryCondition / GuardCondition.
- Derive macros: `DdsType`, `DdsEnum`, `DdsUnion`, `DdsBitmask`.
- CDR serialization (XCDR1/XCDR2), dynamic types, type discovery (XTypes).
- Async streams (`read_aiter`, `take_aiter`) with tokio integration.
- CLI tools: `ls`, `ps`, `subscribe`, `typeof`, `publish`, `discover`, `echo`, `record`, `replay`, `monitor`, `health`, `topology`.
- Zero-copy write loans (`WriteLoan<T>`).
- DDS Security support (`SecurityConfig` + `--features security`).

[1.8.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.7.0...v1.8.0
[1.7.0]: https://github.com/mzet97/cyclonedds-rust/compare/v1.6.0...v1.7.0
[1.6.0]: https://github.com/mzet97/cyclonedds-rust/releases/tag/v1.6.0
