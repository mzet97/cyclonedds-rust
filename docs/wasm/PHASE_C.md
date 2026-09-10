# Fase C — primeira fatia ponta a ponta (working tree, sem commit)

Branch/SHA na execução: `main` / `f0f766b` (prompt citava `134c92c`;
revalidado contra o checkout real — vale `f0f766b`). Nada commitado, nada
pushed (regra dura). Parte de prior result 1 (Fase B) + `docs/wasm`.

## O que a fatia cobre

Um tipo IDL com chave + string + sequência, nos dois sentidos, com CDR
binário no data plane:

```idl
struct WasmEcho { @key long id; string text; sequence<long> values; };
```

```
"browser" (BridgeClient, mesmos bytes do wasm32) --TCP/DataFrame(CDR)-->
gateway nativo (dds-wasm-bridge) --DDS--> peer nativo (e vice-versa)
```

- Codec CDR puro em `cyclonedds-proto/src/echo.rs` (`EchoMsg`,
  `encode/decode_echo_xcdr1`, com serde só para o caminho compat JSON).
  Fonte única de verdade dos bytes do data plane.
- `cyclonedds-wasm`: `encode_echo_frame`/`decode_echo_frame` puros +
  `WasmEchoWriter` (wasm32 envia `ArrayBuffer` binário; host valida o
  frame e retorna `NotConnected`) + `create_echo_reader` (wasm32 decodifica
  `ArrayBuffer`; host registra assinatura). `WasmDdsError::Timeout` novo.
- `dds-wasm-bridge` (novo, nativo por desenho — único com `sys` no grafo):
  gateway TCP de referência (`u32 LE len` + `DataFrame`), thread DDS única
  dona das entidades, tradução CDR<->amostra tipada, `error` tipado sem
  derrubar a conexão, JSON legado só com `legacy_json: true`, contadores
  (`frames_in/samples_in/samples_out/errors_out/seq_gaps`).
- Transporte TCP carrega os bytes `DataFrame` idênticos aos do WebSocket;
  só o tipo de socket difere (browser não roda em `cargo test`).

## Evidência de testes (esta sessão, reais, com loopback pinado)

Ambiente: o seletor default de interface do CycloneDDS mira `enp4s0`
(inexistente; viva `enp7s0`), então todo `DomainParticipant::new` falha com
`ReturnCode(-1)` — inclusive testes do próprio repo (`test_keyed…`
confirmado falhando sem a variável). Os testes da ponte fixam `lo` via
`CYCLONEDDS_URI` sob `Once` (`ensure_loopback_dds`); sem isso falham na
criação do participante (comando+erro registrados, item independente
contornado sem mascarar).

- `cargo test -p dds-wasm-bridge`: **6 passed** —
  `codec_matches_native_libddsc_byte_for_byte` (codec puro == bytes
  `CdrSerializer` XCDR1, ida e volta),
  `browser_to_dds_cdr_binary` (cliente->DDS + loopback, `samples_in>=1`,
  `errors_out==0`),
  `dds_to_browser_cdr_binary` (DDS->cliente, `samples_out>=1`),
  `legacy_json_rejected_by_default_then_binary_still_flows`
  (`legacy_json_disabled`, rejeitado não chega ao peer, binário segue),
  `legacy_json_accepted_in_explicit_compat_and_forwarded_as_cdr`
  (JSON aceito chega ao peer; volta como frame CDR),
  `truncated_frame_and_unknown_topic_are_typed_errors_then_idle_times_out`
  (`invalid_frame`, `unknown_topic`, `errors_out>=2`,
  `BridgeError::Timeout` em leitura ociosa de 300 ms).
- `cargo test -p cyclonedds-proto`: **14 passed** (8 Fase B + 6
  `echo_cdr`: roundtrip com bytes exatos, padding desalinhado, truncados
  -> `InvalidFrame`, comprimentos adversariais -> `TooLarge`/sem pânico).
- `cargo test -p cyclonedds-wasm`: **6 passed** (3 Fase B + roundtrip do
  frame CDR com `flags==CDR_LE`, rejeição de flag legada/garbage,
  `write_echo` host -> `NotConnected`).
- `cargo check -p cyclonedds-wasm --target wasm32-unknown-unknown`: OK
  (writer/reader binários compilam no browser).
- `cargo check --workspace`: OK, zero warnings nos crates tocados.
- `cargo test -p cyclonedds --lib`: 25 passed / 3 failed — **idênticas às
  3 da Fase B no HEAD pristino** (interface do sandbox), sem regressão.
- `cargo tree -p dds-wasm-bridge` contém `cyclonedds-rust-sys` (nativo por
  desenho); `cargo tree -p dds-wasm-consumer` continua sem `sys`.

## Sem happy-path-only

Caminho negativo observável em cada camada: truncado/adversarial nunca
entra em pânico (erros tipados), `error{invalid_frame, unknown_topic,
legacy_json_disabled, unsupported_flags, frame_too_large,
unsupported_proto}` sem derrubar a conexão, `Timeout` tipado em leitura
ociosa, `Unsupported` para QoS != best-effort/volátil (Fase B, mantido).

## Lacunas que continuam lacunas

Handshake `hello`/`ack` e `register/subscribe` versionados, roteamento
multi-tópico (um tópico por gateway), reliable/durability (segue
`Unsupported`), framing WebSocket no lado gateway, dispatcher único no
browser (um `onmessage` por reader), `seq_gaps` só contado (sem
retransmissão — best-effort). Nada disso foi discretizado: o que não foi
implementado continua marcado como lacuna.
