# Batalha Naval PvP — Rust + WebAssembly + Cyclone DDS

Jogo web de Batalha Naval para **dois navegadores**, 100% Rust no cliente
(compilado para WebAssembly via `wasm-bindgen`), com o estado do jogo
trafegando por **Cyclone DDS** (tópicos `battleship/*`). Sem HTTP no caminho
de dados: o browser fala WebSocket com um proxy fino, o proxy fala TCP com
o gateway nativo, e o gateway publica/assina DDS de verdade.

## Arquitetura

```
[Navegador A] --WS--> [ws_dds_proxy.mjs] --TCP--> [battleship-gateway] <--> DDS
[Navegador B] --WS-->        (8901)        --TCP-->   (porta efêmera)     (domínio local)
      |                                                              tópicos battleship/
      +--- site estático servido em http://127.0.0.1:8900           lobby, shots, results
```

| Arquivo | Papel |
|---|---|
| `src/game.rs` | Regras puras: `Board`, navios, tiros, `FireResult` (testável sem browser) |
| `src/protocol.rs` | `Msg` (`Advertise`/`Join`/`Ready`/`Shot`/`Result`) + serialização CDR-like; todo `Msg` carrega `from` (id da página) |
| `src/client.rs` | Cliente WASM: DOM, frota, turnos; **descarta o próprio eco** do gateway comparando `from` com o id da página |
| `src/bin/gateway.rs` | Binário nativo: ponte TCP↔DDS, publica nos tópicos e reespelha para todos os TCPs (com eco — por isso o `from`) |
| `support/ws_dds_proxy.mjs` | Proxy WS→TCP (Node, sem dependências) |
| `site/` | `index.html` + `pkg/` (gerado por `wasm-bindgen --target web`) |

## Como rodar

```bash
./run.sh
# Abra DOIS navegadores em http://127.0.0.1:8900
# Um cria o jogo, o outro entra pelo saguão. CTRL-C derruba tudo.
```

Um cria → posiciona a frota (`Aleatório`) → o outro clica `Entrar` no saguão →
posiciona → o criador atira primeiro → alterna no erro → quem afundar tudo
vence (`Você venceu! 🎉` / `Você perdeu. Tente de novo!`).

## Por que o campo `from`

O gateway reespelha cada mensagem DDS para **todos** os clientes TCP,
incluindo o remetente. Sem identidade de remetente, cada página processava
o próprio tiro/resultado (ex.: atirava e recebia o próprio `Shot` de volta,
travando em `Vez do oponente…`). Cada página gera um id único no `start()`
e todo handler de `Msg` descarta mensagens com `from == me`.

## Verificação (evidência por execução)

- `cargo test -p dds-battleship --lib` — 7 testes (regras + protocolo).
- `cargo clippy -p dds-battleship --all-targets -- -D warnings` — limpo.
- `cargo fmt -p dds-battleship --check` — limpo.
- PvP real com dois Chromes headless (Playwright): saguão → join → prontos →
  turnos alternados → banner de vitória nos dois lados (`WINNER_BRAVO`
  observado; o perdedor mostra `Você perdeu. Tente de novo!`).
