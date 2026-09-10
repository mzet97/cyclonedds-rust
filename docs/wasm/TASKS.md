# Tarefas (implementação futura — NADA feito nesta fase)

Ordem sugerida; cada item fecha um requisito e um cenário do TEST_PLAN.

- [ ] T1 `ConnectionState` + `await open` + `NotConnected` real (REQ-CONN-01/02; cenários 1–2)
- [ ] T2 Dispatcher único `onmessage → mapa topic→callbacks` (REQ-MUX-01; cenários 4–5)
- [ ] T3 Guardar `Closure`s; cleanup em `disconnect`/drop (REQ-CONN-03; cenários 3, 11)
- [ ] T4 Controle `hello/register/subscribe/unsubscribe/bye` + `Unsupported` (REQ-PROTO-01; cenários 6–7)
- [ ] T5 Enforcement best-effort/volatile + contadores (REQ-QOS-01, REQ-OBS-01)
- [ ] T6 Codec CDR + frame binário + corpus never-panic (REQ-PROTO-02; cenários 9–10) — ver ADR-001
- [ ] T7 Harness P1/Node + cenários 1–11 no CI (bloqueado hoje só por falta de harness, não de toolchain)
- [ ] T8 Instalar toolchain wasm32/wasm-pack + wasmtime; cenários 12–13
- [ ] T9 Bridge de referência nativa fina (ADR-002) + limites T1–T4/T8 do THREAT_MODEL
- [ ] T10 Reliable/durability (REQ-QOS-02) — backlog explícito, não MVP

Não entraram: reescrever C para wasm, DDS-Security no browser, multicast no engine (ver ADR-003).
