// Minimal WebSocket echo server for the wasm32 client tests.
// Zero dependencies (node builtins only): HTTP upgrade + SHA-1 handshake,
// masked client frames in, unmasked binary frames out. Reference tool for
// `tests/wasm_node.rs` and manual browser runs — NOT production code.
//
// Usage: node tests/support/ws_echo_server.mjs [port]   (default 18711)
import { createServer } from 'http';
import { createHash } from 'crypto';

const port = Number(process.argv[2] || 18711);

function accept(key) {
  return createHash('sha1').update(key + '258EAFA5-E914-47DA-95CA-C5AB0DC85B11').digest('base64');
}

// Read exactly n bytes from a socket (queues surplus).
function reader(sock) {
  let buf = Buffer.alloc(0);
  let waiters = [];
  sock.on('data', (chunk) => {
    buf = Buffer.concat([buf, chunk]);
    pump();
  });
  sock.on('close', () => {
    for (const [, rej] of waiters.splice(0)) rej(new Error('closed'));
  });
  function pump() {
    while (waiters.length && buf.length >= waiters[0][0]) {
      const [n, res] = waiters.shift();
      const out = buf.subarray(0, n);
      buf = buf.subarray(n);
      res(out);
    }
  }
  return (n) => new Promise((res, rej) => {
    waiters.push([n, res, rej]);
    pump();
  });
}

async function frame(read) {
  const h = await read(2);
  const fin = h[0] & 0x80;
  const opcode = h[0] & 0x0f;
  const masked = h[1] & 0x80;
  let len = h[1] & 0x7f;
  if (len === 126) len = (await read(2)).readUInt16BE(0);
  else if (len === 127) len = Number((await read(8)).readBigUInt64BE(0));
  let mask = masked ? await read(4) : null;
  let payload = await read(len);
  if (masked) {
    const out = Buffer.alloc(len);
    for (let i = 0; i < len; i++) out[i] = payload[i] ^ mask[i % 4];
    payload = out;
  }
  return { fin, opcode, payload };
}

function send(sock, opcode, payload) {
  const h = Buffer.alloc(2);
  h[0] = 0x80 | opcode;
  if (payload.length < 126) {
    h[1] = payload.length;
    sock.write(Buffer.concat([h, payload]));
  } else {
    h[1] = 126;
    const ext = Buffer.alloc(2);
    ext.writeUInt16BE(payload.length, 0);
    sock.write(Buffer.concat([h, ext, payload]));
  }
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
  const read = reader(sock);
  // An abrupt client disconnect must never take the server down.
  sock.on('error', () => {});
  (async () => {
    try {
      for (;;) {
        const f = await frame(read);
        if (f.opcode === 0x8) return;               // close
        if (f.opcode === 0x9) { send(sock, 0xa, f.payload); continue; } // ping
        if (f.opcode === 0x2 || f.opcode === 0x1) send(sock, 0x2, f.payload); // echo as binary
      }
    } catch { /* client gone */ }
  })();
});
server.listen(port, '127.0.0.1', () => console.log(`ws-echo on 127.0.0.1:${port}`));
