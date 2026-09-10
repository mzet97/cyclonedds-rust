//! Minimal web consumer (Phase B separation proof).
//!
//! Links ONLY the portable gateway client (`cyclonedds-wasm`) and the
//! portable contracts (`cyclonedds-proto`): no `cyclonedds` native backend,
//! no `cyclonedds-rust-sys`, no libddsc. Run `cargo tree -p
//! dds-wasm-consumer -i cyclonedds-rust-sys` — it must report "not found".
//!
//! On `wasm32` this drives the live WebSocket; on host it exercises the
//! portable subset (envelope encode, QoS treatment) and reports the
//! transport gap as typed `NotConnected` instead of failing to link.

use cyclonedds_proto::{check_qos, Control, DataFrame, LegacyEnvelope, Qos, FLAG_CDR_LE};
use cyclonedds_wasm::{check_gateway_qos, encode_sample, WasmDdsError, WasmDomainParticipant};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Hello {
    id: i32,
    text: String,
}

fn main() {
    std::process::exit(report_run(run()));
}

/// Map one `run()` outcome to a process exit code. Split out so both
/// arms are unit-testable (the `Err` arm cannot happen on host, but a
/// future transport may fail — the mapping must still be pinned).
fn report_run(result: Result<(), WasmDdsError>) -> i32 {
    match result {
        Ok(()) => {
            println!("dds-wasm-consumer: OK");
            0
        }
        Err(e) => {
            eprintln!("dds-wasm-consumer: {e}");
            1
        }
    }
}

/// Report one transport write outcome. Split out so every arm is
/// unit-testable on host (the live `Ok` arm only happens on wasm32).
fn report_write(result: Result<(), WasmDdsError>) -> Result<(), WasmDdsError> {
    match result {
        Ok(()) => println!("published live (wasm32 transport)"),
        Err(WasmDdsError::NotConnected) => {
            println!("host target: portable path OK, transport NotConnected (expected)");
        }
        Err(e) => return Err(e),
    }
    Ok(())
}

fn run() -> Result<(), WasmDdsError> {
    // 1. Portable envelope: byte-compatible with the legacy bridge format.
    let sample = Hello {
        id: 1,
        text: "hello".to_string(),
    };
    let env_json = encode_sample("HelloWorld", &sample)?;
    let env = LegacyEnvelope::decode(&env_json)?;
    assert_eq!(env.topic, "HelloWorld");

    // 2. Gateway QoS treatment: default accepted, reliable refused typed.
    check_gateway_qos(&Qos::default())?;
    let reliable = Qos {
        reliability: cyclonedds_proto::Reliability::Reliable,
        durability: cyclonedds_proto::Durability::Volatile,
    };
    assert!(matches!(
        check_qos(&reliable),
        Err(cyclonedds_proto::ProtoError::Unsupported(_))
    ));

    // 3. Versioned control + binary frame contracts (proto v0).
    let hello = Control::Hello {
        proto: cyclonedds_proto::PROTO_VERSION,
        client: "dds-wasm-consumer/0.1".to_string(),
    };
    hello.check_version()?;
    let frame = DataFrame {
        proto: cyclonedds_proto::PROTO_VERSION,
        flags: FLAG_CDR_LE,
        topic: "HelloWorld".to_string(),
        cdr: vec![1, 2, 3],
        seq: 1,
    };
    let bytes = frame.encode()?;
    assert_eq!(DataFrame::decode(&bytes)?, frame);

    // 4. Client API links without the native backend. On wasm32 this opens
    // the socket; on host it records topics and reports NotConnected on I/O.
    let participant = WasmDomainParticipant::new("ws://localhost:8080/dds")?;
    let topic = participant.create_topic::<Hello>("HelloWorld")?;
    let writer = participant.create_writer(&topic)?;
    report_write(writer.write(&sample))?;
    participant.disconnect();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_write_names_every_outcome() {
        report_write(Ok(())).unwrap();
        report_write(Err(WasmDdsError::NotConnected)).unwrap();
        assert!(matches!(
            report_write(Err(WasmDdsError::Timeout)),
            Err(WasmDdsError::Timeout)
        ));
    }

    #[test]
    fn report_run_maps_both_outcomes_to_exit_codes() {
        assert_eq!(report_run(Ok(())), 0);
        assert_eq!(report_run(Err(WasmDdsError::Timeout)), 1);
    }

    #[test]
    fn portable_path_runs_without_native_backend() {
        run().unwrap();
    }
}
