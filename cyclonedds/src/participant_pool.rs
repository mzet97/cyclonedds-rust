use crate::{error::DdsResult, DomainParticipant, Qos};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A pool of DDS domain participants with service discovery and health checks.
///
/// `ParticipantPool` manages multiple `DomainParticipant` instances across
/// different DDS domains, provides topic discovery, and automatically removes
/// stale participants.
///
/// # Example
/// ```no_run
/// use cyclonedds::ParticipantPool;
/// use std::time::Duration;
///
/// let mut pool = ParticipantPool::new();
/// pool.join_domain(0, None).unwrap();
/// pool.join_domain(1, None).unwrap();
///
/// let topics = pool.discover_topics(0, Duration::from_secs(1)).unwrap();
/// for t in &topics {
///     println!("Discovered topic: {} (type: {})", t.name, t.type_name);
/// }
/// ```
pub struct ParticipantPool {
    participants: Arc<Mutex<HashMap<u32, PoolEntry>>>,
}

struct PoolEntry {
    participant: DomainParticipant,
    last_heartbeat: Instant,
}

/// Information about a discovered topic.
#[derive(Debug, Clone)]
pub struct DiscoveredTopic {
    pub name: String,
    pub type_name: String,
}

/// Information about a discovered participant.
#[derive(Debug, Clone)]
pub struct DiscoveredParticipant {
    pub domain_id: u32,
    pub name: Option<String>,
}

impl ParticipantPool {
    pub fn new() -> Self {
        ParticipantPool {
            participants: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create and add a participant for the given domain.
    pub fn join_domain(&self, domain_id: u32, qos: Option<&Qos>) -> DdsResult<()> {
        let participant = if let Some(q) = qos {
            DomainParticipant::with_qos(domain_id, Some(q))?
        } else {
            DomainParticipant::new(domain_id)?
        };

        let mut p = self.participants.lock().unwrap();
        p.insert(
            domain_id,
            PoolEntry {
                participant,
                last_heartbeat: Instant::now(),
            },
        );
        Ok(())
    }

    /// Remove a participant from the pool.
    pub fn leave_domain(&self, domain_id: u32) -> DdsResult<()> {
        let mut p = self.participants.lock().unwrap();
        p.remove(&domain_id);
        Ok(())
    }

    /// Returns the participant for a domain, if any.
    pub fn get(&self, domain_id: u32) -> Option<DomainParticipant> {
        self.participant_for_domain(domain_id).ok()
    }

    fn participant_for_domain(&self, domain_id: u32) -> DdsResult<DomainParticipant> {
        self.participants
            .lock()
            .unwrap()
            .get(&domain_id)
            .map(|entry| entry.participant.clone())
            .ok_or_else(|| crate::DdsError::BadParameter("domain not in pool".into()))
    }

    /// Discover topics in a domain using DDS builtin topics.
    pub fn discover_topics(
        &self,
        domain_id: u32,
        timeout: Duration,
    ) -> DdsResult<Vec<DiscoveredTopic>> {
        let participant = self.participant_for_domain(domain_id)?;
        let reader = participant.create_builtin_topic_reader()?;

        let deadline = Instant::now() + timeout;
        let mut discovered = Vec::new();

        while Instant::now() < deadline {
            for sample in reader.take()? {
                discovered.push(DiscoveredTopic {
                    name: sample.topic_name(),
                    type_name: sample.type_name_value(),
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        Ok(discovered)
    }

    /// Discover participants in a domain using DDS builtin topics.
    pub fn discover_participants(
        &self,
        domain_id: u32,
        timeout: Duration,
    ) -> DdsResult<Vec<DiscoveredParticipant>> {
        let participant = self.participant_for_domain(domain_id)?;
        let reader = participant.create_builtin_participant_reader()?;

        let deadline = Instant::now() + timeout;
        let mut discovered = Vec::new();

        while Instant::now() < deadline {
            for sample in reader.take()? {
                discovered.push(DiscoveredParticipant {
                    domain_id,
                    name: sample.participant_name(),
                });
            }
            std::thread::sleep(Duration::from_millis(50));
        }

        Ok(discovered)
    }

    /// Update heartbeat timestamp for a domain.
    pub fn heartbeat(&self, domain_id: u32) -> DdsResult<()> {
        let mut p = self.participants.lock().unwrap();
        if let Some(entry) = p.get_mut(&domain_id) {
            entry.last_heartbeat = Instant::now();
            Ok(())
        } else {
            Err(crate::DdsError::BadParameter("domain not in pool".into()))
        }
    }

    /// Remove participants that have not sent a heartbeat within the threshold.
    pub fn purge_stale(&self, threshold: Duration) -> Vec<u32> {
        let mut p = self.participants.lock().unwrap();
        let now = Instant::now();
        let stale: Vec<u32> = p
            .iter()
            .filter(|(_, entry)| now.duration_since(entry.last_heartbeat) > threshold)
            .map(|(&id, _)| id)
            .collect();
        for id in &stale {
            p.remove(id);
        }
        stale
    }

    /// List all domains currently in the pool.
    pub fn domains(&self) -> Vec<u32> {
        let p = self.participants.lock().unwrap();
        p.keys().copied().collect()
    }
}

impl Default for ParticipantPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::DdsEntity;

    #[test]
    fn participant_lookup_reuses_stored_entity_and_releases_pool_lock() {
        // Given: a pool with one participant.
        let pool = ParticipantPool::new();
        pool.join_domain(172, None).unwrap();
        let stored = pool
            .participants
            .lock()
            .unwrap()
            .get(&172)
            .unwrap()
            .participant
            .entity();

        // When: lookup clones the participant out of the map.
        let fetched = pool.participant_for_domain(172).unwrap();

        // Then: the DDS entity is identical and the map lock is no longer held.
        assert_eq!(fetched.entity(), stored);
        assert!(pool.participants.try_lock().is_ok());
    }

    #[test]
    fn discovery_wait_does_not_block_other_pool_operations() {
        let pool = Arc::new(ParticipantPool::new());
        pool.join_domain(173, None).unwrap();

        let discovery_pool = Arc::clone(&pool);
        let discovery = std::thread::spawn(move || {
            discovery_pool
                .discover_topics(173, Duration::from_millis(500))
                .unwrap()
        });

        std::thread::sleep(Duration::from_millis(100));
        let started = Instant::now();
        pool.heartbeat(173).unwrap();
        assert!(
            started.elapsed() < Duration::from_millis(100),
            "heartbeat waited for the discovery loop to release the pool lock"
        );

        discovery.join().unwrap();
    }
}
