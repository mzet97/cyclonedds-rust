use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use cyclonedds::{
    BuiltinEndpointSample, BuiltinParticipantSample, DdsEntity, DdsTypeDerive as DdsType,
    DomainParticipant, DataReader, DataWriter, Publisher, QosBuilder, Reliability,
    Subscriber, WaitSet, STATUS_DATA_AVAILABLE, discover_type_from_type_info,
    cdr_to_dynamic_data,
};

// ---------------------------------------------------------------------------
// CLI definition
// ---------------------------------------------------------------------------

#[derive(Parser)]
#[command(name = "cyclonedds", version, about = "CycloneDDS CLI tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List discovered DDS entities (participants, publications, subscriptions)
    Ls {
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
    },
    /// List DDS participants
    Ps {
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
    },
    /// Subscribe to a topic and print received samples
    Subscribe {
        /// Topic name to subscribe to
        topic: String,
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
        /// Maximum number of samples to receive (0 = unlimited)
        #[arg(long, default_value_t = 0)]
        samples: usize,
    },
    /// Run a ping-pong latency performance test
    Perf {
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
        /// Number of samples for the perf test
        #[arg(long, default_value_t = 100)]
        samples: usize,
    },
    /// Discover and print the type schema of a topic
    Typeof {
        /// Topic name to look up
        topic: String,
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
    },
    /// Publish simple string messages to a topic
    Publish {
        /// Topic name to publish to
        topic: String,
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
        /// Message to publish (if omitted, reads from stdin)
        #[arg(short, long)]
        message: Option<String>,
        /// Number of times to publish the message
        #[arg(short, long, default_value_t = 1)]
        count: usize,
        /// Delay between messages in milliseconds
        #[arg(short, long, default_value_t = 0)]
        delay_ms: u64,
    },
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn format_guid(guid: &cyclonedds_rust_sys::dds_guid_t) -> String {
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        guid.v[0], guid.v[1], guid.v[2], guid.v[3],
        guid.v[4], guid.v[5], guid.v[6], guid.v[7],
        guid.v[8], guid.v[9], guid.v[10], guid.v[11],
        guid.v[12], guid.v[13], guid.v[14], guid.v[15],
    )
}

fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}

/// Check a DDS entity handle returned from an FFI call.
fn check_entity(handle: cyclonedds_rust_sys::dds_entity_t) -> cyclonedds::DdsResult<cyclonedds_rust_sys::dds_entity_t> {
    if handle < 0 {
        Err(cyclonedds::DdsError::from(handle))
    } else {
        Ok(handle)
    }
}

// ---------------------------------------------------------------------------
// ls command
// ---------------------------------------------------------------------------

fn cmd_ls(domain_id: u32) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;

    let pub_reader = participant.create_builtin_publication_reader()?;
    let sub_reader = participant.create_builtin_subscription_reader()?;
    let par_reader = participant.create_builtin_participant_reader()?;

    // Wait for discovery
    std::thread::sleep(Duration::from_millis(200));

    let participants: Vec<BuiltinParticipantSample> = par_reader.take().unwrap_or_default();
    let publications: Vec<BuiltinEndpointSample> = pub_reader.take().unwrap_or_default();
    let subscriptions: Vec<BuiltinEndpointSample> = sub_reader.take().unwrap_or_default();

    // Print participants
    println!("=== Participants ({}) ===", participants.len());
    println!("{:<40} {}", "GUID", "Name");
    println!("{}", "-".repeat(60));
    for p in &participants {
        let name = p.participant_name().unwrap_or_else(|| "-".to_string());
        println!("{:<40} {}", format_guid(&p.key()), name);
    }

    // Print publications
    println!();
    println!("=== Publications ({}) ===", publications.len());
    println!("{:<40} {:<30} Type", "GUID", "Topic");
    println!("{}", "-".repeat(100));
    for p in &publications {
        println!(
            "{:<40} {:<30} {}",
            format_guid(&p.key()),
            p.topic_name(),
            p.type_name()
        );
    }

    // Print subscriptions
    println!();
    println!("=== Subscriptions ({}) ===", subscriptions.len());
    println!("{:<40} {:<30} Type", "GUID", "Topic");
    println!("{}", "-".repeat(100));
    for s in &subscriptions {
        println!(
            "{:<40} {:<30} {}",
            format_guid(&s.key()),
            s.topic_name(),
            s.type_name()
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// ps command
// ---------------------------------------------------------------------------

fn cmd_ps(domain_id: u32) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let reader = participant.create_builtin_participant_reader()?;

    // Wait for discovery
    std::thread::sleep(Duration::from_millis(200));

    let samples: Vec<BuiltinParticipantSample> = reader.take().unwrap_or_default();

    println!("Participants discovered: {}", samples.len());
    println!();
    println!("{:<40} Name", "GUID");
    println!("{}", "-".repeat(60));

    for s in &samples {
        let name = s.participant_name().unwrap_or_else(|| "-".to_string());
        println!("{:<40} {}", format_guid(&s.key()), name);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// subscribe command
// ---------------------------------------------------------------------------

fn cmd_subscribe(
    topic_name: &str,
    domain_id: u32,
    max_samples: usize,
) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let pub_reader = participant.create_builtin_publication_reader()?;

    println!("Searching for writers on topic '{}'...", topic_name);

    // Wait for discovery and find the matching publication
    let endpoint = 'search: {
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(200));
            let pubs: Vec<BuiltinEndpointSample> = pub_reader.take().unwrap_or_default();
            for ep in &pubs {
                if ep.topic_name() == topic_name {
                    break 'search ep.clone();
                }
            }
        }
        println!(
            "No publication found for topic '{}' within timeout.",
            topic_name
        );
        return Ok(());
    };

    let type_name = endpoint.type_name();
    println!("Found publication: topic='{}', type='{}'", topic_name, type_name);

    // Discover the full type (schema + topic descriptor)
    let type_info = endpoint.type_info()?;
    let discovered = discover_type_from_type_info(
        participant.entity(),
        &type_info,
        &type_name,
        5_000_000_000,
    )?;
    let schema = discovered.type_schema;
    let topic_descriptor = &discovered.topic_descriptor;

    // Create the topic and reader
    let topic = topic_descriptor.create_topic(participant.entity(), topic_name)?;
    let subscriber = Subscriber::new(participant.entity())?;
    let qos = QosBuilder::new()
        .reliability(Reliability::Reliable, 10_000_000_000)
        .build()?;

    let reader_entity = unsafe {
        check_entity(cyclonedds_rust_sys::dds_create_reader(
            subscriber.entity(),
            topic.entity(),
            qos.as_ptr(),
            std::ptr::null_mut(),
        ))?
    };

    // Create a waitset to wait for data
    let waitset = WaitSet::new(participant.entity())?;

    // Enable data-available status on reader
    unsafe {
        cyclonedds_rust_sys::dds_set_status_mask(reader_entity, STATUS_DATA_AVAILABLE);
    }
    waitset.attach(reader_entity, 1)?;

    println!("Subscribed. Waiting for samples... (Ctrl+C to stop)");
    let mut received = 0usize;

    // Set up Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::Relaxed);
    });

    while running.load(Ordering::Relaxed) {
        if max_samples > 0 && received >= max_samples {
            break;
        }

        // Wait for data (1 second timeout)
        if waitset.wait(1_000_000_000).is_err() {
            continue;
        }

        // Take CDR samples from the reader
        let max_buf: usize = 256;
        let mut sdbuf: Vec<*mut cyclonedds_rust_sys::ddsi_serdata> =
            vec![std::ptr::null_mut(); max_buf];
        let mut infos: Vec<cyclonedds_rust_sys::dds_sample_info_t> =
            vec![unsafe { std::mem::zeroed() }; max_buf];

        let n = unsafe {
            cyclonedds_rust_sys::dds_takecdr(
                reader_entity,
                sdbuf.as_mut_ptr(),
                max_buf as u32,
                infos.as_mut_ptr() as *mut cyclonedds_rust_sys::dds_sample_info_t,
                0,
            )
        };

        if n <= 0 {
            continue;
        }
        let n = n as usize;

        for i in 0..n {
            let sd = sdbuf[i];
            if sd.is_null() || !infos[i].valid_data {
                if !sd.is_null() {
                    unsafe { cyclonedds_rust_sys::ddsi_serdata_unref(sd) };
                }
                continue;
            }

            // Extract CDR bytes from the serdata
            let size = unsafe { cyclonedds_rust_sys::ddsi_serdata_size(sd) } as usize;
            let mut cdr_bytes = vec![0u8; size];
            unsafe {
                cyclonedds_rust_sys::ddsi_serdata_to_ser(
                    sd,
                    0,
                    size,
                    cdr_bytes.as_mut_ptr() as *mut std::ffi::c_void,
                );
            }

            // Unref the serdata now that we have the bytes
            unsafe { cyclonedds_rust_sys::ddsi_serdata_unref(sd) };

            // Deserialize CDR into DynamicData
            received += 1;
            match cdr_to_dynamic_data(&cdr_bytes, &schema, topic_descriptor) {
                Ok(data) => println!("[{}] {:?}", received, data),
                Err(e) => println!("[{}] <decode error: {:?}>", received, e),
            }
        }
    }

    println!("Received {} samples.", received);

    // Clean up reader entity
    unsafe {
        cyclonedds_rust_sys::dds_delete(reader_entity);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// perf command (ping-pong latency test)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Debug, Clone, DdsType)]
struct PerfSample {
    seq_num: u64,
    timestamp_ns: u64,
}

fn cmd_perf(domain_id: u32, num_samples: usize) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let publisher = Publisher::new(participant.entity())?;
    let subscriber = Subscriber::new(participant.entity())?;

    let unique_id = std::process::id();
    let ping_topic_name = format!("cyclonedds_cli_perf_ping_{}", unique_id);
    let pong_topic_name = format!("cyclonedds_cli_perf_pong_{}", unique_id);

    let ping_topic = participant.create_topic::<PerfSample>(&ping_topic_name)?;
    let pong_topic = participant.create_topic::<PerfSample>(&pong_topic_name)?;

    let qos = QosBuilder::new().build()?;

    let ping_writer = DataWriter::with_qos(publisher.entity(), ping_topic.entity(), Some(&qos))?;
    let pong_writer = DataWriter::with_qos(publisher.entity(), pong_topic.entity(), Some(&qos))?;
    let ping_reader = DataReader::with_qos(subscriber.entity(), ping_topic.entity(), Some(&qos))?;
    let pong_reader = DataReader::with_qos(subscriber.entity(), pong_topic.entity(), Some(&qos))?;

    // Wait for matching
    println!("Waiting for matching participant...");
    println!(
        "Run another instance with the same domain ID to perform the ping-pong test."
    );
    println!();

    // Enable data available status
    unsafe {
        cyclonedds_rust_sys::dds_set_status_mask(ping_reader.entity(), STATUS_DATA_AVAILABLE);
        cyclonedds_rust_sys::dds_set_status_mask(pong_reader.entity(), STATUS_DATA_AVAILABLE);
    }

    let ping_waitset = WaitSet::new(participant.entity())?;
    let pong_waitset = WaitSet::new(participant.entity())?;

    ping_waitset.attach(ping_reader.entity(), 1)?;
    pong_waitset.attach(pong_reader.entity(), 1)?;

    // Detect role: if a matched publication already exists on the pong topic,
    // we are the ponger; otherwise we are the pinger.
    std::thread::sleep(Duration::from_millis(500));

    let is_pinger = pong_reader
        .matched_publications()
        .map(|pubs| pubs.is_empty())
        .unwrap_or(true);

    if is_pinger {
        println!("Mode: PINGER (sending pings, waiting for pongs)");
        run_pinger(&ping_writer, &pong_waitset, &pong_reader, num_samples)?;
    } else {
        println!("Mode: PONGER (reflecting pings)");
        run_ponger(&ping_waitset, &ping_reader, &pong_writer, num_samples)?;
    }

    Ok(())
}

fn run_pinger(
    writer: &DataWriter<PerfSample>,
    waitset: &WaitSet,
    reader: &DataReader<PerfSample>,
    num_samples: usize,
) -> cyclonedds::DdsResult<()> {
    let mut latencies_ns: Vec<u64> = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let sample = PerfSample {
            seq_num: i as u64,
            timestamp_ns: now_ns(),
        };

        writer.write(&sample)?;

        // Wait for pong (5 second timeout)
        if waitset.wait(5_000_000_000).is_err() {
            println!("Timeout waiting for pong on sample {}", i);
            continue;
        }

        let pong_samples: Vec<PerfSample> = reader.take().unwrap_or_default();
        let recv_ts = now_ns();

        if let Some(pong) = pong_samples.first() {
            let latency = recv_ts.saturating_sub(pong.timestamp_ns);
            latencies_ns.push(latency);
            println!(
                "Sample {:>4}/{}: {:.3} ms",
                i + 1,
                num_samples,
                latency as f64 / 1_000_000.0
            );
        }
    }

    if !latencies_ns.is_empty() {
        latencies_ns.sort();
        let min = latencies_ns[0];
        let max = latencies_ns[latencies_ns.len() - 1];
        let avg: u64 = latencies_ns.iter().sum::<u64>() / latencies_ns.len() as u64;
        let p50 = latencies_ns[latencies_ns.len() / 2];

        println!();
        println!("=== Latency Results ({} samples) ===", latencies_ns.len());
        println!("  Min : {:.3} ms", min as f64 / 1_000_000.0);
        println!("  Avg : {:.3} ms", avg as f64 / 1_000_000.0);
        println!("  P50 : {:.3} ms", p50 as f64 / 1_000_000.0);
        println!("  Max : {:.3} ms", max as f64 / 1_000_000.0);
    }

    Ok(())
}

fn run_ponger(
    waitset: &WaitSet,
    reader: &DataReader<PerfSample>,
    writer: &DataWriter<PerfSample>,
    num_samples: usize,
) -> cyclonedds::DdsResult<()> {
    let mut reflected = 0usize;

    while reflected < num_samples {
        if waitset.wait(5_000_000_000).is_err() {
            println!("Timeout waiting for ping.");
            break;
        }

        let samples: Vec<PerfSample> = reader.take().unwrap_or_default();
        for ping in &samples {
            // Reflect the pong preserving the original timestamp
            let pong = PerfSample {
                seq_num: ping.seq_num,
                timestamp_ns: ping.timestamp_ns,
            };
            writer.write(&pong)?;
            reflected += 1;
        }
    }

    println!("Ponger reflected {} samples.", reflected);
    Ok(())
}

// ---------------------------------------------------------------------------
// typeof command
// ---------------------------------------------------------------------------

fn cmd_typeof(topic_name: &str, domain_id: u32) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let pub_reader = participant.create_builtin_publication_reader()?;

    println!("Searching for writers on topic '{}'...", topic_name);

    let endpoint = 'search: {
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(200));
            let pubs: Vec<BuiltinEndpointSample> = pub_reader.take().unwrap_or_default();
            for ep in &pubs {
                if ep.topic_name() == topic_name {
                    break 'search ep.clone();
                }
            }
        }
        println!(
            "No publication found for topic '{}' within timeout.",
            topic_name
        );
        return Ok(());
    };

    let type_name = endpoint.type_name();
    println!("Found publication: topic='{}', type='{}'", topic_name, type_name);

    let type_info = endpoint.type_info()?;
    let discovered = discover_type_from_type_info(
        participant.entity(),
        &type_info,
        &type_name,
        5_000_000_000,
    )?;

    println!("\n=== Type Information ===");
    println!("Type name: {}", discovered.type_name);
    println!("Type info present: {}", type_info.is_present());
    println!("\n=== Type Schema (Debug) ===");
    println!("{:?}", discovered.type_schema);

    Ok(())
}

// ---------------------------------------------------------------------------
// publish command
// ---------------------------------------------------------------------------

fn cmd_publish(
    topic_name: &str,
    domain_id: u32,
    message: Option<String>,
    count: usize,
    delay_ms: u64,
) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let publisher = Publisher::new(participant.entity())?;
    let pub_reader = participant.create_builtin_publication_reader()?;

    println!("Searching for writers on topic '{}'...", topic_name);

    let endpoint = 'search: {
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(200));
            let pubs: Vec<BuiltinEndpointSample> = pub_reader.take().unwrap_or_default();
            for ep in &pubs {
                if ep.topic_name() == topic_name {
                    break 'search ep.clone();
                }
            }
        }
        println!(
            "No publication found for topic '{}' within timeout. Cannot determine type to publish.",
            topic_name
        );
        return Ok(());
    };

    let type_name = endpoint.type_name();
    println!("Found publication: topic='{}', type='{}'", topic_name, type_name);

    let type_info = endpoint.type_info()?;
    let discovered = discover_type_from_type_info(
        participant.entity(),
        &type_info,
        &type_name,
        5_000_000_000,
    )?;

    let topic = discovered.create_topic(participant.entity(), topic_name)?;
    let qos = QosBuilder::new()
        .reliability(Reliability::Reliable, 10_000_000_000)
        .build()?;

    let writer_entity = unsafe {
        check_entity(cyclonedds_rust_sys::dds_create_writer(
            publisher.entity(),
            topic.entity(),
            qos.as_ptr(),
            std::ptr::null_mut(),
        ))?
    };

    let payload = message.unwrap_or_else(|| "Hello from cyclonedds-cli".to_string());
    println!("Publishing {} message(s) to '{}'...", count, topic_name);

    for i in 0..count {
        let msg = format!("{} [{}]", payload, i);
        let c_msg = std::ffi::CString::new(msg).unwrap();
        unsafe {
            let _ = cyclonedds_rust_sys::dds_write(writer_entity, c_msg.as_ptr() as *const std::ffi::c_void);
        }
        println!("Published: {}", c_msg.to_string_lossy());
        if delay_ms > 0 && i + 1 < count {
            std::thread::sleep(Duration::from_millis(delay_ms));
        }
    }

    unsafe {
        cyclonedds_rust_sys::dds_delete(writer_entity);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Ls { domain_id } => cmd_ls(domain_id),
        Commands::Ps { domain_id } => cmd_ps(domain_id),
        Commands::Subscribe {
            topic,
            domain_id,
            samples,
        } => cmd_subscribe(&topic, domain_id, samples),
        Commands::Perf { domain_id, samples } => cmd_perf(domain_id, samples),
        Commands::Typeof { topic, domain_id } => cmd_typeof(&topic, domain_id),
        Commands::Publish {
            topic,
            domain_id,
            message,
            count,
            delay_ms,
        } => cmd_publish(&topic, domain_id, message, count, delay_ms),
    };

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
