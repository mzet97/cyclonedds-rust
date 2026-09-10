# Fase H — pacote consumível, CI bloqueante, benchmarks, relatório final

Branch/SHA na execução: `main` / `f0f766b` (prompt citava `134c92c`;
revalidado — vale `f0f766b`). `git status --short` no início mostrava
`M Cargo.lock, Cargo.toml, cyclonedds-wasm/*, cyclonedds/*` +
untracked `cyclonedds-proto/ dds-wasm-bridge/ dds-wasm-consumer/
dds-wasm-guest/ docs/wasm/` (fases B–G no working tree). Nada commitado,
nada pushed (regra dura). Sem discretização: lacuna não implementada
continua lacuna; `Unsupported` prova tratamento, não implementação.

## Arquivos alterados/criados nesta fase (só working tree)

- `cyclonedds-wasm/src/bigint.rs` (novo): mapeamento u64/i64 <->
  decimal exato + `u64_needs_bigint` + 4 testes host.
- `cyclonedds-wasm/src/js.rs` (novo, só `wasm32`): `WasmClient::connect`
  (init assíncrona via `Promise`, resolve após `onopen`, reject tipado,
  timeout via `setTimeout` quando disponível), `send_echo`/`on_echo`/
  `close` idempotente, `u64_to_bigint`/`bigint_to_u64`/`i64_to_bigint`/
  `bigint_to_i64` (sempre `BigInt`, nunca `Number`).
- `cyclonedds-wasm/src/lib.rs`: `mod bigint` (+ re-exports),
  `mod js` (`cfg wasm32`).
- `cyclonedds-wasm/Cargo.toml`: `criterion` dev-dep + bench
  `echo_codec` (nenhuma dep nova no grafo wasm).
- `cyclonedds-wasm/benches/echo_codec.rs` (novo, criterion).
- `cyclonedds-wasm/js/`: `package.json`, `index.d.ts` (handwritten),
  `index.mjs`, `example.mjs` (node, BigInt + uso documentado),
  `example.html` (browser).
- `cyclonedds-wasm/examples/echo_host.rs` (novo, executável no host).
- `dds-wasm-bridge/benches/wasm_vs_native.rs` + `Cargo.toml`
  (`criterion` dev-dep): codec puro vs baseline libddsc.
- `.github/workflows/wasm.yml` (novo): 5 jobs bloqueantes, sem
  `continue-on-error`, sem `|| true`.
- `docs/wasm/{TUTORIAL_H,BENCH_H,PHASE_H}.md` (este último = relatório).

## Comandos + resultados (observados nesta sessão)

| comando | resultado |
|---|---|
| `cargo test -p cyclonedds-proto` | 6 + 8 = 14/14 ok |
| `cargo test -p cyclonedds-wasm` | 10/10 ok (6 gateway + 4 bigint novos) |
| `cargo test -p dds-wasm-guest` | 5/5 ok |
| `CYCLONEDDS_URI=...lo cargo test -p dds-wasm-bridge -- --test-threads=1` | 9+6+11+3 = 29/29 ok |
| `cargo run -p cyclonedds-wasm --example echo_host` | 4/4 asserções OK (72 bytes, Unsupported, BigInt, NotConnected) |
| `cargo check --workspace` / `-p cyclonedds-wasm --target wasm32-unknown-unknown` / `RUSTFLAGS="-D warnings"` idem | Finished, zero warnings |
| `cargo tree -p cyclonedds-wasm --target wasm32-unknown-unknown` | sem `cyclonedds-rust-sys`; só wasm-bindgen/js-sys/web-sys |
| `cargo bench -p cyclonedds-wasm --bench echo_codec` | 30.1 / 46.7 / 57.5 / 91.0 ns (ver BENCH_H) |
| `cargo bench -p dds-wasm-bridge --bench wasm_vs_native` | 542.9 / 541.7 ns baseline nativo |
| `cargo build -p cyclonedds-wasm --target wasm32-unknown-unknown --release` | `cyclonedds_wasm.wasm` = 687 282 bytes |
| `node --check js/{example,index}.mjs` | OK |
| `cargo test -p cyclonedds --lib` | 25 passed / **3 failed — idênticas ao baseline pristino** (ver abaixo) |
| `cargo bench -p cyclonedds-wasm -p dds-wasm-bridge --no-run` | compila (job bench-smoke do CI) |

## Falhas / não executados (comando + erro + impacto)

- `cargo test -p cyclonedds --lib` → 3 falhas pré-existentes, sem
  regressão: `participant_retains_listener_after_caller_drops_handle`,
  `discovery_wait_does_not_block_other_pool_operations`,
  `participant_lookup_reuses_stored_entity_and_releases_pool_lock`;
  log mostra `enp4s0: does not match an available interface` +
  `ReturnCode(-1)` — sandbox sem a NIC default; idêntico às fases B–F.
  Nenhum arquivo de `cyclonedds/` foi tocado nesta fase.
- `DomainParticipant::new` sem `CYCLONEDDS_URI=...lo` → `ReturnCode(-1)`
  (bloqueio de ambiente já documentado na Fase C; contornado com `lo`).
- `npx --no-install tsc --version` → `canceled due to missing packages`
  (sem `tsc`, sem rede): `index.d.ts` **não validado por compilador** —
  fica como risco aberto, não como garantia.
- `command -v wasm-pack wasm-bindgen wasmtime emcc` → vazio:
  `js/pkg/` nunca gerado, `example.mjs` (imports de `./pkg-node/`)
  **não executado**, `example.html` nunca aberto, `WasmClient` (js.rs)
  **compilado para wasm32 mas nunca executado em browser/node**.
- Engine wasm (Fase G): continua bloqueada no toolchain; nenhum stub
  ddsrt/wasi criado.

## Confronto requisito × evidência (todos os 11 IDs)

| ID | estado | evidência |
|---|---|---|
| REQ-CONN-01 state machine | `parcial` | `js.rs::WasmClient::connect` resolve só após `onopen` (nunca `Ok` antecipado); `close()` idempotente; write-before-open impossível (sem client antes do open). Não executado em browser → parcial, não ok. |
| REQ-CONN-02 await open | `parcial` | `connect(url, timeout_ms) -> Promise<WasmClient>`; timeout/`closed-before-open`/erro de socket viram reject tipado. wasm32-compila; sem run no browser. |
| REQ-CONN-03 cleanup | `parcial` | connect limpa handlers one-shot ao assentar; `close()` faz `set_onmessage(None)` + drop do `Closure`. Os 3 `forget` legados do backend JSON (`lib.rs`) seguem intactos → lacuna parcial preservada. |
| REQ-MUX-01 fan-out | `ausente` | `on_echo` substitui um único callback global; sem mapa `topic→callbacks`, sem contador de drops no browser. Dispatcher existe só no lado nativo (Fase D). Continua lacuna. |
| REQ-PROTO-01 controle | `parcial` | Lado bridge/guest (hello/ack, Fase D, 11/11 re-validados aqui). Lado browser (`cyclonedds-wasm` legado) não envia controle → parcial. |
| REQ-PROTO-02 CDR | `ok` (escopo: `WasmEcho`) | Codec XCDR1 puro, never-panic, byte-idêntico ao libddsc (phase_c 6/6), JSON só atrás de flag; benches H medidos. Fora do escopo: outros tipos IDL. |
| REQ-QOS-01 best-effort | `ok` | `check_gateway_qos` rejeita não-best-effort/volatile com `Unsupported`; testes 6/6 + exemplo host. |
| REQ-QOS-02 reliable | `ausente` (por desenho) | Pedir reliable → `Unsupported` (tratamento, não implementação). Nenhum ACK/sequence implementado; continua backlog explícito. |
| REQ-TYPE-01 stride Native | `ausente` nos perfis wasm | Contrato `topic.rs` é do perfil nativo; wasm cobre só `WasmEcho` (i32/string/seq). Sem discretização. |
| REQ-OBS-01 erros/contadores | `parcial` | `NotConnected`/`Timeout`/`Unsupported` usados de verdade (testes host 10/10); `dropped_overflow` no bridge (Fase D, re-validado). Contadores de decode-error no browser seguem ausentes. |
| REQ-SEC-01 limites bridge | `parcial` | Bridge valida tópico/tamanho/tipo; oversize → `TooLarge` sem derrubar conexão (suítes C/D re-validadas 29/29). `THREAT_MODEL.md` inalterado nesta fase. |

## Perfis suportados × CI bloqueante

`.github/workflows/wasm.yml`: `proto`, `browser-gateway-host`,
`bridge-and-guest`, `wasm32` (inclui `-D warnings` + auditoria
anti-vazamento `cyclonedds-rust-sys` + tamanho do artefato),
`bench-smoke`. YAML validado por parse; jobs ainda não executados no
GitHub (sem push por regra dura — execução remota é risco aberto).
`wasm-engine` deliberadamente sem job (Fase G bloqueada).

## Adendo — campanha 100% real (2026-09-10, working tree, sem commit)

Bug real achado por sonda: tópico de 8 MiB → SIGSEGV (exit 139) dentro
de `dds_create_topic`. RED reproduzido como unit test (signal 11);
GREEN via `MAX_TOPIC_CHARS=1024` validado no `bind()` antes de qualquer
FFI (erro tipado, sem crash). Cobertura 97.92% → **98.14%** linhas
(1940, 36 perdidas), regiões 95.57%, funções 91.06%.

| comando | resultado |
|---|---|
| `cargo test -p ... (6 crates) --all-targets` | **135 passed / 0 failed** (+8: adversarial 5, fd_exhaustion 1, oversize 1, soak 1) |
| `... --test wasm_node` (wasm32, node) | 3/3 ok (pós-mudança no Cargo.toml) |
| `cargo llvm-cov ... --all-targets --show-missing-lines` | só `dds-wasm-bridge/src/lib.rs`: 99, 665-669, 832-837, 946, 948, 975, 980 — todas com prova em EVIDENCE.md (662/829 cobertas pelo soak, 3/3 runs) |
| `RUSTFLAGS="-D warnings" cargo clippy --all-targets` | zero (só build-script do sys) |
| `cargo fmt --all --check` | limpo |

Arquivos novos: `dds-wasm-e2e/tests/adversarial.rs` (158 LOC puras),
`dds-wasm-bridge/tests/fd_exhaustion.rs` (123),
`dds-wasm-e2e/tests/shutdown_race.rs` (soak ~1 s, invariantes duras),
`libc = "0.2"` como dev-dep do workspace (já no lock, 0.2.182).
REQ-SEC-01 fica mais forte: além de tópico/tamanho/tipo, nome de tópico
agora tem cap fail-fast. Achados do caminho: pacote legacy do soak com
`}` extra (zero publishes silenciosos → invariante `samples_in > 0`),
hang de feeder em CLOSE_WAIT (write-timeout 5 s), ambos documentados.

## Requisitos abertos (para o dono do workflow)

1. Rodar `example.mjs`/`example.html` com `wasm-pack` + bridge
   WebSocket (aqui só existe gateway TCP de referência).
2. Validar `index.d.ts` com `tsc --noEmit --strict`.
3. Executar o workflow `wasm.yml` remotamente (push proibido aqui).
4. REQ-MUX-01, REQ-TYPE-01 (wasm), REQ-QOS-02: lacunas preservadas.
5. Engine (Fase G): depende de wasi-sdk + wasmtime + target wasip2.
