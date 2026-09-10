# Changelog — dds-battleship

Formato Keep a Changelog. Datas em UTC.

## [0.1.0] - 2026-09-10

Primeira versão jogável: Batalha Naval PvP para dois navegadores sobre Cyclone DDS.

### Added

- Regras puras (`src/game.rs`): tabuleiro, frota, tiros, `FireResult`.
- Protocolo DDS (`src/protocol.rs`): `Advertise`/`Join`/`Ready`/`Shot`/`Result`
  com campo `from` (identidade do remetente).
- Cliente browser em WASM (`src/client.rs`): DOM, posicionamento de frota,
  turnos alternados, banners de vitória/derrota.
- Gateway nativo TCP↔DDS (`src/bin/gateway.rs`) e proxy WS→TCP
  (`support/ws_dds_proxy.mjs`).
- `run.sh`: build WASM + gateway + proxy + site com um comando.
- 7 testes de lib (regras + protocolo), README do crate.

### Fixed

- Eco do gateway: a página consumia o próprio tiro e travava em
  `Vez do oponente…`. Correção: id único por página + descarte de
  `from == me` em todos os handlers, com `me` preservado nos resets de
  criar/entrar.
