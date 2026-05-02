use cyclonedds::*;
use std::time::Duration;

#[derive(DdsTypeDerive, Clone, Debug, PartialEq)]
struct AddRequest {
    correlation_id: u64,
    a: i32,
    b: i32,
}

impl RequestReply for AddRequest {
    fn correlation_id(&self) -> u64 {
        self.correlation_id
    }
    fn set_correlation_id(&mut self, id: u64) {
        self.correlation_id = id;
    }
}

#[derive(DdsTypeDerive, Clone, Debug, PartialEq)]
struct AddReply {
    correlation_id: u64,
    result: i32,
}

impl RequestReply for AddReply {
    fn correlation_id(&self) -> u64 {
        self.correlation_id
    }
    fn set_correlation_id(&mut self, id: u64) {
        self.correlation_id = id;
    }
}

fn main() {
    let participant = DomainParticipant::new(0).unwrap();

    let requester = Requester::<AddRequest, AddReply>::new(
        &participant, "CalcService", None, None,
    )
    .unwrap();

    let replier = Replier::<AddRequest, AddReply>::new(
        &participant, "CalcService", None, None,
    )
    .unwrap();

    // Spawn replier in background
    std::thread::spawn(move || {
        loop {
            if let Some(req) = replier.receive_request(Duration::from_secs(1)).unwrap() {
                let reply = AddReply {
                    correlation_id: req.correlation_id(),
                    result: req.a + req.b,
                };
                replier.send_reply(reply).unwrap();
                println!("Replier: {} + {} = {}", req.a, req.b, req.a + req.b);
            }
        }
    });

    // Give replier time to start
    std::thread::sleep(Duration::from_millis(200));

    // Send requests
    for i in 1..=5 {
        let req = AddRequest {
            correlation_id: 0,
            a: i,
            b: i * 2,
        };
        let rep = requester.request(req, Duration::from_secs(1)).unwrap();
        println!("Requester: got reply = {} (corr_id={})", rep.result, rep.correlation_id());
        assert_eq!(rep.result, i + i * 2);
    }

    println!("Request-Reply example completed successfully!");
}
