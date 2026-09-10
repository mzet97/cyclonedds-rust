# Changelog — cyclonedds-wasm

Formato Keep a Changelog. Datas em UTC.

## [0.2.0] - 2026-09-10

Primeira release versionada da lib DDS-para-WASM (era 0.1.0 sem release).

### Added

- Ponte `WasmClient` (browser ↔ gateway via WebSocket): `connect` com
  timeout, recepção de amostras `{ topic, text }` (`on_echo`).
- Suporte a inteiros grandes (`src/bigint.rs`) e superfície JS
  (`src/js.rs`, `js/index.mjs` + tipos).
- Harness de campanha DDS-WASM: `cyclonedds-proto`, `dds-wasm-bridge`,
  `dds-wasm-e2e`/`consumer`/`guest`, workflow `wasm.yml`, docs em `docs/wasm/`.
- Consumidor de referência da API: jogo Batalha Naval PvP
  (`dds-battleship`, cliente 100% Rust/WASM).

### Fixed

- Causa-raiz do eco do gateway documentada na campanha: broadcast com eco
  exige identidade de remetente no protocolo (consumidor descarta `from == me`).
