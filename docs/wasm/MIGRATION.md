# Migração (para o perfil browser-gateway)

## De hoje (JSON sem versão) para proto v0

1. Bridge passa a aceitar ambos; responde `ack{proto}` — cliente legado (sem `proto`) recebe comportamento legado.
2. Cliente implementa `hello/register/subscribe` + state machine; flag `legacy_json` default ON.
3. Cliente implementa CDR; flag `legacy_json` default OFF; bridge rejeita JSON sem flag com `error{deprecated}`.
4. `proto v1`: remover JSON (major). Regra: bump de `proto` = changelog em PROTOCOL.md + cenário 6 verde antes do merge.

## Requisitos de ambiente (pré-condições bloqueadas hoje)

- `rustup target add wasm32-unknown-unknown wasm32-wasip2`; instalar `wasm-pack` (ou `wasm-bindgen-cli`); um runtime WASI (wasmtime) para cenário 12.
- Fora de localhost: servir bridge em `wss://` (T5).

## Rollback

Cada passo é reversível por flag (`legacy_json`, `proto` negociado). Falha em cenário 1–8 ou 10–11 bloqueia o passo seguinte.
