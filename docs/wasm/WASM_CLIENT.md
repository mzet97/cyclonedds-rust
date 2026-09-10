# Cliente WebAssembly (DDS no browser)

Como o Rust vira jogo jogável no navegador neste repo: toolchain, API
exportada, fluxo de mensagens e como reconstruir. Exemplo vivo: o crate
`dds-battleship` (ver `dds-battleship/README.md` e
`docs/wasm/RELATORIO_BATALHA_NAVAL.md`).

## Toolchain

- Alvo `wasm32-unknown-unknown` (Rust ≥ 1.85), `crate-type = ["cdylib", "rlib"]`.
- `wasm-bindgen --target web` gera `site/pkg/` (`dds_battleship.js` + `.wasm`).
- `site/pkg/` é **artefato gerado**: está no `.gitignore` e o `./run.sh`
  o reconstrói. Nunca edite à mão.
- Dependências só-browser (`cfg(target_arch = "wasm32")`): `wasm-bindgen`,
  `wasm-bindgen-futures`, `js-sys`, `web-sys` (features mínimas: `Document`,
  `Element`, `HtmlInputElement`, `Location`, `Window`, …) e
  `cyclonedds-wasm` (ponte `WasmClient`, path dep `../cyclonedds-wasm`).
- O mesmo crate compila no host (`rlib`) para testes: regras e protocolo
  são 100% testáveis sem browser (`cargo test -p dds-battleship --lib`).

## Superfície exportada

Só um símbolo cruza a fronteira WASM↔JS:

```rust
#[wasm_bindgen]
pub async fn start()  // dds-battleship/src/client.rs
```

O `site/app.js` faz apenas bootstrap — sem lógica de jogo:

```js
import init, { start } from './pkg/dds_battleship.js';
await init();
await start();
```

Todo o resto (DOM, estado, rede) vive no Rust e fala com o browser via
`web-sys`/`js-sys`.

## Como o browser chega ao DDS

O browser **não** fala DDS direto. O caminho é:

1. `start()` gera o id da página, monta os tabuleiros e abre
   `ws://<host>:8901` (porta do proxy; host sai de `location.hostname()`).
2. `WasmClient::connect(url, 8000)` (de `cyclonedds-wasm`) abre o WebSocket
   com timeout de 8 s; falha mostra `Sem conexão com a ponte` no `#conn`.
3. Cada amostra DDS chega como `{ topic, text }` no callback `on_echo`;
   `on_sample` desserializa o `Msg` e despacha para `on_lobby`/`on_shot`/`on_result`.
4. Enviar = serializar o `Msg` e publicar no tópico (`LOBBY`, `SHOTS`,
   `RESULTS`); o proxy repassa ao gateway, que publica no DDS de verdade.
5. Estado em `thread_local!` (`STATE`, `CLIENT`); callbacks do DOM
   (`Closure::wrap` + `forget`) chamam `place_at`/`fire_at`/`join_game`.

## Regra de ouro: o campo `from`

O gateway reespelha cada mensagem para **todos** os TCPs, incluindo o
remetente (eco). Por isso todo `Msg` carrega `from` (id da página) e todo
handler descarta `from == me` — sem isso a página consome o próprio tiro
e o jogo trava em `Vez do oponente…`. Quem adicionar mensagem nova ao
protocolo precisa incluir `from` e o descarte correspondente.

## Reconstruir e verificar

```bash
cd dds-battleship
cargo test -p dds-battleship --lib                        # host, sem browser
cargo clippy -p dds-battleship --all-targets -- -D warnings
cargo fmt -p dds-battleship --check
cargo build -p dds-battleship --target wasm32-unknown-unknown --lib
wasm-bindgen --target web --out-dir site/pkg \
  --out-name dds_battleship \
  ../target/wasm32-unknown-unknown/debug/dds_battleship.wasm
./run.sh   # faz tudo acima + gateway + proxy + site em :8900
```

Limites conhecidos: sem autenticação (qualquer página pode forjar `from`);
`DDS Security` não ativo; DDS nunca sai de loopback nesse setup.
