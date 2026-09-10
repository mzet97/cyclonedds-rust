# Arquitetura WASM

Complementa `docs/architecture.md` (nativo) sem duplicá-lo: aqui só o que é específico dos perfis wasm. Nenhum código implementado nesta fase.

## Contexto

Nativo: app → `cyclonedds` (Rust, `std`+`tokio` por default) → `cyclonedds-rust-sys` → C CycloneDDS 11.0.1 bundled (`select()`, pthreads, `clock_gettime`). C não compila para wasm32 — por isso os perfis wasm não reutilizam o C.

## Perfil browser-gateway (atual)

```
browser (wasm32-unknown-unknown)
  cyclonedds-wasm (lib.rs:69-217, Rc<Participant>, JSON)
    ↕ WebSocket (ws://bridge/dds)
bridge (nativo, fora do repo)
    ↕ DDS/RTPS
participantes nativos
```

- Estado atual: sem state machine, sem fan-out, sem controle, sem CDR (detalhes em AUDIT.md A1–A6).
- Alvo: adicionar `ConnectionState`, dispatcher único `onmessage → mapa topic→callbacks`, controle versionado, CDR opcional (ver PROTOCOL.md e ADRs).

## Perfil wasi-host (futuro)

```
wasmtime/wasmer (wasm32-wasip2)
  app Rust + TCP via WASI
    ↕ RTPS/UDP-TCP nativo do host
participantes nativos
```

Sem código. Bloqueio de ambiente: target `wasm32-wasip2` não instalado; runtimes wasmtime/wasmer ausentes.

## Perfil wasm-engine (futuro, ver ADR-003)

Motor RTPS/CDR puro em wasm32 sem bridge nem C. Sem código; começa por codec CDR + loopback, depois descoberta unicast.

## Decisões

- ADR-001: codec CDR em wasm.
- ADR-002: virtualização do gateway (bridge como adaptador fino, sem semântica DDS própria).
- ADR-003: runtime engine inicial (escopo mínimo: loopback + unicast, sem multicast).
