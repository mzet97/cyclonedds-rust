# ADR-003 — Runtime engine inicial

- Estado: proposto (sem código).
- Contexto: perfis wasi-host (WASI + TCP) e wasm-engine (RTPS puro) não têm código; toolchain `wasm32-wasip2` e runtimes ausentes (PR2).
- Decisão: escopo inicial do engine = codec CDR (ADR-001) + loopback local + descoberta unicast via bridge/wasi-host; SEM multicast, SEM reliable, SEM durability, SEM security no MVP.
- Alternativas: (a) engine full-RTPS de largada — rejeitada (superfície enorme, sem harness); (b) só wasi-host sem engine — rejeitada (fecha a porta do browser sem bridge).
- Consequências: cenários 12–13 nascem como lacunas que falham; REQ-QOS-02 fica em backlog explícito (T10).
