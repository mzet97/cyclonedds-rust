# Auditoria WASM — cyclonedds-rust (síntese FASE A)

Local-only. Nenhum push/publish/release/delete-branch/mudança destrutiva. Nenhum código implementado nesta fase; só docs.

## Registro de checkout

- Branch: `main`
- SHA (HEAD real): `f0f766b724b991b67d5a95f918966c1d1ed6c1c7` (curto `f0f766b`)
- Dirty state: LIMPO (`git status --short` sem saída, verificado antes e depois da escrita)
- DIVERGÊNCIA: o prompt citava snapshot `134c92c`; o checkout real é `f0f766b`. Todas as afirmações abaixo foram revalidadas contra o código real em `f0f766b`. Nada ancorado em `134c92c` foi assumido.
- Crate auditada: `cyclonedds-wasm/src/lib.rs` (217 linhas), `cyclonedds-wasm/Cargo.toml`, `cyclonedds-wasm/README.md`
- Workspace: 12 membros, versão 3.0.0, edition 2021, rust 1.85 (`Cargo.toml:2-25`)

## Achados (todos confirmados contra código real)

### A1. Ausência de state machine de conexão — CONFIRMADO (lacuna, não bug menor)
- `WasmDomainParticipant` só tem `ws + topics` (`lib.rs:70-73`); nenhum enum/flag de estado, zero ocorrências de `ready_state`.
- `WasmDdsError::NotConnected` existe (`lib.rs:50`, Display `lib.rs:59`) mas nunca é construído/retornado.
- `disconnect()` (`lib.rs:168-170`) só chama `ws.close()`, sem marcar estado.
- Consequência: impossível distinguir CONNECTING/OPEN/CLOSED; chamador não tem como aguardar prontidão.

### A2. Bug onmessage-por-reader (último reader vence) — CONFIRMADO
- `create_reader` chama `ws.set_onmessage(...)` (`lib.rs:157`), que substitui o handler global do socket; closure captura um único `topic_name` (`lib.rs:138`) e filtra por ele (`lib.rs:146`).
- Segundo `create_reader` sobrescreve o primeiro — leitores anteriores calam.
- `WasmDataReader.ws` guardado mas morto (`#[allow(dead_code)]`, `lib.rs:206`).

### A3. `Closure::forget` sem guarda/chave de remoção — CONFIRMADO
- Três `forget()`: onopen (`lib.rs:92`), onerror (`lib.rs:100`), onmessage (`lib.rs:158`). Nenhum handle guardado.
- `disconnect()`/drop nunca consegue `set_onmessage(None)` nem liberar — leak intencional do wasm-bindgen sem caminho de cleanup.

### A4. Criação sem aguardar abertura — CONFIRMADO
- `new()` (`lib.rs:77-103`) cria `WebSocket::new`, registra onopen que só dá `console.log` (`lib.rs:86-88`) e retorna `Ok` imediatamente; sem promise/await/callback de pronto.
- `write()` (`lib.rs:188-201`) chama `send_with_str` direto (`lib.rs:197-199`): se o socket ainda está CONNECTING, o send falha como `WasmDdsError::WebSocket`.

### A5. Falta de protocolo de registro/subscribe — CONFIRMADO
- `create_topic` só insere no HashMap local (`lib.rs:111-113`); nada é enviado ao bridge.
- `create_reader` não envia subscribe; `create_writer` não envia register. O bridge nunca é informado — roteamento depende de convenção externa não especificada.
- Envelope JSON atual: `{"topic": ..., "data": ...}` (`lib.rs:191-194`); sem versão, sem tipo, sem sequência, sem qos.

### A6. Só best-effort/volatile; sem QoS, sem RTPS, sem CDR — CONFIRMADO (lacuna declarada)
- Doc do crate declara: não é DDS completo; JSON sobre WebSocket, não RTPS (`lib.rs:9-13`).
- Sem serialização CDR, sem descoberta, sem confiabilidade, sem durabilidade, sem chaves/instâncias, sem loans (loans nativos vivem em `cyclonedds/src/sample.rs, reader.rs, writer.rs` — inaplicáveis ao browser).
- Teste negativo de `Unsupported` prova tratamento, não implementação: lacuna não implementada continua lacuna.

## Superfície FFI e runtime nativo (contexto do que NÃO atravessa o WASM)

- `DdsType::Native` unsafe (`cyclonedds/src/topic.rs:221`), contrato Safety (`topic.rs:215-220`), `descriptor_size/align` de `Native` (`topic.rs:249-255`), descriptor montado em `Topic::create` (`topic.rs:455-562`), `DescriptorHolder` Send/Sync por imutabilidade (`topic.rs:119-124`).
- Loans via `dds_request_loan` (`writer.rs:339-367`); parents retidos; nada disso existe no perfil browser.
- Fonte C efetiva: `cyclonedds-src` bundled vence (`build.rs:284-313`), versão 11.0.1; `vendor/cyclonedds` é submódulo NÃO inicializado (diretório vazio) — linhas de `vendor/` não descrevem a lib linkada.
- ddsrt: sockets POSIX/Windows + `select()`, threads POSIX/Windows/FreeRTOS, clocks via `clock_gettime`, sem epoll/kqueue dedicados.

## Capacidades do ambiente (verificado por execução)

- rustc 1.95.0, toolchains stable/nightly/1.85.0; targets instalados SOMENTE `x86_64-unknown-linux-gnu` — `wasm32-unknown-unknown` e `wasm32-wasip2` NÃO instalados.
- `wasm-pack` AUSENTE, `wasm-bindgen-cli` AUSENTE; node v24.15.0 presente; wasmtime/wasmer/emcc AUSENTES; playwright 1.60.0 + chromium presente (teste headless possível); cmake e `idlc` presentes.
- Baseline nativo: `cargo test -p cyclonedds` lib 25 passed / 3 FAILED (participant listener + 2 pool, `ReturnCode(-1)`, causa C: sandbox sem interface UDP utilizável). Testes de rede falham no sandbox — esperado.

## Testes existentes relevantes

Loans (`loaned_reads`, `loan_heap_fields`, `write_loan`, `write_loan_async`), listeners (`listeners`, `listener_panic_barrier`), cancelamento (`async_cancellation`, `async_wait_cancellation`), CDR malformado (`cdr_deserialize_corpus` 5 testes + fuzz target), ABI/layout (`native_layout_recursive`, `ops_scanner_alignment`, `ops_vs_idlc`), mais `error_recovery`, `error_retcode_mapping`, `union_*`, `descriptor_clone`, `dynamic_cdr_roundtrip`.
