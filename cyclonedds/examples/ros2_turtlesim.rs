//! ROS2 turtlesim demo — move the turtle in a circle.
//!
//! This example demonstrates interoperability with ROS2 by publishing
//! `geometry_msgs/Twist` messages to the `/turtle1/cmd_vel` topic.
//!
//! # Prerequisites
//!
//! 1. Install ROS2 (Humble, Iron, or Jazzy).
//! 2. Start turtlesim:
//!    ```bash
//!    ros2 run turtlesim turtlesim_node
//!    ```
//! 3. Run this example:
//!    ```bash
//!    cargo run --example ros2_turtlesim
//!    ```
//!
//! The turtle should move in a circle.

use cyclonedds::{DataWriter, DdsEntity, DdsTypeDerive, DomainParticipant, Publisher, Topic};

#[derive(DdsTypeDerive, Clone, Debug)]
#[allow(dead_code)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let participant = DomainParticipant::new(0)?;
    let publisher = Publisher::new(participant.entity())?;

    // ROS2 topic names use a leading slash and are mapped directly to DDS topics.
    let topic = Topic::<Twist>::new(participant.entity(), "/turtle1/cmd_vel")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;

    println!("Connected to ROS2 turtlesim. Publishing Twist messages...");
    println!("Make sure turtlesim is running: ros2 run turtlesim turtlesim_node");

    let twist = Twist {
        linear: Vector3 {
            x: 2.0,
            y: 0.0,
            z: 0.0,
        },
        angular: Vector3 {
            x: 0.0,
            y: 0.0,
            z: 1.8,
        },
    };

    for i in 0..100 {
        writer.write(&twist)?;
        println!(
            "Published Twist #{} — linear.x={}, angular.z={}",
            i, twist.linear.x, twist.angular.z
        );
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    println!("Done!");
    Ok(())
}
