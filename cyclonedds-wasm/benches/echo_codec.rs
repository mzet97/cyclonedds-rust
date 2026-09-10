//! Phase H benchmarks: browser data-plane codec (the exact code shipped
//! in the wasm artifact) — encode/decode latency and throughput.
//!
//! The native-baseline comparison lives in
//! `dds-wasm-bridge/benches/wasm_vs_native.rs` (this crate must never
//! depend on the native backend). Run with:
//! `cargo bench -p cyclonedds-wasm --bench echo_codec`.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use cyclonedds_proto::echo::{decode_echo_xcdr1, encode_echo_xcdr1, EchoMsg};
use cyclonedds_wasm::{decode_echo_frame, encode_echo_frame};

fn sample() -> EchoMsg {
    EchoMsg {
        id: 42,
        text: "hello-wasm-benchmark".into(),
        values: (0..16).collect(),
    }
}

fn bench_encode_xcdr1(c: &mut Criterion) {
    let msg = sample();
    let mut group = c.benchmark_group("xcdr1_encode");
    let bytes = encode_echo_xcdr1(&msg).unwrap();
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("encode_echo_xcdr1", |b| {
        b.iter(|| black_box(encode_echo_xcdr1(black_box(&msg)).unwrap()));
    });
    group.finish();
}

fn bench_decode_xcdr1(c: &mut Criterion) {
    let msg = sample();
    let bytes = encode_echo_xcdr1(&msg).unwrap();
    let mut group = c.benchmark_group("xcdr1_decode");
    group.throughput(Throughput::Bytes(bytes.len() as u64));
    group.bench_function("decode_echo_xcdr1", |b| {
        b.iter(|| black_box(decode_echo_xcdr1(black_box(&bytes)).unwrap()));
    });
    group.finish();
}

fn bench_frame_roundtrip(c: &mut Criterion) {
    let msg = sample();
    let frame = encode_echo_frame("WasmEcho", &msg, 7).unwrap();
    let mut group = c.benchmark_group("frame_roundtrip");
    group.throughput(Throughput::Bytes(frame.len() as u64));
    group.bench_function("encode_echo_frame", |b| {
        b.iter(|| black_box(encode_echo_frame("WasmEcho", black_box(&msg), 7).unwrap()));
    });
    group.bench_function("decode_echo_frame", |b| {
        b.iter(|| black_box(decode_echo_frame(black_box(&frame)).unwrap()));
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_encode_xcdr1,
    bench_decode_xcdr1,
    bench_frame_roundtrip
);
criterion_main!(benches);
