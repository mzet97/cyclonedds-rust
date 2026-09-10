// WS<->TCP proxy for the battleship game (and any DataFrame client).
// Zero dependencies (node builtins only), same pattern as the reference
// `ws_echo_server.mjs`: HTTP upgrade + SHA-1 handshake by hand.
//
// Framing contract (must match both ends):
//   browser -> here: one WS binary message = one raw DataFrame
//     (NO length prefix; see cyclonedds-wasm `encode_echo_frame`)
//   here -> gateway TCP: u32-LE length + DataFrame bytes
//     (see dds-wasm-bridge `read_packet`)
//   gateway TCP -> here: split the byte stream on u32-LE length
//   here -> browser: one WS binary message per payload (no prefix)
//
// Usage: node ws_dds_proxy.mjs [ws-port] [dds-host] [dds-port]
//   defaults: 8901 127.0.0.1 <printed by battleship-gateway as READY>
import { createServer } from 'http';
import { createHash } from 'crypto';
import { connect } from 'net';

const wsPort = Number(process.argv[2] || 8901);
const ddsHost = process.argv[3] || '127.0.0.1';
const ddsPort = Number(process.argv[4]);
if (!ddsPort) {
  console.error('usage: node ws_dds_proxy.mjs [ws-port] [dds-host] <dds-port>');
  process.exit(1);
}

function accept(key) {
  return createHash('sha1').update(key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
}

// ---- WS frame helpers (server side: unmasked out, masked in) ----
function wsSend(sock, payload) {
  const n = payload.length;
  let header;
  if (n < 126) header = Buffer.from([0x82, n]);
  else if (n < 65536) { header = Buffer.alloc(4); header[0] = 0x82; header[1] = 126; header.writeUInt16BE(n, 2); }
  else { header = Buffer.alloc(10); header[0] = 0x82; header[1] = 127; header.writeBigUInt64BE(BigInt(n), 2); }
  sock.write(Buffer.concat([header, payload]));
}

// Minimal masked-text/binary frame parser (client->server always masked).
function wsReader(sock, onMessage, onClose) {
  let buf = Buffer.alloc(0);
  sock.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    pump();
  });
  sock.on('close', onClose);
  sock.on('error', onClose);
  function pump() {
    for (;;) {
      if (buf.length < 2) return;
      const masked = (buf[1] & 0x80) !== 0;
      let len = buf[1] & 0x7f;
      let off = 2;
      if (len === 126) { if (buf.length < 4) return; len = buf.readUInt16BE(2); off = 4; }
      else if (len === 127) { if (buf.length < 10) return; len = Number(buf.readBigUInt64BE(2)); off = 10; }
      const mask = masked ? 4 : 0;
      if (buf.length < off + mask + len) return;
      const opcode = buf[0] & 0x0f;
      let payload = buf.subarray(off + mask, off + mask + len);
      if (masked) {
        const key = buf.subarray(off, off + 4);
        const out = Buffer.alloc(len);
        for (let i = 0; i < len; i++) out[i] = payload[i] ^ key[i % 4];
        payload = out;
      }
      buf = buf.subarray(off + mask + len);
      if (opcode === 0x8) { onClose(); return; } // close frame
      onMessage(payload);
    }
  }
}

// ---- TCP side: length-prefixed stream ----
function tcpReader(sock, onFrame) {
  let buf = Buffer.alloc(0);
  sock.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    while (buf.length >= 4) {
      const n = buf.readUInt32LE(0);
      if (n > 8 * 1024 * 1024 + 64 || buf.length < 4 + n) return;
      onFrame(buf.subarray(4, 4 + n));
      buf = buf.subarray(4 + n);
    }
  });
}

const server = createServer();
server.on('upgrade', (req, sock) => {
  const key = req.headers['sec-websocket-key'];
  if (!key) { sock.destroy(); return; }
  sock.write(
    'HTTP/1.1 101 Switching Protocols\r\n' +
    'Upgrade: websocket\r\nConnection: Upgrade\r\n' +
    `Sec-WebSocket-Accept: ${accept(key)}\r\n\r\n`
  );
  const dds = connect(ddsPort, ddsHost, () => {});
  let dead = false;
  const die = () => { if (!dead) { dead = true; try { sock.destroy(); } catch {} try { dds.destroy(); } catch {} } };
  // Browser -> DDS: add the u32-LE prefix the TCP gateway expects.
  wsReader(sock, (payload) => {
    const out = Buffer.alloc(4 + payload.length);
    out.writeUInt32LE(payload.length, 0);
    payload.copy(out, 4);
    dds.write(out);
  }, die);
  // DDS -> browser: one WS message per length-delimited frame.
  tcpReader(dds, (frame) => { if (!dead) wsSend(sock, frame); });
  dds.on('close', die);
  dds.on('error', die);
});

server.listen(wsPort, '127.0.0.1', () => {
  console.log(`READY ws=127.0.0.1:${wsPort} dds=${ddsHost}:${ddsPort}`);
});
