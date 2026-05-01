use cyclonedds::{
    DomainParticipant, QosBuilder, SecurityConfig, Subscriber, Topic, DataReader,
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
    let subscriber = Subscriber::new(participant.entity())?;
    let topic = Topic::<HelloWorld>::new(participant.entity(), "HelloWorld")?;
    let reader = DataReader::new(subscriber.entity(), topic.entity())?;

    println!("Secure subscriber started on domain 0");

    loop {
        let samples = reader.take()?;
        for sample in samples {
            println!("Received: id={}, message={}", sample.id, sample.message);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
