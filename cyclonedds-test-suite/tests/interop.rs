use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Test cross-process pub/sub using example binaries.
#[test]
fn cross_process_pub_sub_reliable() {
    let domain_id = 99; // Use high domain to avoid conflicts
    let topic_name = format!("interop_test_{}", std::process::id());
    let count = 5;

    // Build examples first
    let build_status = Command::new("cargo")
        .args([
            "build",
            "--example",
            "interop_pub",
            "--example",
            "interop_sub",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("failed to build examples");
    assert!(build_status.success(), "example build failed");

    let workspace_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("must be in workspace");
    let pub_bin = workspace_root
        .join("target/debug/examples/interop_pub")
        .to_string_lossy()
        .to_string();
    let sub_bin = workspace_root
        .join("target/debug/examples/interop_sub")
        .to_string_lossy()
        .to_string();

    let env_vars = [
        ("DDS_DOMAIN_ID", domain_id.to_string()),
        ("DDS_TOPIC_NAME", topic_name.clone()),
        ("DDS_PUB_COUNT", count.to_string()),
    ];

    // Start subscriber first
    let mut sub = Command::new(&sub_bin)
        .envs(env_vars.iter().map(|(k, v)| (*k, v.as_str())))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start subscriber");

    // Give subscriber time to start
    std::thread::sleep(Duration::from_millis(300));

    // Start publisher
    let mut pub_ = Command::new(&pub_bin)
        .envs(env_vars.iter().map(|(k, v)| (*k, v.as_str())))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start publisher");

    // Wait for both with timeout
    let timeout = Duration::from_secs(30);
    let start = Instant::now();

    let pub_result = loop {
        if start.elapsed() > timeout {
            let _ = pub_.kill();
            let _ = sub.kill();
            panic!("publisher timed out");
        }
        match pub_.try_wait().expect("failed to check publisher") {
            Some(status) => break status,
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    assert!(pub_result.success(), "publisher exited with error");

    let sub_result = loop {
        if start.elapsed() > timeout {
            let _ = sub.kill();
            panic!("subscriber timed out");
        }
        match sub.try_wait().expect("failed to check subscriber") {
            Some(status) => break status,
            None => std::thread::sleep(Duration::from_millis(100)),
        }
    };

    assert!(sub_result.success(), "subscriber exited with error");
}
