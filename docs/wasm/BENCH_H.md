# Benchmarks Fase H — latência / throughput / tamanho

Máquina: host de desenvolvimento (x86_64, criterion 0.5, release).
Amostra: `WasmEcho{id: 42, text: "hello-wasm-benchmark" (20 chars),
values: 0..16}` → CDR 100 bytes, frame proto v0 124 bytes.
Comandos em `docs/wasm/TUTORIAL_H.md` §4. Baseline nativo: libddsc via
`CdrSerializer`/`CdrDeserializer` (`CdrEncoding::Xcdr1`); identidade de
bytes provada em
`dds-wasm-bridge/tests/phase_c.rs::codec_matches_native_libddsc_byte_for_byte`.

## Latência (criterion, mediana, ns/op)

| bench | medição | ns/op |
|---|---|---|
| `xcdr1_encode/encode_echo_xcdr1` | codec wasm (puro) | **30.1** |
| `encode_wasm_vs_native/native_libddsc_encode` | baseline nativo | **542.9** |
| `xcdr1_decode/decode_echo_xcdr1` | codec wasm (puro) | **46.7** |
| `decode_wasm_vs_native/native_libddsc_decode` | baseline nativo | **541.7** |
| `frame_roundtrip/encode_echo_frame` | frame v0 completo | **57.5** |
| `frame_roundtrip/decode_echo_frame` | frame v0 completo | **91.0** |

Leitura: o codec puro é ~18× mais rápido que o caminho libddsc no
encode e ~12× no decode **neste micro-benchmark** — o nativo inclui
alocação via DDS allocator, `DdsSequence` e validação de tipos; o puro
opera sobre `Vec`/`String` já validados. Não é "wasm mais rápido que
nativo" em geral: é o custo do codec isolado, e o número que importa
para o browser é o absoluto (~30–90 ns, irrelevante perto de 1 RTT de
WebSocket).

## Throughput derivado (bytes medidos / latência)

| caminho | bytes | throughput |
|---|---|---|
| encode CDR puro | 100 | ~3.3 GB/s |
| decode CDR puro | 100 | ~2.1 GB/s |
| encode frame v0 | 124 | ~2.2 GB/s |
| decode frame v0 | 124 | ~1.4 GB/s |

## Tamanho do artefato wasm

| artefato | bytes |
|---|---|
| `target/wasm32-unknown-unknown/release/cyclonedds_wasm.wasm` (cdylib cru, sem `wasm-bindgen`/otimização de tamanho) | **687 282** |
| `target/wasm32-unknown-unknown/debug/cyclonedds_wasm.wasm` | 8 251 212 |

Limites honestos: o `.wasm` acima é o cdylib cru do `cargo build`
(`crate-type = ["cdylib", "rlib"]`), NÃO a saída do `wasm-pack`
(pós-processamento + glue JS) — `wasm-pack` está ausente neste
ambiente, então o tamanho final do pacote (`js/pkg/`) é **não medido**.
Sem `wasm-opt`/LTO dedicado, 687 KiB é o teto, não o piso. O número
serve como baseline de regressão de tamanho no CI (`ls -la` no job
`wasm32`).
