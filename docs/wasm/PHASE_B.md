# Fase B — separação mínima (working tree, sem commit)

Branch/SHA na execução: `main` / `f0f766b` (prompt citava `134c92c`;
revalidado contra o checkout real — afirmações abaixo valem para `f0f766b`).
Estado: só arquivos novos/modificados listados aqui; nada commitado, nada
pushed (regra dura).

## O que foi separado

- `cyclonedds`: `std` NÃO puxa mais `libddsc`. Features:
  `std` (puro) / `native` (= `sys` + `std`) / `async` (exige `native`);
  `default = ["std","native","async"]` preserva a superfície histórica.
  Módulos com FFI foram de `#[cfg(feature="std")]` para
  `#[cfg(feature="native")]` em `cyclonedds/src/lib.rs`; helpers puros
  (`DdsError`, `DdsResult`) e derives ficam em `std`. `error::check` /
  `check_entity` / `err_*` (FFI) → `native`.
- `cyclonedds-proto` (novo, sem `sys`/`wasm-bindgen`/`web-sys`):
  contratos portáteis proto v0 — `Control` (hello/register/subscribe/
  unsubscribe/bye/ack/error), `Qos` + `check_qos` (só best-effort/volatile;
  resto → `Unsupported`, tratamento per REQ-QOS-01), `DataFrame` binário LE
  com `decode` never-panic, `LegacyEnvelope` (formato atual, atrás de flag).
- `cyclonedds-wasm`: depende de `cyclonedds-proto`, NUNCA de `cyclonedds`
  nativo; `wasm-bindgen`/`js-sys`/`web-sys` só em
  `[target.cfg(wasm32).dependencies]`. Envelope agora via `proto`
  (wire-idêntico ao legado). `create_*_with_qos` aplica `check_qos`.
  Fallback host: mesma API linka fora do browser; I/O → `NotConnected`
  tipado (lacuna explícita, sem discretização).
- `dds-wasm-consumer` (novo bin): só `cyclonedds-wasm` + `proto` + serde.
  `cargo tree -p dds-wasm-consumer -i cyclonedds-rust-sys` → "did not match
  any packages" (host e wasm32). Build wasm32 OK.
- Fixtures: `cyclonedds-proto/tests/fixtures/` (hello/register/
  error_unsupported_qos/legacy_envelope) + `tests/proto_roundtrip.rs`
  (8 testes: goldens, QoS negativo, proto futuro → Unsupported, roundtrip
  byte-idêntico do frame, prefixes truncados/adversariais never-panic).

## Evidência de testes (esta sessão)

- `cargo test -p cyclonedds-proto`: 8 passed.
- `cargo test -p cyclonedds-wasm` (host): 3 passed.
- `cargo run -p dds-wasm-consumer` (host): OK ("transport NotConnected").
- `cargo build -p dds-wasm-consumer --target wasm32-unknown-unknown`: OK.
- `cargo check -p cyclonedds --no-default-features --features std`: limpo,
  sem warnings; `cargo tree --edges normal` sem `sys` (só via dev-deps).
- `cargo check -p cyclonedds` (default nativo): OK.
- `cargo check --workspace`: OK.
- `cargo test -p cyclonedds --lib`: 25 passed / 3 failed —
  IDÊNTICO no HEAD pristino (`git worktree` em `f0f766b`, depois removido):
  `participant_retains_listener…`, 2× `participant_pool…`, causa
  `ReturnCode(-1)` + "enp4s0: does not match an available interface"
  (rede do sandbox). Pré-existente, sem regressão.

## Lacunas que continuam lacunas

Controle versionado no transporte, dispatcher N-readers, `await open`,
cleanup de `Closure`s, CDR no plano de dados, bridge de referência,
reliable/durability (T1–T10 em `TASKS.md`).
