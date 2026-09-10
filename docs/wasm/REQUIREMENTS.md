# Requisitos WASM

Convenções: cada requisito tem ID, perfil, semântica esperada, teste positivo, teste negativo, evidência, estado.
Estados: `ausente` (lacuna), `parcial`, `ok`. Sem discretização: teste negativo de `Unsupported` prova tratamento, não implementação.

Perfis: `browser-gateway` (WS/JSON via bridge), `wasi-host` (WASI + TCP nativo futuro), `wasm-engine` (motor DDS puro em wasm32, sem bridge).
Nativo (`native`) aparece só como referência — não é alvo deste plano.

## Transporte / conexão

### REQ-CONN-01 — State machine de conexão observável
- Perfil: browser-gateway.
- Semântica: participante expõe `Connecting/Open/Closed/Error`; `write` antes de `Open` retorna `NotConnected` (ou enfileira com limite documentado); `disconnect` transita para `Closed`.
- Teste positivo: conectar contra bridge fake, aguardar `Open`, escrever → entregue.
- Teste negativo: escrever com socket ainda CONNECTING → `NotConnected`, não `WebSocket` genérico nem panic.
- Evidência: `cyclonedds-wasm/src/lib.rs:70-73,77-103,168-170` (sem estado hoje).
- Estado: `ausente` (A1, A4).

### REQ-CONN-02 — Prontidão assíncrona (await open)
- Perfil: browser-gateway.
- Semântica: API para aguardar abertura (`promise`/`Future`/callback registrado antes de `Ok`); `new()` não reporta sucesso antes do `onopen`.
- Teste positivo: `connect().await` resolve após `onopen` do servidor fake.
- Teste negativo: servidor que nunca completa handshake → timeout documentado, erro tipado.
- Evidência: `lib.rs:86-92` (só `console.log`).
- Estado: `ausente` (A4).

### REQ-CONN-03 — Cleanup de handlers (sem leak obrigatório)
- Perfil: browser-gateway.
- Semântica: handles de `Closure` guardados; `disconnect`/drop faz `set_onmessage(None)`/`set_onopen(None)` e libera; múltiplos ciclos connect/disconnect sem crescimento.
- Teste positivo: 100 ciclos connect/disconnect contra fake → sem erro, handlers removidos.
- Teste negativo: `disconnect` duas vezes → idempotente, sem throw JS.
- Evidência: `lib.rs:92,100,158` (três `forget` sem guarda).
- Estado: `ausente` (A3).

## Multiplexação

### REQ-MUX-01 — N readers por socket (fan-out por tópico)
- Perfil: browser-gateway.
- Semântica: UM handler `onmessage` global despacha para N readers via mapa `topic -> callbacks`; criar reader B não cala reader A.
- Teste positivo: 2 readers em tópicos distintos recebem cada um só suas amostras.
- Teste negativo: amostra para tópico sem reader → descartada sem chamar callbacks (contador de drops).
- Evidência: `lib.rs:133-164,206` (handler por reader, `ws` morto).
- Estado: `ausente` (A2).

## Protocolo de controle

### REQ-PROTO-01 — Controle versionado (register/subscribe/unsubscribe)
- Perfil: browser-gateway.
- Semântica: `create_topic`/`create_reader`/`create_writer`/drop enviam mensagens de controle versionadas (`hello/register/subscribe/...`, ver PROTOCOL.md); bridge confirma ou erra tipado.
- Teste positivo: sequência `hello→register→subscribe` observada no socket fake.
- Teste negativo: bridge responde `error{unsupported}` → erro `Unsupported`, não crash; lacuna continua lacuna.
- Evidência: `lib.rs:106-130` (só HashMap local).
- Estado: `ausente` (A5).

### REQ-PROTO-02 — Plano de dados binário CDR
- Perfil: browser-gateway (futuro), wasi-host, wasm-engine.
- Semântica: payload CDR (XCDR2, little-endian) com cabeçalho versionado; JSON mantido só como fallback legado atrás de flag.
- Teste positivo: roundtrip CDR de struct com `string`/`sequence` via loopback fake byte-idêntico.
- Teste negativo: CDR truncado/adversarial → `Serialization`, never-panic (espelhar `cdr_deserialize_corpus`).
- Evidência: `lib.rs:9-13,191-194` (JSON sem versão); corpus nativo em `cyclonedds-test-suite/tests/cdr_deserialize_corpus.rs`.
- Estado: `ausente` (A6). Ver ADR-001.

## Entrega / QoS

### REQ-QOS-01 — Best-effort/volatile documentado e observável
- Perfil: browser-gateway.
- Semântica: única QoS suportada é best-effort + volatile; qualquer QoS incompatível é rejeitada com `Unsupported` no `create_*`.
- Teste positivo: pub/sub best-effort entrega em loopback.
- Teste negativo: pedir `reliable`/`transient-local` → `Unsupported` (prova tratamento, não implementação).
- Evidência: `lib.rs:9-13`.
- Estado: `parcial` (documentado no doc-comment, sem enforcement em código).

### REQ-QOS-02 — Confiabilidade (futuro, fora do MVP)
- Perfil: browser-gateway/wasm-engine.
- Semântica: ACK/sequence numbers, redelivery — NÃO implementado; qualquer teste positivo hoje deve FALHAR e permanecer como lacuna registrada.
- Teste positivo (esperado falhar): perda injetada → recuperada.
- Teste negativo: pedir reliable → `Unsupported`.
- Estado: `ausente` (lacuna declarada, sem discretização).

## Tipos / serialização

### REQ-TYPE-01 — Strings/sequences com stride Native (wasi/engine)
- Perfil: wasi-host, wasm-engine.
- Semântica: elementos `sequence<Struct>` usam stride `Native`, `to_native_value` separado; `all-zero` válido; amostra indecodificável vira discard, não panic (contrato `topic.rs:215-287,311-327`).
- Teste positivo: roundtrip de `sequence<Struct>` com stride correto.
- Teste negativo: discriminante remoto desconhecido → discard contabilizado.
- Evidência: `cyclonedds/src/topic.rs:215-327`.
- Estado: `ausente` nos perfis wasm (ok no nativo).

## Observabilidade / segurança

### REQ-OBS-01 — Erros tipados e contadores
- Perfil: todos os perfis wasm.
- Semântica: `NotConnected` usado de verdade; contadores `drops_unknown_topic`, `decode_errors`, `unsupported_requests`.
- Teste positivo: dashboard/counter reflete drops de fuzz determinístico.
- Teste negativo: `TopicNotFound` ao escrever em tópico desconhecido (quando enforcement ligado).
- Evidência: `lib.rs:45-67` (enum existe, `NotConnected` morto).
- Estado: `parcial`.

### REQ-SEC-01 — Limites do bridge e ameaças documentadas
- Perfil: browser-gateway.
- Semântica: bridge não confia no cliente (valida tópico/tamanho/tipo); ver THREAT_MODEL.md.
- Teste positivo: cliente malformado é rejeitado sem derrubar o bridge.
- Teste negativo: envelope gigante → rejeição `too_large`, conexão preservada.
- Estado: `ausente` (só docs nesta fase).
