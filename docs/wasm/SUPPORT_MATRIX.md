# Matriz de suporte

Contadores por perfil são INDEPENDENTES (um teste conta para um perfil). P1/Node avaliado separadamente onde indicado.
Checkout: `main` / `f0f766b` / limpo. Snapshot `134c92c` do prompt diverge — ignorado.

Legenda: ✅ suportado · ⚠️ parcial · ❌ ausente (lacuna) · — não aplicável.

## Perfis

- **Native**: referência (C CycloneDDS 11.0.1 bundled + Rust). Não é alvo do plano wasm.
- **Browser-gateway**: `cyclonedds-wasm` atual — JSON sobre WebSocket via bridge. Único perfil com código hoje.
- **WASI-host**: wasm32-wasip2, sockets TCP/sandbox do host. Sem código hoje.
- **Wasm-engine**: motor DDS/RTPS puro em wasm32, sem bridge. Sem código hoje.
- **P1/Node**: avaliação do perfil browser-gateway sob Node 24 + ws fake (sem browser real nesta fase).

## Matriz

| Capacidade | Native (ref) | Browser-gateway | WASI-host | Wasm-engine | P1/Node |
|---|---|---|---|---|---|
| Conectar + state machine observável | ✅ | ❌ (A1) | ❌ | ❌ | ❌ |
| Aguardar abertura (await open) | ✅ (síncrono) | ❌ (A4) | ❌ | — | ❌ |
| Cleanup de handlers (sem leak) | ✅ | ❌ (A3) | ❌ | ❌ | ❌ |
| N readers por socket (fan-out) | ✅ | ❌ (A2) | ❌ | ❌ | ❌ |
| Controle versionado (register/subscribe) | ✅ (descoberta RTPS) | ❌ (A5) | ❌ | ❌ | ❌ |
| Plano de dados CDR binário | ✅ | ❌ (JSON só) | ❌ | ❌ | ❌ |
| Best-effort / volatile | ✅ | ⚠️ (sem enforcement) | ❌ | ❌ | ⚠️ |
| Reliable / durability / keys / loans | ✅ | ❌ (lacuna declarada) | ❌ | ❌ | ❌ |
| Erros tipados (`NotConnected` usado) | ✅ | ⚠️ (variante morta) | ❌ | ❌ | ⚠️ |
| Fuzz CDR never-panic | ✅ (corpus 5 testes) | ❌ | ❌ | ❌ | ❌ |

## Contadores independentes (estado atual, por perfil)

- Native: 8 ✅ / 1 ⚠️(QoS observável conta como ok aqui) / resto n/a.
- Browser-gateway: 0 ✅ / 2 ⚠️ (REQ-QOS-01, REQ-OBS-01) / 8 ❌.
- WASI-host: 0 ✅ / 0 ⚠️ / 10 ❌ (nenhum código; toolchain `wasm32-wasip2` nem instalada).
- Wasm-engine: 0 ✅ / 0 ⚠️ / 10 ❌ (nenhum código; decisão em ADR-003).
- P1/Node: espelha browser-gateway (0 ✅ / 2 ⚠️ / 8 ❌); Node v24.15.0 presente, mas sem harness escrito nesta fase.

## Leitura correta

Teste negativo de `Unsupported` (ex.: pedir `reliable`) conta como tratamento ✅ no requisito de *tratamento*, mas NÃO move a capacidade para ✅ — lacuna não implementada continua lacuna.
