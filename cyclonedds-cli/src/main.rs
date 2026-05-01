use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use cyclonedds::{
    BuiltinEndpointSample, BuiltinParticipantSample, DdsEntity, DdsTypeDerive as DdsType,
    DomainParticipant, DataReader, DataWriter, Publisher, QosBuilder, Reliability,
    Subscriber, WaitSet, STATUS_DATA_AVAILABLE,     discover_type_from_type_info,
    dynamic_data_to_cdr, cdr_to_dynamic_data, DynamicData,
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
        /// Output samples as JSON
        #[arg(long)]
        json: bool,
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
    /// Publish messages to a topic (string or JSON)
    Publish {
        /// Topic name to publish to
        topic: String,
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
        /// Message to publish (if omitted, reads from stdin)
        #[arg(short, long)]
        message: Option<String>,
        /// JSON payload to publish as a structured message
        #[arg(long)]
        json: Option<String>,
        /// Number of times to publish the message
        #[arg(short, long, default_value_t = 1)]
        count: usize,
        /// Delay between messages in milliseconds
        #[arg(short, long, default_value_t = 0)]
        delay_ms: u64,
        /// Publish rate in Hz (overrides count and delay_ms)
        #[arg(long)]
        rate: Option<u64>,
    },
    /// Discover and list all types available on a topic
    Discover {
        /// Topic name to discover
        topic: String,
        /// DDS domain ID
        #[arg(long, default_value_t = 0)]
        domain_id: u32,
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
    output_json: bool,
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
                Ok(data) => {
                    if output_json {
                        let json_val = dynamic_value_to_json(data.value());
                        println!("{}", serde_json::to_string_pretty(&json_val).unwrap_or_else(|_| format!("{:?}", data)));
                    } else {
                        println!("[{}] {:?}", received, data);
                    }
                }
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

fn format_type_schema(schema: &cyclonedds::DynamicTypeSchema) -> String {
    use cyclonedds::{DynamicPrimitiveKind, DynamicTypeSchema};

    match schema {
        DynamicTypeSchema::Primitive(kind) => match kind {
            DynamicPrimitiveKind::Boolean => "boolean".into(),
            DynamicPrimitiveKind::Byte => "octet".into(),
            DynamicPrimitiveKind::Int8 => "int8".into(),
            DynamicPrimitiveKind::UInt8 => "uint8".into(),
            DynamicPrimitiveKind::Int16 => "short".into(),
            DynamicPrimitiveKind::UInt16 => "unsigned short".into(),
            DynamicPrimitiveKind::Int32 => "long".into(),
            DynamicPrimitiveKind::UInt32 => "unsigned long".into(),
            DynamicPrimitiveKind::Int64 => "long long".into(),
            DynamicPrimitiveKind::UInt64 => "unsigned long long".into(),
            DynamicPrimitiveKind::Float32 => "float".into(),
            DynamicPrimitiveKind::Float64 => "double".into(),
            DynamicPrimitiveKind::Char8 => "char".into(),
            DynamicPrimitiveKind::Char16 => "wchar".into(),
        },
        DynamicTypeSchema::String { bound: None } => "string".into(),
        DynamicTypeSchema::String { bound: Some(b) } => format!("string<{}>", b),
        DynamicTypeSchema::Sequence { element, bound: None, .. } => {
            format!("sequence<{}>", format_type_schema(element))
        }
        DynamicTypeSchema::Sequence { element, bound: Some(b), .. } => {
            format!("sequence<{}, {}>", format_type_schema(element), b)
        }
        DynamicTypeSchema::Array { element, bounds, .. } => {
            let dims: Vec<String> = bounds.iter().map(|b| b.to_string()).collect();
            format!("{}[{}]", format_type_schema(element), dims.join("]["))
        }
        DynamicTypeSchema::Struct { name, .. } => name.clone(),
        DynamicTypeSchema::Enum { name, .. } => name.clone(),
        DynamicTypeSchema::Union { name, .. } => name.clone(),
        DynamicTypeSchema::Bitmask { name, .. } => name.clone(),
        DynamicTypeSchema::Map { key, value, bound: None, .. } => {
            format!("map<{}, {}>", format_type_schema(key), format_type_schema(value))
        }
        DynamicTypeSchema::Map { key, value, bound: Some(b), .. } => {
            format!("map<{}, {}, {}>", format_type_schema(key), format_type_schema(value), b)
        }
        DynamicTypeSchema::Alias { target, .. } => format_type_schema(target),
    }
}

fn print_type_schema(schema: &cyclonedds::DynamicTypeSchema, indent: usize) {
    use cyclonedds::{DynamicTypeExtensibility, DynamicTypeSchema};
    let prefix = "  ".repeat(indent);

    match schema {
        DynamicTypeSchema::Struct { name, fields, extensibility, autoid, .. } => {
            let ext = match extensibility {
                Some(DynamicTypeExtensibility::Final) => " @final",
                Some(DynamicTypeExtensibility::Appendable) => " @appendable",
                Some(DynamicTypeExtensibility::Mutable) => " @mutable",
                None => "",
            };
            let autoid_str = match autoid {
                Some(cyclonedds::DynamicTypeAutoId::Sequential) => " @autoid(Sequential)",
                Some(cyclonedds::DynamicTypeAutoId::Hash) => " @autoid(Hash)",
                None => "",
            };
            println!("{}{}struct {}{} {{", prefix, prefix, name, format!("{}{}", ext, autoid_str));
            for field in fields {
                let mut attrs = Vec::new();
                if field.key {
                    attrs.push("@key");
                }
                if field.optional {
                    attrs.push("@optional");
                }
                if field.must_understand {
                    attrs.push("@must_understand");
                }
                if field.external {
                    attrs.push("@external");
                }
                let attr_str = if attrs.is_empty() {
                    String::new()
                } else {
                    format!(" {}", attrs.join(" "))
                };
                let type_str = format_type_schema(&field.value);
                println!("{}  {}{}{};", prefix, type_str, attr_str, field.name);
            }
            println!("{}}};", prefix);
        }
        DynamicTypeSchema::Enum { name, literals, .. } => {
            println!("{}enum {} {{", prefix, name);
            for lit in literals {
                let default = if lit.default { "  /* default */" } else { "" };
                println!("{}  {} = {},{}", prefix, lit.name, lit.value, default);
            }
            println!("{}}};", prefix);
        }
        DynamicTypeSchema::Union { name, discriminator, cases, .. } => {
            let disc = format_type_schema(discriminator);
            println!("{}union {} switch ({}) {{", prefix, name, disc);
            for case in cases {
                let labels: Vec<String> = case.labels.iter().map(|l| l.to_string()).collect();
                let default = if case.default { " /* default */" } else { "" };
                let type_str = format_type_schema(&case.value);
                println!(
                    "{}  case {}: {}{} {};",
                    prefix,
                    labels.join(", case "),
                    default,
                    type_str,
                    case.name
                );
            }
            println!("{}}};", prefix);
        }
        DynamicTypeSchema::Bitmask { name, fields, .. } => {
            println!("{}bitmask {} {{", prefix, name);
            for field in fields {
                println!("{}  {} @position({});", prefix, field.name, field.position);
            }
            println!("{}}};", prefix);
        }
        _ => {
            println!("{}{}", prefix, format_type_schema(schema));
        }
    }
}

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
    println!("Topic descriptor size: {} bytes", discovered.topic_descriptor.size());
    println!("Topic descriptor align: {} bytes", discovered.topic_descriptor.align());

    println!("\n=== IDL-like Representation ===");
    print_type_schema(&discovered.type_schema, 0);

    Ok(())
}

// ---------------------------------------------------------------------------
// discover command
// ---------------------------------------------------------------------------

fn cmd_discover(topic_name: &str, domain_id: u32) -> cyclonedds::DdsResult<()> {
    let participant = DomainParticipant::new(domain_id)?;
    let pub_reader = participant.create_builtin_publication_reader()?;

    println!("Discovering types on topic '{}'...", topic_name);

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
    println!("Topic: {}", topic_name);
    println!("Type: {}", type_name);

    let type_info = endpoint.type_info()?;
    let discovered = discover_type_from_type_info(
        participant.entity(),
        &type_info,
        &type_name,
        5_000_000_000,
    )?;

    println!("Type schema discovered successfully.");
    println!("  Size: {} bytes", discovered.topic_descriptor.size());
    println!("  Align: {} bytes", discovered.topic_descriptor.align());
    println!("  Key count: {}", discovered.topic_descriptor.key_count());

    Ok(())
}

// ---------------------------------------------------------------------------
// publish command
// ---------------------------------------------------------------------------

fn json_to_dynamic_value(
    json: &serde_json::Value,
    schema: &cyclonedds::DynamicTypeSchema,
) -> cyclonedds::DdsResult<cyclonedds::DynamicValue> {
    use cyclonedds::{
        DynamicPrimitiveKind, DynamicTypeSchema, DynamicValue,
    };

    match schema {
        DynamicTypeSchema::Primitive(kind) => match kind {
            DynamicPrimitiveKind::Boolean => json
                .as_bool()
                .map(DynamicValue::Bool)
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected bool".into())),
            DynamicPrimitiveKind::Int32 => json
                .as_i64()
                .map(|v| DynamicValue::Int32(v as i32))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected int32".into())),
            DynamicPrimitiveKind::UInt32 => json
                .as_u64()
                .map(|v| DynamicValue::UInt32(v as u32))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected uint32".into())),
            DynamicPrimitiveKind::Int64 => json
                .as_i64()
                .map(DynamicValue::Int64)
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected int64".into())),
            DynamicPrimitiveKind::UInt64 => json
                .as_u64()
                .map(DynamicValue::UInt64)
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected uint64".into())),
            DynamicPrimitiveKind::Float32 => json
                .as_f64()
                .map(|v| DynamicValue::Float32(v as f32))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected float32".into())),
            DynamicPrimitiveKind::Float64 => json
                .as_f64()
                .map(DynamicValue::Float64)
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected float64".into())),
            DynamicPrimitiveKind::Byte => json
                .as_u64()
                .map(|v| DynamicValue::Byte(v as u8))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected byte".into())),
            DynamicPrimitiveKind::Int8 => json
                .as_i64()
                .map(|v| DynamicValue::Int8(v as i8))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected int8".into())),
            DynamicPrimitiveKind::UInt8 => json
                .as_u64()
                .map(|v| DynamicValue::UInt8(v as u8))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected uint8".into())),
            DynamicPrimitiveKind::Int16 => json
                .as_i64()
                .map(|v| DynamicValue::Int16(v as i16))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected int16".into())),
            DynamicPrimitiveKind::UInt16 => json
                .as_u64()
                .map(|v| DynamicValue::UInt16(v as u16))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected uint16".into())),
            DynamicPrimitiveKind::Char8 => json
                .as_str()
                .and_then(|s| s.chars().next())
                .map(|c| DynamicValue::Char8(c as u8))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected char".into())),
            DynamicPrimitiveKind::Char16 => json
                .as_str()
                .and_then(|s| s.chars().next())
                .map(|c| DynamicValue::Char16(c as u16))
                .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected char16".into())),
        },
        DynamicTypeSchema::String { .. } => json
            .as_str()
            .map(|s| DynamicValue::String(s.to_string()))
            .ok_or_else(|| cyclonedds::DdsError::BadParameter("expected string".into())),
        DynamicTypeSchema::Struct { fields, .. } => {
            let obj = json.as_object().ok_or_else(|| {
                cyclonedds::DdsError::BadParameter("expected object for struct".into())
            })?;
            let mut map = std::collections::BTreeMap::new();
            for field in fields {
                if let Some(val) = obj.get(&field.name) {
                    map.insert(field.name.clone(), json_to_dynamic_value(val, &field.value)?);
                } else if !field.optional {
                    return Err(cyclonedds::DdsError::BadParameter(format!(
                        "missing required field '{}'",
                        field.name
                    )));
                }
            }
            Ok(DynamicValue::Struct(map))
        }
        DynamicTypeSchema::Sequence { element, .. } => {
            let arr = json.as_array().ok_or_else(|| {
                cyclonedds::DdsError::BadParameter("expected array for sequence".into())
            })?;
            let items: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| json_to_dynamic_value(v, element))
                .collect();
            Ok(DynamicValue::Sequence(items?))
        }
        DynamicTypeSchema::Array { element, bounds, .. } => {
            let arr = json.as_array().ok_or_else(|| {
                cyclonedds::DdsError::BadParameter("expected array".into())
            })?;
            let expected_len: usize = bounds.iter().map(|b| *b as usize).product();
            if arr.len() != expected_len {
                return Err(cyclonedds::DdsError::BadParameter(format!(
                    "array length mismatch: expected {}, got {}",
                    expected_len,
                    arr.len()
                )));
            }
            let items: Result<Vec<_>, _> = arr
                .iter()
                .map(|v| json_to_dynamic_value(v, element))
                .collect();
            Ok(DynamicValue::Array(items?))
        }
        DynamicTypeSchema::Enum { literals, .. } => {
            let name = json.as_str().ok_or_else(|| {
                cyclonedds::DdsError::BadParameter("expected string for enum".into())
            })?;
            let val = literals
                .iter()
                .find(|l| l.name == name)
                .map(|l| l.value)
                .ok_or_else(|| {
                    cyclonedds::DdsError::BadParameter(format!("unknown enum literal '{}'", name))
                })?;
            Ok(DynamicValue::Enum { value: val })
        }
        _ => Err(cyclonedds::DdsError::BadParameter(
            "unsupported type for JSON conversion".into(),
        )),
    }
}

fn dynamic_value_to_json(value: &cyclonedds::DynamicValue) -> serde_json::Value {
    use cyclonedds::DynamicValue;
    match value {
        DynamicValue::Bool(b) => serde_json::Value::Bool(*b),
        DynamicValue::Byte(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Int8(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::UInt8(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Int16(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::UInt16(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Int32(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::UInt32(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Int64(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::UInt64(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Float32(v) => serde_json::Value::Number(serde_json::Number::from_f64(*v as f64).unwrap_or(0.into())),
        DynamicValue::Float64(v) => serde_json::Value::Number(serde_json::Number::from_f64(*v).unwrap_or(0.into())),
        DynamicValue::Char8(v) => serde_json::Value::String((*v as char).to_string()),
        DynamicValue::Char16(v) => serde_json::Value::String(std::char::from_u32(*v as u32).unwrap_or('?').to_string()),
        DynamicValue::String(s) => serde_json::Value::String(s.clone()),
        DynamicValue::Sequence(items) | DynamicValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(dynamic_value_to_json).collect())
        }
        DynamicValue::Struct(fields) => {
            let mut map = serde_json::Map::new();
            for (k, v) in fields {
                map.insert(k.clone(), dynamic_value_to_json(v));
            }
            serde_json::Value::Object(map)
        }
        DynamicValue::Enum { value } => serde_json::Value::Number((*value).into()),
        DynamicValue::Bitmask(v) => serde_json::Value::Number((*v).into()),
        DynamicValue::Union { discriminator, field, value } => {
            let mut map = serde_json::Map::new();
            map.insert("discriminator".to_string(), serde_json::Value::Number((*discriminator).into()));
            map.insert("field".to_string(), serde_json::Value::String(field.clone()));
            map.insert("value".to_string(), dynamic_value_to_json(value));
            serde_json::Value::Object(map)
        }
        DynamicValue::Map(entries) => {
            // Represent as array of {key, value} objects since JSON keys are strings
            serde_json::Value::Array(entries.iter().map(|(k, v)| {
                let mut obj = serde_json::Map::new();
                obj.insert("key".to_string(), dynamic_value_to_json(k));
                obj.insert("value".to_string(), dynamic_value_to_json(v));
                serde_json::Value::Object(obj)
            }).collect())
        }
        DynamicValue::Null => serde_json::Value::Null,
    }
}

fn cmd_publish(
    topic_name: &str,
    domain_id: u32,
    message: Option<String>,
    json: Option<String>,
    count: usize,
    delay_ms: u64,
    rate: Option<u64>,
) -> cyclonedds::DdsResult<()> {
    let (count, delay_ms) = if let Some(hz) = rate {
        (0, 1000 / hz) // 0 = unlimited count
    } else {
        (count, delay_ms)
    };
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

    if let Some(json_str) = json {
        let json_value: serde_json::Value = serde_json::from_str(&json_str).map_err(|e| {
            cyclonedds::DdsError::BadParameter(format!("invalid JSON: {}", e))
        })?;
        let dyn_value = json_to_dynamic_value(&json_value, &discovered.type_schema)?;
        let data = DynamicData::from_value(&discovered.type_schema, dyn_value)?;
        let cdr = dynamic_data_to_cdr(&data, &discovered.topic_descriptor)?;

        let unlimited = count == 0;
        let prefix = if unlimited { String::new() } else { format!("{} ", count) };
        println!("Publishing {}JSON message(s) to '{}'...", prefix, topic_name);
        let mut i = 0;
        while unlimited || i < count {
            unsafe {
                let _ = cyclonedds_rust_sys::dds_write(
                    writer_entity,
                    cdr.as_ptr() as *const std::ffi::c_void,
                );
            }
            println!("Published JSON message [{}]", i);
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            i += 1;
        }
    } else {
        let payload = message.unwrap_or_else(|| "Hello from cyclonedds-cli".to_string());
        let unlimited = count == 0;
        let prefix = if unlimited { String::new() } else { format!("{} ", count) };
        println!("Publishing {}string message(s) to '{}'...", prefix, topic_name);
        let mut i = 0;
        while unlimited || i < count {
            let msg = format!("{} [{}]", payload, i);
            let c_msg = std::ffi::CString::new(msg).unwrap();
            unsafe {
                let _ = cyclonedds_rust_sys::dds_write(
                    writer_entity,
                    c_msg.as_ptr() as *const std::ffi::c_void,
                );
            }
            println!("Published: {}", c_msg.to_string_lossy());
            if delay_ms > 0 {
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            i += 1;
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
            json,
        } => cmd_subscribe(&topic, domain_id, samples, json),
        Commands::Perf { domain_id, samples } => cmd_perf(domain_id, samples),
        Commands::Typeof { topic, domain_id } => cmd_typeof(&topic, domain_id),
        Commands::Publish {
            topic,
            domain_id,
            message,
            json,
            count,
            delay_ms,
            rate,
        } => cmd_publish(&topic, domain_id, message, json, count, delay_ms, rate),
        Commands::Discover { topic, domain_id } => cmd_discover(&topic, domain_id),
    };

    if let Err(e) = result {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}
