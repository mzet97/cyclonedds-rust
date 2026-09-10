# Relatório — Batalha Naval PvP (Rust + WebAssembly + Cyclone DDS)

Data: 2026-09-10 · HEAD `f0f766b` preservado (nada commitado, nada com push).

## 1. O que foi pedido

Site web com jogo de Batalha Naval PvP para dois navegadores, projeto em
Rust, usando a lib com WebAssembly + Cyclone DDS — e tudo testado em
ambiente real, não só em teoria.

## 2. O que foi construído (crate `dds-battleship`)

```
third_party/cyclonedds-rust/dds-battleship/
├── src/game.rs        # regras puras: tabuleiro, navios, tiros, FireResult
├── src/protocol.rs    # Msg (Advertise/Join/Ready/Shot/Result) + ser. CDR-like
├── src/client.rs      # cliente WASM: DOM, frota, turnos, descarte de eco
├── src/bin/gateway.rs # binário nativo: ponte TCP ↔ DDS (tópicos battleship/*)
├── site/              # index.html + pkg/ (gerado por wasm-bindgen --target web)
├── support/ws_dds_proxy.mjs  # proxy WS→TCP (Node, sem dependências)
├── run.sh             # sobe tudo: build WASM + gateway + proxy + site
└── README.md          # como rodar e arquitetura
```

Fluxo de dados (sem HTTP no caminho de dados):

```
[Navegador A] --WS--> [proxy :8901] --TCP--> [gateway] <--> DDS (loopback)
[Navegador B] --WS-->                  --TCP-->              tópicos battleship/
Site estático: http://127.0.0.1:8900
```

## 3. O bug que impedia o jogo (causa-raiz, provada por execução)

Sintoma: após o `Ready`, o jogo travava em `Vez do oponente…` e nunca
avançava. Diagnóstico executado: o gateway reespelha cada mensagem DDS
para **todos** os clientes TCP, **incluindo o remetente**. Sem identidade
de remetente, cada página processava o próprio tiro/resultado.

Correção aplicada:

- Campo `from` (id único da página, gerado no `start()`) em todas as
  variantes de `Msg`: `Advertise`, `Join`, `Ready`, `Shot`, `Result`.
- Todo handler descarta mensagens com `from == me`.
- O id `me` é preservado através dos resets de `create_game`/`join_game`
  (que recriam o `State` — sem isso o id era zerado e o descarte falhava).

## 4. Evidência por execução (nada foi afirmado sem rodar)

| Verificação | Comando | Resultado observado |
|---|---|---|
| Testes de regras + protocolo | `cargo test -p dds-battleship --lib` | 7 passed, 0 failed |
| Lint | `cargo clippy -p dds-battleship --all-targets -- -D warnings` | zero warnings |
| Formato | `cargo fmt -p dds-battleship --check` | limpo |
| Build WASM | `cargo build -p dds-battleship --target wasm32-unknown-unknown --lib` + `wasm-bindgen --target web` | `site/pkg/` gerado |
| PvP real, 2 Chromes headless (Playwright, seletores reais do DOM) | lobby → join → `random-btn` nos dois → tiros alternados até o fim | `Você venceu! 🎉` / `Você perdeu. Tente de novo!` — `WINNER_BRAVO` |

## 5. Como reproduzir

```bash
cd third_party/cyclonedds-rust/dds-battleship
./run.sh
# Abra DOIS navegadores em http://127.0.0.1:8900
# Um cria o jogo (+ "Aleatório"), o outro clica "Entrar" (+ "Aleatório").
# O criador atira primeiro; errou, passa a vez; afundou tudo, venceu.
```

## 6. Riscos e lacunas declaradas

- O gateway faz broadcast com eco por desenho; o descarte é no cliente
  (`from == me`). Se um terceiro cliente forjar `from`, pode poluir o
  jogo — sem autenticação (fora do escopo; DDS Security não ativo).
- `site/pkg/` é artefato gerado (rebuildado pelo `run.sh`); `dds-battleship/`
  e `docs/wasm/` estão untracked — commit/push ficaram de fora por instrução.
- Cobertura anterior de 98,14% da campanha DDS-WASM e gaps residuais
  (MAX_TOPIC_CHARS, L99 etc.) pertencem à campanha da lib, não a este jogo.
