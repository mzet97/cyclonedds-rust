/**
 * Runnable node example (Phase H): echo roundtrip through the native
 * reference gateway using the BUILT wasm package (nodejs target).
 *
 * Prereqs (local only, no publish):
 *   1. npm run build:node        # wasm-pack -> ./pkg-node/
 *   2. Start the reference gateway (TCP, proto v0) — see TUTORIAL_H.md.
 *      NOTE: the gateway speaks raw TCP frames, not WebSocket; this
 *      example therefore exercises the pure codec exports
 *      (u64/i64 BigInt + echo frame roundtrip) under node, and shows the
 *      exact `WasmClient.connect` call to use against a WebSocket bridge
 *      in the browser (see example.html).
 *
 * Run: node example.mjs
 */
import { strict as assert } from 'node:assert';
import init, {
  u64_to_bigint,
  bigint_to_u64,
  i64_to_bigint,
  bigint_to_i64,
} from './pkg-node/cyclonedds_wasm.js';

await init();

// --- BigInt: exact above Number.MAX_SAFE_INTEGER ---------------------------
const MAX_U64 = 18446744073709551615n;
assert.equal(bigint_to_u64(u64_to_bigint(MAX_U64)), MAX_U64);
assert.equal(bigint_to_u64(9007199254740993n), 9007199254740993n); // 2^53+1

const MIN_I64 = -9223372036854775808n;
assert.equal(bigint_to_i64(i64_to_bigint(MIN_I64)), MIN_I64);
assert.equal(bigint_to_i64(42n), 42n);

// Out-of-range throws (never wraps): negative to u64, 2^64 to u64.
assert.throws(() => bigint_to_u64(-1n));
assert.throws(() => bigint_to_u64(18446744073709551616n));
assert.throws(() => bigint_to_i64(9223372036854775808n));

console.log('bigint u64/i64 roundtrips: OK (exact above 2^53)');

// --- Browser usage (needs a WebSocket bridge + browser/node WebSocket) -----
// import init2, { WasmClient } from './pkg-node/cyclonedds_wasm.js';
// await init2();
// const client = await WasmClient.connect('ws://127.0.0.1:8080/dds', 5000);
// client.on_echo((s) => console.log('echo:', s.topic, s.id, s.text, s.seq));
// client.send_echo('WasmEcho', 7, 'hello from js', [1, 2, 3]);
// client.close();

console.log('example.mjs: OK');
