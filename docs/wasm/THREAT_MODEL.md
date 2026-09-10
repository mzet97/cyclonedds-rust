# Modelo de ameaças (browser-gateway)

## Fronteiras

Navegador (não confiável) → WebSocket público → bridge → rede DDS (confiável). Bridge é o enforcement point; cliente wasm nunca é autoridade.

## Ameaças e mitigações (proposta, sem código)

| # | Ameaça | Impacto | Mitigação |
|---|---|---|---|
| T1 | Cliente envia tópico/tipo inexistente | Roteamento fantasma, bridge confuso | Bridge valida contra registro; `error{unknown_topic}`; REQ-OBS-01 conta drops |
| T2 | Envelope gigante / JSON profundo | OOM do bridge | Limite de tamanho + profundidade; `error{too_large}`, conexão preservada |
| T3 | CDR adversarial | Panic/UB no parse | Parser never-panic, fuzz corpus (cenário 10); `Serialization` sem crash |
| T4 | Flood de publish | DoS do bridge/rede DDS | Rate-limit por conexão + cota; `error{rate_limited}` |
| T5 | WS sem TLS (ws://) em produção | Espionagem/injeção | Exigir `wss://` fora de localhost; documentar em MIGRATION.md |
| T6 | Origem cruzada maliciosa | CSRF de tópicos | Checar `Origin` no bridge;allowlist |
| T7 | `Closure::forget` eterno (A3) | Vazamento no cliente, não no bridge | REQ-CONN-03; sem impacto de segurança no servidor |
| T8 | Bridge confia no `type` do cliente | Type-confusion na rede DDS | Bridge resolve tipo no lado nativo (IDL), nunca aceita descriptor do cliente |

Fora de escopo nesta fase: autenticação DDS-Security no browser, E2E crypto, auditoria de supply-chain do bundle wasm.
