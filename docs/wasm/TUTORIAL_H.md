# Tutorial verificável — pacote consumível WASM (Fase H)

Base: `main` / `f0f766b`. Todos os comandos abaixo foram executados
nesta sessão salvo marcação explícita em contrário. Regra dura mantida:
nada commitado, nada publicado, só working tree.

Pré-requisitos observados: toolchain Rust 1.95.0 estável,
target `wasm32-unknown-unknown` instalado, `node` v24. `wasm-pack`,
`tsc`, `wasmtime` e `emcc` **ausentes** (passos que precisam deles estão
marcados como NÃO EXECUTADOS).

## 1. Testes dos perfis suportados

```sh
cargo test -p cyclonedds-proto --locked
# ok: 6 + 8 passed (14/14)

cargo test -p cyclonedds-wasm --locked
# ok: 10 passed (6 do gateway + 4 do mapeamento BigInt u64/i64)

cargo test -p dds-wasm-guest --locked
# ok: 5 passed
```

O gateway nativo precisa de interface de rede explícita no sandbox
(`DomainParticipant::new` falha com `ReturnCode(-1)` porque o default
mira `enp4s0`, inexistente aqui):

```sh
export CYCLONEDDS_URI='<CycloneDDS><Domain><General><NetworkInterfaceAddress>lo</NetworkInterfaceAddress></General></Domain></CycloneDDS>'
cargo test -p dds-wasm-bridge --locked -- --test-threads=1
# ok: 9 (parity) + 6 (phase_c) + 11 (phase_d) + 3 (wit_pubsub) = 29 passed
```

## 2. Exemplo executável (host, sem browser)

```sh
cargo run -p cyclonedds-wasm --example echo_host --locked
# echo frame roundtrip: OK (72 bytes)
# reliable -> Unsupported: OK
# bigint u64/i64 mapping: OK (exact above 2^53)
# host write -> NotConnected: OK (transport gap is typed)
```

## 3. Checagens wasm32 (o código exato embarcado no browser)

```sh
rustup target list --installed
# wasm32-unknown-unknown + x86_64-unknown-linux-gnu

cargo check -p cyclonedds-wasm -p dds-wasm-guest \
  --target wasm32-unknown-unknown --locked
# Finished, sem warnings

RUSTFLAGS="-D warnings" cargo check -p cyclonedds-wasm \
  --target wasm32-unknown-unknown --locked --offline
# Finished (o CI bloqueia warnings como erro)

# O backend nativo não pode vazar no grafo web:
cargo tree -p cyclonedds-wasm --target wasm32-unknown-unknown \
  --prefix none | grep 'cyclonedds-rust-sys '
# (vazio = limpo; só wasm-bindgen/js-sys/web-sys no grafo)
```

## 4. Benchmarks (latência/throughput + tamanho)

```sh
cargo bench -p cyclonedds-wasm --bench echo_codec --locked
cargo bench -p dds-wasm-bridge --bench wasm_vs_native --locked
cargo build -p cyclonedds-wasm --target wasm32-unknown-unknown --release --locked
ls -la target/wasm32-unknown-unknown/release/cyclonedds_wasm.wasm
# Números em docs/wasm/BENCH_H.md
```

## 5. Pacote JS/TS (NÃO EXECUTADO aqui — sem wasm-pack)

```sh
# Requer: cargo install wasm-pack (rede). Não executado nesta sessão.
npm run build       # alvo web   -> cyclonedds-wasm/js/pkg/
npm run build:node  # alvo node  -> cyclonedds-wasm/js/pkg-node/
node cyclonedds-wasm/js/example.mjs   # BigInt + exemplo de connect
```

`node --check` nos `.mjs` passa; `index.d.ts` é handwritten e **não**
foi validado com `tsc` (ausente, sem rede para `npx -y tsc`).
O browser (`js/example.html`) nunca foi aberto nesta sessão.

## 6. CI bloqueante

`.github/workflows/wasm.yml`, 5 jobs (`proto`,
`browser-gateway-host`, `bridge-and-guest`, `wasm32`, `bench-smoke`),
sem `continue-on-error`, sem `|| true`. Validação local:

```sh
grep -n 'continue-on-error' .github/workflows/wasm.yml
# só comentários; nenhum uso real
python3 -c "import yaml; print(sorted(yaml.safe_load(
  open('.github/workflows/wasm.yml'))['jobs']))"
# ['bench-smoke', 'bridge-and-guest', 'browser-gateway-host', 'proto', 'wasm32']
```
