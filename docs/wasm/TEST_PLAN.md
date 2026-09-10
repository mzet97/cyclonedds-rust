# Plano de testes (13 cenários mínimos)

Harness: P1/Node (Node 24 + socket fake) e, quando DOM/WS real for preciso, playwright+chromium (presente e verificado: chromium headless real com WebSocket na página). Nenhum harness escrito nesta fase — só especificação. Atualização 2026-09-10: target `wasm32-unknown-unknown` INSTALADO (rustc 1.95.0) e `dds-wasm-consumer.wasm` linkado (módulo MVP válido); `wasm-pack`/`wasm-bindgen-cli` seguem ausentes. Cenários 12–13 seguem bloqueados (sem `wasm32-wasip2`/wasmtime e sem código do engine).

1. `conn-open`: bridge fake completa handshake → `Open` observável; `write` entrega. (REQ-CONN-01/02)
2. `conn-not-connected`: `write` com socket CONNECTING → `NotConnected`. (negativo A4)
3. `conn-double-disconnect`: `disconnect` 2× idempotente, sem throw. (REQ-CONN-03)
4. `mux-two-readers`: readers A/B em tópicos distintos recebem só o seu. (positivo A2)
5. `mux-unknown-topic`: amostra sem reader → drop contabilizado, callbacks silenciosos. (negativo A2)
6. `ctrl-handshake`: ordem `hello→register→subscribe` observada no fake. (REQ-PROTO-01)
7. `ctrl-unsupported-qos`: pedir reliable → `Unsupported`, conexão viva. (negativo; prova tratamento)
8. `data-json-roundtrip`: struct com string/sequence via loopback legado. (compat)
9. `data-cdr-roundtrip`: mesmo payload em CDR byte-idêntico. (REQ-PROTO-02; esperado FALHAR até implementar)
10. `data-cdr-corpus`: bateria truncado/adversarial → `Serialization`, never-panic. (negativo REQ-PROTO-02)
11. `leak-cycles`: 100× connect/disconnect → handlers removidos, sem crescimento. (REQ-CONN-03)
12. `wasi-loopback`: (bloqueado: sem `wasm32-wasip2`/wasmtime) pub/sub local sob wasmtime.
13. `engine-loopback`: (bloqueado: sem código) motor puro loopback sem bridge.

Critério: 1–8,10–11 passam no MVP browser-gateway; 9,12,13 devem falhar como lacunas registradas (sem discretização).
