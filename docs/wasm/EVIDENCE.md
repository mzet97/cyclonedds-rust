# Evidências

Checkout: branch `main`, SHA `f0f766b724b991b67d5a95f918966c1d1ed6c1c7`, `git status --short` limpo (antes e depois). Prompt citava `134c92c` — diverge; tudo revalidado em `f0f766b`.

## Código (leitura direta nesta sessão)

- `cyclonedds-wasm/src/lib.rs` integral (217 linhas): struct `70-73`, `new`/`onopen` `77-103`, `create_topic` `106-118`, `create_writer` `121-130`, `create_reader`/`set_onmessage` `133-165`, `disconnect` `167-171`, envelope JSON `191-194`, `send_with_str` `197-199`, `WasmDataReader.ws` morto `205-210`, `NotConnected` definido `50,59` e nunca construído, `forget` ×3 em `92,100,158`, doc-limitações `9-13`.
- `Cargo.toml` workspace `2-25`: 12 membros incl. `cyclonedds-wasm`, v3.0.0, edition 2021, rust 1.85.

## Prior results reutilizados (com file:line inspecionado)

- PR1 (wasm 4/6): A1–A5 acima, todos com linhas — aceito.
- PR2 (ambiente 6/6, por execução): rustc 1.95.0, só target x86_64, sem wasm-pack/bindgen-cli/wasmtime, node 24, playwright+chromium, baseline `cargo test -p cyclonedds` 25 passed/3 FAILED (sem UDP no sandbox) — aceito.
- PR3 (C/ddsrt): precedência `build.rs:284-313`, C 11.0.1, `vendor/` vazio, ddsrt `select()`/threads/clocks — aceito como contexto.
- PR4 (FFI): `topic.rs` Safety/stride/descriptor/loans — aceito como contexto nativo.
- PR5 (inventário): workspace/features `std`→async+tokio — aceito.
- PR6 (testes): lista de arquivos de teste + corpus CDR — aceito.

## Comandos rodados nesta sessão

- `git rev-parse --abbrev-ref HEAD` → `main`; `git rev-parse HEAD` → `f0f766b…`; `git status --short` → vazio (limpo).
- `ls docs` (sem `docs/wasm/` — criado nesta sessão), `ls cyclonedds-wasm`, `wc -l lib.rs` (217).

## Correção de baseline (2026-09-10, por execução)

- Os "3 FAILED (sem UDP no sandbox)" do PR2 foram reinterpretados: causa real =
  `CYCLONEDDS_URI` global apontando para NIC inexistente `enp4s0`
  (`enp4s0: does not match an available interface`, `ReturnCode(-1)` em
  `test_derive_keyed_pub_sub`). Com override inline para a NIC real (`enp7s0`,
  `/tmp/cyclonedds-enp7s0.xml`), `test_derive_keyed_pub_sub --test-threads=1`
  **PASSA em 1,00 s**. Loopback DDS nativo funciona nesta máquina; gateway da
  Fase C não está bloqueado por rede. Ação de máquina (fora do repo): corrigir
  `~/.bashrc:50` de `enp4s0` para `enp7s0`.
- Alvos wasm32-unknown-unknown instalado; `cargo check -p cyclonedds-wasm
  --target wasm32-unknown-unknown` passa; `dds-wasm-consumer.wasm` linkado
  (módulo MVP válido); playwright+chromium headless com WebSocket verificado.

## Auditoria independente pós-Fase H (2026-09-10, por execução)

- Re-rodados: `cyclonedds-proto` 14/14, `cyclonedds-wasm` 10/10 (+1 doctest
  ignorado legítimo — exemplo browser), `dds-wasm-guest` 5/5,
  `dds-wasm-bridge` 29/29 (9+6+11+3, com `CYCLONEDDS_URI` p/ `enp7s0`),
  `echo_host` 4/4, `cargo test -p cyclonedds --lib` 25/3 (mesmos 3 do
  baseline pristino, causa `enp4s0` — sem regressão).
- Corrigidos 4 lints clippy no código novo: `repeat().take()` →
  `core::iter::repeat_n` (`echo.rs:95`), alias `SetupBundle` + `if let`
  (`bridge/lib.rs`), 2× `len() >= 1` → `!is_empty()` (`parity.rs`).
  Clippy zerado nos 4 crates novos; `cargo fmt` aplicado (estava sujo).
- HEAD segue `f0f766b`, nada commitado/pusheado (regra dura cumprida).
- Riscos abertos herdados do PHASE_H.md: `WasmClient` nunca executado em
  browser, `index.d.ts` sem `tsc`, CI remoto nunca rodado, engine bloqueada.

## Cobertura: medição e exclusões (`cargo llvm-cov`, 2026-09-10)

- Comando: `cargo llvm-cov -p cyclonedds-proto -p cyclonedds-wasm
  -p dds-wasm-guest -p dds-wasm-consumer -p dds-wasm-bridge -p dds-wasm-e2e
  --all-targets` → **98.14% linhas** (1940, 36 perdidas), 95.57% regiões,
  91.06% funções. Campanhas anteriores: 98.04% (1923+17, 38), 97.92%
  (1923, 40). Baseline: 80.37%.
- **100% linhas** em: `cyclonedds-proto/src/echo.rs`, `dds-wasm-consumer`,
  `dds-wasm-guest`, `dds-wasm-e2e` (tabela; ver nota de metrologia).
- Suite host: **135 passed / 0 failed** (baseline 127; +8 novos).
- `js.rs` (só wasm32): **executado sob node real** — suite durável
  `tests/wasm_node.rs` 3/3 via `wasm-bindgen-test-runner`
  (connect+echo+BigInt+reject) + 12 checagens no harness scratch; servidor
  echo sem dependências em `tests/support/ws_echo_server.mjs`; job CI
  `node-client` adicionado. Sem medição de linha (llvm-cov host não
  instrumenta wasm32) — executado, não medido.
- Novos projetos de teste duráveis (campanha atual):
  - `dds-wasm-e2e/tests/adversarial.rs` (5 testes reais, gateway vivo):
    bye fecha limpo com EOF; ack absorvido sem resposta; proto binário
    novo → `unsupported_proto`; `topic_len` declarado gigante →
    `frame_too_large` sem alocar (prefixo de 20 bytes); legacy com
    300000 values atravessa o DDS e é descartado no re-encode, e o
    echo pequeno seguinte faz roundtrip fim-a-fim.
  - `dds-wasm-bridge/tests/fd_exhaustion.rs` (binário isolado — a tabela
    de fds é do processo): `setrlimit(RLIMIT_NOFILE)` + stuffing com
    `/dev/null` até EMFILE → `try_clone` do pump falha → conexão
    não-servida recebe EOF em vez de pendurar; testemunha prova o
    gateway vivo (hello→ack). RAII restaura limite e fds. Verde em
    0.12 s, determinístico (3 tentativas de contenção).
  - Unit `oversize_topic_name_fails_fast_without_entering_dds`: RED foi
    **SIGSEGV (signal 11)** com loopback; GREEN após a validação.
  - `dds-wasm-e2e/tests/shutdown_race.rs` (soak determinístico, ~1 s):
    5 gateways legacy + 5 binários, 32 conexões cada, pacotes de
    ~200 KB, `close()` mid-burst; invariantes duras por round
    (`frames_in > 0`, `samples_in > 0`, close retorna) + gateway fresco
    servindo no fim. Write-timeout de 5 s nos feeders (após `close()`,
    escrita em socket CLOSE_WAIT com buffers cheios pendura para
    sempre — FIN interrompe leituras, não escritas; hang real
    diagnosticado via gdb e corrigido). Cobriu L662+L829 em 3/3 runs.
- Caudas pós `publish_to_dds → false` **cobertas pelo soak (L662
  legacy, L829 binário)**: a instrumentação temporária (`eprintln`,
  revertida) mediu 4 hits/round cheio e 0 em round só-legacy antes da
  correção — o que revelou que o pacote legacy do soak tinha uma chave
  `}` a mais (`"]}}}"`, JSON inválido, zero publishes silenciosos).
  Corrigido para `"]}}"` + invariante `samples_in > 0` por round (uma
  rajada rejeitada em silêncio nunca mais esconde o pré-requisito).
  A thread DDS não tem caminho de pânico (auditoria completa: sem
  unwrap/expect no caminho de dados; `expect`s só em Mutexes sem
  aninhamento → sem poison), então o close-race é o único gatilho —
  e o soak o dispara de verdade.
- Linhas restantes auditáveis (`--show-missing-lines`, só em
  `dds-wasm-bridge/src/lib.rs`), cada uma com prova de gatilho:
  - L99: **artefato de atribuição do proc-macro, provado por
    comportamento**: o teste `native_value_conversion_is_lossless`
    chama `to_native_value`/`write_to_native` com asserções reais
    (quebrado de propósito, falha sob llvm-cov, restaurado) e todo
    publish E2E atravessa `with_native_ptr` → `write_to_native`
    (roundtrips de tipos com heap EXIGEM o layout Native da arena —
    o default `self as *const Self` corromperia); os contadores
    (4 instanciações, 2 hashes de build) nunca se movem em nenhuma
    formulação de chamada (direta ou via fn-pointer, testada e
    revertida). Código morto só no contador, não no binário.
  - L665-669 (legacy): braço Err de `DdsSequence::from_vec` —
    **inalcançável, prova em duas partes**. Teoria: o `Value` do
    serde_json (~35 MB p/ 1M ints) é liberado antes do `from_proto`;
    no `dds_alloc`, o pico é só texto+EchoMsg+clone (~16 MB); se o
    parse passou (≥43 MB livres), o `dds_alloc` (4 MB) passa —
    contrapositiva fecha. Empiria: sonda com `RLIMIT_AS`
    auto-calibrado (bisseção do floor + scan, `MALLOC_ARENA_MAX`,
    conexão única) publicou de 290 MB até 229 MB sem UMA falha de
    qualquer tipo; sem janela, sem HIT. (Sonda scratch removida.)
  - L832-837 (binário): mesmo braço — inalcançável por construção:
    `decode_echo_xcdr1` limita `values` a 262144 antes de alocar, logo
    `from_vec` (≤1 MiB) nunca falha por OOM.
  - L946: `break` em `DataWriter::write` Err — sonda real: writes
    nativos de 8/32/128 MiB retornam `Ok`; falha só em exaustão de
    recursos ou entidade deletada (dono local, nunca deletada).
  - L948/L980: chaves `}` de guardas sempre-verdadeiras (`writers.get`
    cobre exatamente os tópicos de `served`; `reader.take` em reader
    local vivo) — defesa em profundidade contra refactors futuros.
  - L975: `frame.encode` Err — prova por limite: re-codificado =
    16 + tópico (≤1024, cap novo) + 16 + cdr (≤ ~2 MiB+16, caps do
    encoder) < 8 MiB = MAX sempre.
- Nota de metrologia honesta: a tabela conta 6 linhas "missed" em
  `cyclonedds-proto`/`cyclonedds-wasm` **sem atribuição file:line em
  nenhuma lente** — `--show-missing-lines` não as lista, análise por
  segmento no JSON canônico mostra toda linha atribuível com ≥1 região
  executada, e runs isolados desses crates mostram zero uncovered.
  São contadores de expansões de macro/monomorfizações não
  instanciadas sem mapeamento (sítios plausíveis: closures `map_err`
  de `to_json`/`encode`, serialização infalível de structs em memória,
  e instanciações genéricas não usadas no host). Mesma família do
  artefato L99. Nenhuma linha de produto atribuível ficou sem teste
  real ou sem prova de inalcançabilidade.
- Produto corrigido pelos testes (causa-raiz, sem `allow`):
  - `close()` encerra conexões vivas + limpa registry + derruba o
    listener (`Mutex<Option<TcpListener>>`); `Drop` delega a `close()`
    (campanha anterior).
  - **`bind()` rejeita tópico > `MAX_TOPIC_CHARS` (1024) com erro
    tipado antes de qualquer FFI**: nome de 8 MiB abortava o processo
    dentro de `dds_create_topic` (exit 139 / SIGSEGV observado). O cap
    também completa a prova de inalcançabilidade do `frame.encode`.
- Gates: `cargo test` 135/0 host + `wasm_node` 3/3 sob node;
  `RUSTFLAGS="-D warnings" cargo clippy` zero (só avisos do
  build-script do sys, pré-existentes); `cargo fmt --check` limpo;
  `wasm.yml` válido com job `node-client` + etapa `dds-wasm-e2e`, sem
  `continue-on-error`/`|| true`.

## Lacunas de evidência (declaradas, não assumidas)

- `README.md`/`Cargo.toml` do `cyclonedds-wasm` e `docs/architecture.md` nativo: citados via PR1/PR5, não relidos aqui — referenciados, não usados como prova de achado.
- Sem harness executado nesta fase (docs-only por instrução); cenários do TEST_PLAN são especificação, não resultado.

## Batalha Naval PvP (crate `dds-battleship`, 2026-09-10)

- Site Rust-only: `src/game.rs` (regras) + `src/protocol.rs` (`Msg` com
  campo `from`) + `src/client.rs` (WASM/`wasm-bindgen`), gateway nativo
  TCP↔DDS (`src/bin/gateway.rs`), proxy WS→TCP (`support/ws_dds_proxy.mjs`),
  `./run.sh` sobe tudo (site :8900, proxy :8901, DDS em loopback).
- Causa-raiz provada por execução: gateway reespelha para todos os TCPs
  (eco para o remetente); sem `from`, a página consumia o próprio `Shot` e
  travava em `Vez do oponente…`. Correção: id único por página + descarte
  de `from == me` em `Advertise`/`Join`/`Ready`/`Shot`/`Result`, com `me`
  preservado através dos resets de `create_game`/`join_game`.
- Gates: `cargo test -p dds-battleship --lib` 7/0; `cargo clippy
  --all-targets -- -D warnings` zero; `cargo fmt --check` limpo.
- PvP real (2 Chromes headless, Playwright, seletores reais): saguão →
  join → frotas prontas → turnos alternados → `Você venceu! 🎉` /
  `Você perdeu. Tente de novo!` (resultado observado: `WINNER_BRAVO`).
- README do jogo em `dds-battleship/README.md`. Não commitado (HEAD
  preservado por instrução).
