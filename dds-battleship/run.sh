#!/bin/bash
# Sobe a Batalha Naval DDS local: pacote WASM + gateway nativo + proxy
# WS->TCP + servidor estático. Nada é instalado; tudo roda do repo.
# Uso: ./run.sh   (CTRL-C derruba os 3 processos auxiliares)
set -euo pipefail

CRATE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$CRATE/.." && pwd)
cd "$ROOT"

# 8080 é comumente o remote-debugging do Chrome: padrão 8900.
HTTP_PORT=${BATTLESHIP_HTTP_PORT:-8900}
WS_PORT=${BATTLESHIP_WS_PORT:-8901}
# Loopback sempre: browser, proxy e gateway são todos 127.0.0.1 e o DDS
# nunca sai da máquina (também blinda contra NICs obsoletas no ambiente,
# como `enp4s0` em XMLs antigos).
if [ -n "${CYCLONEDDS_URI:-}" ]; then
  echo "(ignorando CYCLONEDDS_URI do ambiente: jogo é localhost)"
fi
export CYCLONEDDS_URI='<CycloneDDS><Domain><General><Interfaces><NetworkInterface name="lo"/></Interfaces></General></Domain></CycloneDDS>'

echo "[1/4] compilando o cliente WASM…"
cargo build --target wasm32-unknown-unknown -p dds-battleship --lib 2>&1 | tail -1
wasm-bindgen --target web --out-dir "$CRATE/site/pkg" \
  target/wasm32-unknown-unknown/debug/dds_battleship.wasm 2>&1 | tail -1

echo "[2/4] subindo o gateway DDS (tópicos battleship/*)…"
GW_LOG=$(mktemp)
cargo run -q -p dds-battleship --bin battleship-gateway >"$GW_LOG" 2>&1 &
GW_PID=$!
GW_PORT=""
for _ in $(seq 1 100); do
  GW_PORT=$(grep -oE "READY addr=127.0.0.1:[0-9]+" "$GW_LOG" | grep -oE "[0-9]+$" || true)
  [ -n "$GW_PORT" ] && break
  sleep 0.2
done
[ -n "$GW_PORT" ] || { echo "gateway não subiu; veja $GW_LOG"; kill $GW_PID 2>/dev/null; exit 1; }
echo "        gateway no TCP 127.0.0.1:$GW_PORT (pid $GW_PID)"

echo "[3/4] subindo o proxy WS->TCP na porta $WS_PORT…"
node "$CRATE/support/ws_dds_proxy.mjs" "$WS_PORT" 127.0.0.1 "$GW_PORT" >"$GW_LOG.proxy" 2>&1 &
PROXY_PID=$!
sleep 0.5
grep -q READY "$GW_LOG.proxy" || { echo "proxy não subiu"; kill $GW_PID $PROXY_PID 2>/dev/null; exit 1; }

echo "[4/4] servindo o site em http://127.0.0.1:$HTTP_PORT …"
cleanup() { kill $GW_PID $PROXY_PID 2>/dev/null; }
trap cleanup EXIT INT TERM
echo ""
echo "Abra DOIS navegadores/janelas em http://127.0.0.1:$HTTP_PORT"
echo "Um cria o jogo, o outro entra pelo saguão. CTRL-C para derrubar."
python3 -m http.server "$HTTP_PORT" -d "$CRATE/site"
