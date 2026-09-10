/**
 * Re-export surface for `@tese/cyclonedds-wasm` (Phase H).
 *
 * After `npm run build` (web target), `./pkg/` holds the wasm-pack
 * output (`cyclonedds_wasm.js` + `.wasm`); for node, use
 * `./pkg-node/` (see `example.mjs`). This module re-exports the
 * typed surface so consumers import one path:
 *
 *   import init, { WasmClient } from '@tese/cyclonedds-wasm';
 *   await init();
 *   const client = await WasmClient.connect('ws://127.0.0.1:8080/dds', 5000);
 */
export * from './pkg/cyclonedds_wasm.js';
import init from './pkg/cyclonedds_wasm.js';
export default init;
