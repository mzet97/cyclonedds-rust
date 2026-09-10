// Carrega o pacote WASM gerado por `run.sh` (wasm-bindgen --target web)
// e entrega o jogo ao `start` do Rust. Sem lógica de jogo aqui: só
// bootstrap + superfície para erro fatal.
import init, { start } from './pkg/dds_battleship.js';

const status = document.getElementById('status');

try {
  await init();
  await start();
} catch (err) {
  status.textContent = `Falha fatal ao iniciar: ${err}`;
  throw err;
}
