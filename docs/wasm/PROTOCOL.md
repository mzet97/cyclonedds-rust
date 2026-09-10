# Protocolo bridge (browser-gateway)

Versão deste documento: `proto v0` (proposta — NADA implementado; envelope atual `{"topic","data"}` em `lib.rs:191-194` não tem versão).

## Controle (JSON, versionado)

Campos base: `{"proto": 0, "kind": ..., "seq": n, ...}`. `kind ∈ {hello, register, subscribe, unsubscribe, bye, ack, error}`.

- `hello {client, proto}` → `ack {proto}` ou `error {code: unsupported_proto}`.
- `register {topic, type, qos: {reliability: best_effort, durability: volatile}}` → `ack` ou `error {unsupported_qos}`.
- `subscribe {topic}` / `unsubscribe {topic}` → `ack`.
- `bye` → fecha limpo; `error {code, detail}` nunca derruba a conexão por si só.

Regra: QoS ≠ best-effort/volatile → `error{unsupported_qos}` (tratamento, não implementação).

## Plano de dados

### Legado (atual): JSON
`{"topic": "<nome>", "data": {...}}`. Sem tipo, sem sequência, sem versão. Mantido atrás de flag `legacy_json` após migração.

### Alvo: CDR binário
Frame WS binário: `u16 proto | u16 flags | u32 topic_len | topic bytes | u32 cdr_len | CDR(XCDR2 LE) | u32 seq`.
- `flags`: bit0 = CDR-LE, bit1 = legacy-json (nunca ambos binários), resto reservado.
- `seq` por (writer, topic); permite detectar perda mesmo em best-effort (REQ-QOS-02 futuro usa para reliable).
- CDR truncado/adversarial → `Serialization`, never-panic (espelhar corpus `cdr_deserialize_corpus`).

## Compatibilidade

Bridge anuncia `proto` no `ack.hello`; cliente com `proto` maior faz downgrade ou aborta com `Unsupported`. Mudanças de `proto` seguem MIGRATION.md.
