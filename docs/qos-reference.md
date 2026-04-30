# QoS Policy Reference

All QoS policies available in cyclonedds-rust, their builder methods, defaults, and applicable entities.

## Policy Reference

| Policy | Builder Method | Default | Entities | Description |
|--------|---------------|---------|----------|-------------|
| Reliability | `.reliability(kind, max_blocking_time)` / `.reliable()` / `.best_effort()` | BestEffort, 0 | Topic, DataWriter, DataReader | Delivery guarantee. `Reliable` retransmits lost samples. `max_blocking_time` in nanoseconds. |
| Durability | `.durability(kind)` / `.transient_local()` / `.volatile()` | Volatile | Topic, DataWriter, DataReader | How long samples persist. `TransientLocal` retains for matched late-joiners. |
| History | `.history(kind)` / `.keep_last(depth)` / `.keep_all()` | KeepLast(1) | Topic, DataWriter, DataReader | How many samples per instance are stored. |
| Deadline | `.deadline(ns)` | Infinite (0) | Topic, DataWriter, DataReader | Expected maximum inter-arrival time. Triggers status if violated. |
| Lifespan | `.lifespan(ns)` | Infinite (0) | Topic, DataWriter | Maximum duration a sample remains valid. |
| Latency Budget | `.latency_budget(ns)` | 0 | Topic, DataWriter, DataReader | Hint for acceptable end-to-end latency. |
| Ownership | `.ownership(kind)` | Shared | Topic, DataWriter, DataReader | `Shared` allows multiple writers; `Exclusive` uses ownership strength. |
| Ownership Strength | `.ownership_strength(value)` | 0 | DataWriter | Priority for exclusive ownership. Higher wins. |
| Liveliness | `.liveliness(kind, lease_ns)` | Automatic, infinite | Topic, DataWriter, DataReader | How liveliness is asserted. `Automatic` uses participant; `ManualByParticipant`/`ManualByTopic` require explicit assertion. |
| Destination Order | `.destination_order(kind)` | ByReceptionTimestamp | Topic, DataWriter, DataReader | How instances are ordered when multiple writers exist. |
| Writer Data Lifecycle | `.writer_data_lifecycle(autodispose)` | true | DataWriter | Whether disposed instances are auto-disposed on unregistration. |
| Transport Priority | `.transport_priority(value)` | 0 | DataWriter | Hint for transport-layer prioritization. |
| Partition | `.partition(name)` | Empty string | Publisher, Subscriber | Logical partition name for scoping communication. |
| Entity Name | `.entity_name(name)` | None | Any | Human-readable entity name for discovery. |
| User Data | `.userdata(data)` | Empty | DomainParticipant, DataWriter, DataReader | Application-defined byte blob attached to the entity. |
| Topic Data | `.topicdata(data)` | Empty | Topic | Application-defined byte blob attached to the topic. |
| Group Data | `.groupdata(data)` | Empty | Publisher, Subscriber | Application-defined byte blob attached to the group. |
| Presentation | `.presentation(scope, coherent, ordered)` | Instance, false, false | Publisher, Subscriber | Controls coherent/ordered access across instances. |
| Durability Service | `.durability_service(history, max_samples, max_inst, max_per_inst)` | N/A | Topic, DataWriter | Configures the durability service for transient/persistent data. |
| Ignore Local | `.ignore_local(kind)` | None | DomainParticipant, DataWriter, DataReader | Controls whether local endpoints are visible. `Participant` or `Process` to hide same-participant/process data. |
| Writer Batching | `.writer_batching(enabled)` | false | DataWriter | Enables batched writing for improved throughput. |
| Reader Data Lifecycle | `.reader_data_lifecycle(autopurge_nowriter_ns, autopurge_disposed_ns)` | Infinite | DataReader | Delays before auto-purging instances with no writers or disposed state. |
| Type Consistency | `.type_consistency(policy)` | AllowTypeCoercion | DataReader | Controls how strictly types must match. `TypeConsistencyPolicy` has sub-options for ignoring bounds, names, etc. |
| Data Representation | `.data_representation(values)` | XCDR1 + XCDR2 | DataWriter, DataReader | Accepted CDR encoding versions. Values: `DataRepresentation::Xcdr1`, `Xcdr2`, `Xml`. |
| Resource Limits | `.resource_limits(max_samples, max_inst, max_per_inst)` | Unlimited | Topic, DataWriter, DataReader | Caps on sample/instance counts in the reader/writer cache. |
| Time-Based Filter | `.time_based_filter(min_separation_ns)` | 0 | DataReader | Minimum interval between delivering samples of the same instance. |
| PSMX | `.psmx_instances(names)` | None | DomainParticipant, Publisher, Subscriber | PSMX (Plug-in Shared Memory Exchange) instance names. |
| Property | `.property(name, value)` | None | Any | Key-value string property. Use `.property_propagate(name, value, true)` to propagate. |
| Binary Property | `.binary_property(name, value)` | None | Any | Key-value binary property. Use `.binary_property_propagate(name, value, true)` to propagate. |

## QosBuilder Patterns

### Basic Reliable Publisher

```rust
let qos = QosBuilder::new()
    .reliable()
    .transient_local()
    .keep_last(10)
    .build()?;
```

### Exclusive Ownership with Deadline

```rust
let qos = QosBuilder::new()
    .ownership(Ownership::Exclusive)
    .ownership_strength(100)
    .deadline(1_000_000_000)  // 1 second
    .build()?;
```

### Content-Filtered Reader with Resource Limits

```rust
let qos = QosBuilder::new()
    .reliable()
    .keep_all()
    .resource_limits(1000, 100, 10)
    .time_based_filter(100_000_000)  // 100ms minimum separation
    .build()?;
```

### Partitioned Communication with Custom Properties

```rust
let qos = QosBuilder::new()
    .reliable()
    .partition("sensor-network")
    .property("transport", "udp")
    .entity_name("my-writer")
    .build()?;
```

### Reading QoS from an Entity

```rust
let reliability = qos.reliability()?;       // Option<(Reliability, i64)>
let history = qos.history()?;               // Option<History>
let deadline = qos.deadline()?;             // Option<i64>
let ownership = qos.ownership()?;           // Option<Ownership>
let partition = qos.partition()?;           // Option<Vec<String>>
let entity_name = qos.entity_name()?;       // Option<String>
```

### QosProvider (XML Profiles)

Load QoS profiles from an XML configuration file:

```rust
use cyclonedds::{QosProvider, QosKind};

let provider = QosProvider::new("file://qos_profiles.xml")?;
let qos = provider.get_qos(QosKind::Writer, "MyProfile")?;
```
