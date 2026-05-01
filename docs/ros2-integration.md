# ROS2 Integration

This guide explains how to use `cyclonedds-rust` to communicate with ROS2 nodes.

## Background

ROS2 uses DDS as its underlying middleware. By default, ROS2 Humble+ uses **Eclipse CycloneDDS** as the RMW (ROS Middleware) implementation. This means ROS2 nodes are actually DDS participants under the hood, and their topics are standard DDS topics.

Because `cyclonedds-rust` is built on top of CycloneDDS, it can natively interoperate with ROS2 nodes running on the same DDS domain.

## Topic Naming

ROS2 topics are mapped to DDS topics using the following rules:

| ROS2 Topic | DDS Topic Name |
|------------|----------------|
| `/turtle1/cmd_vel` | `/turtle1/cmd_vel` |
| `/chatter` | `/chatter` |
| `/rosout` | `/rosout` |

The leading slash is preserved in the DDS topic name.

## Message Types

ROS2 message types are defined in IDL and compiled to C++ structs. To communicate with ROS2 from Rust, you need to define equivalent Rust structs and derive `DdsType`.

### Common ROS2 Message Types

#### `geometry_msgs/Twist`

```rust
use cyclonedds_derive::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone, Debug)]
struct Vector3 {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(DdsTypeDerive, Clone, Debug)]
struct Twist {
    linear: Vector3,
    angular: Vector3,
}
```

#### `geometry_msgs/Pose`

```rust
#[derive(DdsTypeDerive, Clone, Debug)]
struct Point {
    x: f64,
    y: f64,
    z: f64,
}

#[derive(DdsTypeDerive, Clone, Debug)]
struct Quaternion {
    x: f64,
    y: f64,
    z: f64,
    w: f64,
}

#[derive(DdsTypeDerive, Clone, Debug)]
struct Pose {
    position: Point,
    orientation: Quaternion,
}
```

#### `std_msgs/String`

```rust
#[derive(DdsTypeDerive, Clone, Debug)]
struct StringMsg {
    data: String,
}
```

## QoS Considerations

ROS2 uses specific QoS profiles. When communicating with ROS2 nodes, you should match their QoS settings:

| ROS2 Profile | DDS QoS |
|--------------|---------|
| Default | Reliable, Volatile, KeepLast(10) |
| Sensor Data | BestEffort, Volatile, KeepLast(5) |
| Services | Reliable, Volatile, KeepLast(1) |
| Parameters | Reliable, TransientLocal, KeepLast(1) |

Example QoS for a sensor publisher:

```rust
use cyclonedds::QosBuilder;

let qos = QosBuilder::new()
    .reliability(cyclonedds::Reliability::BestEffort)
    .durability(cyclonedds::Durability::Volatile)
    .history(cyclonedds::History::KeepLast(5))
    .build()?;
```

## Turtlesim Demo

The `examples/ros2_turtlesim.rs` example demonstrates moving a turtle in a circle by publishing `Twist` messages.

### Running the Demo

1. **Start ROS2 and turtlesim** (in a ROS2 environment):
   ```bash
   source /opt/ros/humble/setup.bash
   ros2 run turtlesim turtlesim_node
   ```

2. **Run the Rust publisher**:
   ```bash
   cargo run --example ros2_turtlesim
   ```

The turtle should move in a circle.

### Echoing ROS2 Topics from Rust

You can also subscribe to ROS2 topics from Rust. For example, to listen to `/turtle1/pose`:

```rust
use cyclonedds::{DomainParticipant, Subscriber, Topic, DataReader};
use cyclonedds_derive::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone, Debug)]
struct Pose {
    x: f32,
    y: f32,
    theta: f32,
    linear_velocity: f32,
    angular_velocity: f32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let subscriber = Subscriber::new(participant.entity())?;
    let topic = Topic::<Pose>::new(participant.entity(), "/turtle1/pose")?;
    let reader = DataReader::new(subscriber.entity(), topic.entity())?;

    loop {
        for pose in reader.take()? {
            println!("x={:.2}, y={:.2}, theta={:.2}", pose.x, pose.y, pose.theta);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
```

## Generating Types from ROS2 IDL

ROS2 message definitions are stored in `.msg` files, which are converted to IDL during the build process. You can find the generated IDL files in your ROS2 installation:

```bash
/opt/ros/humble/share/geometry_msgs/msg/Twist.idl
```

You can use `cyclonedds-idlc` to compile these IDL files to Rust:

```bash
cargo run --bin cyclonedds-idlc -- \
  --input /opt/ros/humble/share/geometry_msgs/msg/Twist.idl \
  --output-dir src/ros2_types
```

## Domain ID

ROS2 uses DDS domain 0 by default. You can change the domain ID by setting the `ROS_DOMAIN_ID` environment variable:

```bash
export ROS_DOMAIN_ID=42
ros2 run turtlesim turtlesim_node
```

When connecting from Rust, use the matching domain ID:

```rust
let participant = DomainParticipant::new(42)?;
```

## Troubleshooting

### No discovery
- Ensure both ROS2 and your Rust application use the same DDS domain ID.
- Check that firewalls are not blocking multicast traffic (DDS discovery uses multicast by default).
- On some networks, you may need to configure CycloneDDS to use unicast discovery via the `CYCLONEDDS_URI` environment variable.

### Type mismatch
- Ensure your Rust struct layout exactly matches the ROS2 message definition.
- Pay attention to field ordering — DDS CDR serialization is order-dependent.
- Use `std_msgs/String` instead of native `String` for string fields when interoperating with ROS2.

### QoS mismatch
- If messages are not received, check that the publisher and subscriber QoS are compatible.
- ROS2's `sensor_data` QoS uses BestEffort, which is incompatible with a Reliable subscriber.
