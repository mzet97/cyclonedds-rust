use cyclonedds::{
    DomainParticipant, Publisher, QosBuilder, SecurityConfig, Topic, DataWriter,
};
use cyclonedds_derive::DdsTypeDerive;

#[derive(DdsTypeDerive, Clone)]
struct HelloWorld {
    id: i32,
    message: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let security = SecurityConfig::new()
        .identity_ca("examples/security/identity_ca_cert.pem")
        .identity_certificate("examples/security/participant_cert.pem")
        .identity_private_key("examples/security/participant_key.pem")
        .governance("examples/security/governance.xml")
        .permissions("examples/security/permissions.xml")
        .permissions_ca("examples/security/permissions_ca_cert.pem");

    let qos = QosBuilder::new()
        .security(security)
        .build()?;

    let participant = DomainParticipant::with_qos(0, Some(&qos))?;
    let publisher = Publisher::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorld")?;
    let writer = DataWriter::new(publisher.entity(), topic.entity())?;

    println!("Secure publisher started on domain 0");

    for i in 0..100 {
        let msg = HelloWorld {
            id: i,
            message: format!("Hello from secure publisher [{}]", i),
        };
        writer.write(&msg)?;
        println!("Published: id={}, message={}", msg.id, msg.message);
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    Ok(())
}
