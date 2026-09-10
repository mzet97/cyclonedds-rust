# Fases D + E + F — dispatcher, paridade, guest WASI (working tree, sem commit)

Branch/SHA na execução: `main` / `f0f766b` (prompt citava `134c92c`;
revalidado — vale `f0f766b`). Nada commitado, nada pushed (regra dura).
Continua Fase C (`docs/wasm/PHASE_C.md`); só `dds-wasm-bridge`,
`dds-wasm-guest` (novo), `Cargo.toml` (membro novo) e testes foram tocados.

## D — dispatcher por conexão, multi-tópico, filas limitadas, backoff, cancel, drop

- Uma lane DDS por tópico (`BridgeConfig::extra_topics`; default preserva
  Fase C de 1 tópico). `DdsCmd::Publish { topic, sample }` roteia para o
  writer da lane; takes por lane com `seq` de saída compartilhado.
- Dispatcher por conexão: thread leitora (frames -> DDS + `subscribe`/
  `unsubscribe`/`register`/`hello`/`bye` com `ack` tipado) + thread pump
  (fila limitada -> socket). `unsubscribe` filtra downstream só naquela
  conexão; default assina todos os tópicos servidos.
- Fila limitada por conexão (`queue_cap`, default 64): `try_send`; cheia
  derruba o frame e incrementa `dropped_overflow` — nunca bloqueia a
  thread dona do DDS. `Disconnected` poda o slot do registry.
- `backoff_delay` pura (schedule `base*2^n`, saturada, com cap);
  `BridgeClient::connect_with_retry` com orçamento que falha alto (`Io`).
- `CancelFlag` + `recv_echo_cancel`: `Cancelled` tipado no próximo quantum
  de 200 ms; sem cancel, `Timeout` no deadline.
- `close()` idempotente + `Drop`: remover o slot derruba o sender da fila
  e a outra ponta sai — sem thread/socket órfão (roundtrip antes e depois
  de drop+rebind no mesmo processo).
- QoS incompatível no `register` falha alto: `unsupported_qos` tipado,
  conexão sobrevive.

Provas: `phase_d` **11/11** (hello/ack, 2 tópicos ida+volta, filtro
unsubscribe+resubscribe, register ok vs reliable/transient-local ->
`unsupported_qos`, schedule/backoff puro + budget esgotado + late join de
listener, cancel acorda / quiet dá timeout, stuck consumer com
`queue_cap: 1` + 300×32 KiB => `dropped_overflow>=1` com `samples_out`
progredindo, close/drop+rebind).

## E — matriz de paridade (nativo real, loopback)

`parity` **9/9**: read não consome / take consome; máscaras NOT_READ ->
READ; dispose entrega amostra com `instance_state == DISPOSED` (observado:
`valid_data` continua true — check corrigido contra o real, não o
assumido); unregister com `autodispose=false` entrega `NO_WRITERS`
(observado: default auto-dispose termina em DISPOSED); chaves isolam
instâncias (`lookup/read/take_instance`, `instance_get_key`); `take_next`
com `valid_data` + `NOT_READ` + `ALIVE`; `check_qos` rejeita
reliable/transient-local com `Unsupported` e JSON com variante
desconhecida falha no parse (fail closed); writer best-effort × reader
reliable => `requested/offered_incompatible_qos.total_count>=1` e nada
entrega; writer transient-local+keep_all publicado **antes** do reader =>
late joiner recebe os 3 via `wait_for_historical_data`.

## F — guest WASI P2 + host de referência, WIT versionada

- `dds-wasm-guest/wit/dds-bridge.wit`: `package tese:dds-bridge@0.1.0`
  (`types` com `echo-sample`/`qos`/`data-frame`/`guest-error`, `guest`,
  `host`, worlds `wasm-guest`/`native-host`).
- `dds-wasm-guest` (portável: só `cyclonedds-proto`; `cargo tree` confirma
  zero `cyclonedds-rust-sys`; compila em `wasm32-unknown-unknown`):
  `GuestDispatcher` (encode+seq, filtro por inscrição, `handle-control`,
  outbox limitada com `drops`, gaps) + `WitEchoSample` projeção 1:1 do
  record WIT. Testes próprios **5/5**.
- Host de referência `wit_pubsub` **3/3**: guest publica -> peer nativo
  recebe; peer nativo publica -> guest decodifica; bytes adversariais
  (lixo, tópico errado) viram `invalid_frame`/`unknown_topic` e nada chega
  ao DDS (host revalida tudo).

## Sem discretização / lacunas que continuam lacunas

`wasmtime` ausente no ambiente (`command -v wasmtime` => vazio):
sandbox de componente real, target `wasm32-wasip2` e framing WebSocket no
gateway continuam lacunas — guest roda in-process e o isolamento repousa
na fronteira de tipos + revalidação do host. Reliable/durável seguem
`Unsupported` (tratamento, não implementação); `seq_gaps` só conta.
`cyclonedds --lib`: 25/3 idêntico ao baseline (falhas de sandbox em
`participant`/`participant_pool`, arquivos nunca tocados nesta sessão).
`cargo check --workspace` OK, zero warnings nos crates tocados;
`cyclonedds-proto` 14/14, `cyclonedds-wasm` 6/6 preservados.