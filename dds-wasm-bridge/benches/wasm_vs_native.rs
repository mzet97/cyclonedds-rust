//! Phase H benchmarks: wasm-shipped codec vs native libddsc baseline.
//!
//! Compares the pure-Rust browser codec (`cyclonedds-proto`, the exact
//! bytes shipped in the wasm artifact) against the native libddsc
//! `CdrSerializer`/`CdrDeserializer` for the same `WasmEcho` sample.
//! Byte-identity is established by
//! `dds-wasm-bridge/tests/phase_c.rs::codec_matches_native_libddsc_byte_for_byte`;
//! this bench measures the time side. Run with:
//! `cargo bench -p dds-wasm-bridge --bench wasm_vs_native`.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use cyclonedds::{CdrDeserializer, CdrEncoding, CdrSerializer};
use cyclonedds_proto::echo::{decode_echo_xcdr1, encode_echo_xcdr1, EchoMsg};
use dds_wasm_bridge::WasmEcho;

fn sample() -> EchoMsg {
    EchoMsg {
        id: 42,
        text: "hello-wasm-benchmark".into(),
        values: (0..16).collect(),
    }
}

fn bench_encode_vs_native(c: &mut Criterion) {
    let msg = sample();
    let native = WasmEcho::from_proto(&msg).unwrap();
    let bytes = encode_echo_xcdr1(&msg).unwrap();
    let mut group = c.benchmark_group("encode_wasm_vs_native");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("wasm_proto_encode", |b| {
        b.iter(|| black_box(encode_echo_xcdr1(black_box(&msg)).unwrap()));
    });
    group.bench_function("native_libddsc_encode", |b| {
        b.iter(|| {
            black_box(
                CdrSerializer::<WasmEcho>::serialize(black_box(&native), CdrEncoding::Xcdr1)
                    .unwrap(),
            )
        });
    });
    group.finish();
}

fn bench_decode_vs_native(c: &mut Criterion) {
    let msg = sample();
    let bytes = encode_echo_xcdr1(&msg).unwrap();
    let mut group = c.benchmark_group("decode_wasm_vs_native");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("wasm_proto_decode", |b| {
        b.iter(|| black_box(decode_echo_xcdr1(black_box(&bytes)).unwrap()));
    });
    group.bench_function("native_libddsc_decode", |b| {
        b.iter(|| {
            black_box(
                CdrDeserializer::<WasmEcho>::deserialize(black_box(&bytes), CdrEncoding::Xcdr1)
                    .unwrap(),
            )
        });
    });
    group.finish();
}

criterion_group!(benches, bench_encode_vs_native, bench_decode_vs_native);
criterion_main!(benches);
