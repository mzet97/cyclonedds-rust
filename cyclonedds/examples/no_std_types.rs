//! Minimal example of defining a DDS type using the `no_std` compatible API.
//!
//! This example demonstrates how to use the types and constants exported by
//! the `no_std_types` module to define a DDS-compatible type descriptor.
//!
//! In a real `no_std` embedded target you would compile with:
//! ```bash
//! cargo build --no-default-features --features no_std --target thumbv7em-none-eabihf
//! ```

use cyclonedds::{adr, DdsType, OP_RTS, TYPE_4BY};

/// A simple sensor reading type that can be serialized to DDS CDR.
#[repr(C)]
pub struct SensorReading {
    pub sensor_id: i32,
    pub temperature: f32,
}

impl DdsType for SensorReading {
    fn type_name() -> &'static str {
        "SensorReading"
    }

    fn ops() -> Vec<u32> {
        let mut ops = Vec::new();
        // sensor_id: i32 @ offset 0 (signed 4-byte)
        ops.extend(adr(TYPE_4BY | (1 << 2), 0));
        // temperature: f32 @ offset 4 (4-byte float)
        ops.extend(adr(TYPE_4BY, 4));
        // end of type
        ops.push(OP_RTS);
        ops
    }
}

fn main() {
    // In no_std we cannot create a DomainParticipant (needs FFI).
    // But we can inspect the type descriptor.
    assert_eq!(SensorReading::type_name(), "SensorReading");
    assert_eq!(SensorReading::ops().len(), 5); // 2x adr + OP_RTS
    assert_eq!(SensorReading::descriptor_size(), 8); // i32 + f32
    println!("no_std type descriptor works!");
}
