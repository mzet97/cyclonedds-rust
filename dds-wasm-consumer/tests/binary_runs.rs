//! The built binary is the Phase B separation proof: it must link and
//! run with no native backend in its graph.

#[test]
fn binary_reports_ok_on_stdout_and_exits_zero() {
    let exe = env!("CARGO_BIN_EXE_dds-wasm-consumer");
    let out = std::process::Command::new(exe)
        .output()
        .expect("consumer binary runs");
    assert!(out.status.success(), "exit zero");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dds-wasm-consumer: OK"), "{stdout}");
}
