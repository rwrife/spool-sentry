//! Bounded store with integrity-checked, atomic slot writes (DATA-001/003/005).
//!
//! Storage is modelled as fixed-size slots behind a `Storage` trait so the
//! exact failure modes (short writes, corruption, full) are host-testable
//! before any flash driver exists. Each logical object lives in two slots
//! (A/B); a write commits only when one slot carries a valid header,
//! payload length, and FNV-1a checksum. That gives interrupted-write
//! recovery: current committed data always survives a partial write, and
//! silent destruction is impossible.
//!
//! The checksum detects accidental corruption only; it is not a
//! cryptographic authenticity claim (protocol.md).

use heapless::String;

/// Maximum bytes per stored object (bounded by design, DATA-001).
pub const MAX_OBJECT_BYTES: usize = 4096;
/// 96 hourly aggregates ≈ 4 days at hourly cadence; the real retention
/// window is enforced by `Retention` counts, not by this byte cap alone.
pub const AGGREGATE_CAPACITY: usize = 96;

/// Header magic "SSNT".
pub const MAGIC: [u8; 4] = *b"SSNT";
/// Header size in bytes: magic(4) len u16 checksum u32.
pub const HEADER_BYTES: usize = 10;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageError {
    /// Medium-level read/write failure.
    Fault,
    /// No free slot capacity for the requested write.
    Full,
    /// Both slots of an object are corrupt or absent.
    Corrupt,
    /// Payload exceeds `MAX_OBJECT_BYTES`.
    TooLarge,
}

/// Injected storage boundary. The target adapter maps these onto flash
/// regions; host tests use `MemoryStorage`.
pub trait Storage {
    /// Write `bytes` to the object with `key` atomically (two-slot).
    fn write_object(&mut self, key: u8, bytes: &[u8]) -> StorageResult<()>;
    /// Read the most recent valid version of `key`.
    fn read_object(&mut self, key: u8, out: &mut [u8]) -> StorageResult<usize>;
    /// Discard both slots of `key`.
    fn erase_object(&mut self, key: u8) -> StorageResult<()>;
}

/// Stable object keys.
pub mod keys {
    pub const CALIBRATION: u8 = 1;
    pub const SPOOL_METADATA: u8 = 2;
    pub const EVENTS: u8 = 3;
    pub const AGGREGATES: u8 = 4;
    pub const CONFIG: u8 = 5;
}

/// FNV-1a 32-bit integrity checksum (accidental-corruption detection).
pub fn fnv1a(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    let mut index = 0;
    while index < data.len() {
        hash ^= u32::from(data[index]);
        hash = hash.wrapping_mul(0x0100_0193);
        index += 1;
    }
    hash
}

/// Encode an object with its integrity header.
pub fn encode_object(payload: &[u8], out: &mut [u8]) -> StorageResult<usize> {
    if payload.len() > MAX_OBJECT_BYTES {
        return Err(StorageError::TooLarge);
    }
    if out.len() < HEADER_BYTES + payload.len() {
        return Err(StorageError::TooLarge);
    }
    out[0..4].copy_from_slice(&MAGIC);
    out[4..6].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    out[6..10].copy_from_slice(&fnv1a(payload).to_le_bytes());
    out[HEADER_BYTES..HEADER_BYTES + payload.len()].copy_from_slice(payload);
    Ok(HEADER_BYTES + payload.len())
}

/// Decode and verify an object. Returns the payload slice.
pub fn decode_object(encoded: &[u8]) -> StorageResult<&[u8]> {
    if encoded.len() < HEADER_BYTES {
        return Err(StorageError::Corrupt);
    }
    if encoded[0..4] != MAGIC {
        return Err(StorageError::Corrupt);
    }
    let len = u16::from_le_bytes([encoded[4], encoded[5]]) as usize;
    if len > MAX_OBJECT_BYTES || encoded.len() < HEADER_BYTES + len {
        return Err(StorageError::Corrupt);
    }
    let payload = &encoded[HEADER_BYTES..HEADER_BYTES + len];
    let expected = u32::from_le_bytes([encoded[6], encoded[7], encoded[8], encoded[9]]);
    if fnv1a(payload) != expected {
        return Err(StorageError::Corrupt);
    }
    Ok(payload)
}

/// Retention controls (DATA-001): hourly aggregate history is bounded by
/// a configurable day count with an enforced minimum.
#[derive(Clone, Copy, Debug)]
pub struct Retention {
    pub days: u16,
}

/// Minimum retention that must remain selectable (protocol.md).
pub const MIN_RETENTION_DAYS: u16 = 30;
pub const DEFAULT_RETENTION_DAYS: u16 = 90;

impl Default for Retention {
    fn default() -> Self {
        Self {
            days: DEFAULT_RETENTION_DAYS,
        }
    }
}

impl Retention {
    /// Validate a user-requested retention window.
    pub fn set_days(&mut self, days: u16) -> Result<(), Retention> {
        if !(MIN_RETENTION_DAYS..=DEFAULT_RETENTION_DAYS).contains(&days) {
            return Err(*self);
        }
        self.days = days;
        Ok(())
    }

    pub const fn max_aggregates(&self) -> usize {
        self.days as usize * 24
    }
}

/// Append-only event records (desiccant/spool/calibration events).
#[derive(Clone, Debug)]
pub struct EventLog {
    events: heapless::Vec<Event, 64>,
    dropped_by_retention: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    SpoolChanged,
    DesiccantChanged,
    Calibration,
    UserNote,
}

impl EventKind {
    pub const fn wire(self) -> &'static str {
        match self {
            EventKind::SpoolChanged => "spool",
            EventKind::DesiccantChanged => "desiccant",
            EventKind::Calibration => "calibration",
            EventKind::UserNote => "note",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Event {
    pub kind: EventKind,
    /// Monotonic timestamp at creation; wall time is resolved at export.
    pub at_ms: u64,
    /// Caller-provided detail text stored inline (bounded, no heap).
    pub detail: heapless::String<96>,
}

impl Default for EventLog {
    fn default() -> Self {
        Self::new()
    }
}

impl EventLog {
    pub fn new() -> Self {
        Self {
            events: heapless::Vec::new(),
            dropped_by_retention: 0,
        }
    }

    /// Bounded append: oldest events are evicted (explicit retention,
    /// never a silent unbounded structure).
    pub fn push(&mut self, event: Event) {
        if self.events.is_full() {
            self.events.remove(0);
            self.dropped_by_retention += 1;
        }
        let _ = self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.dropped_by_retention = 0;
    }

    pub fn dropped_count(&self) -> u32 {
        self.dropped_by_retention
    }
}

/// Deletion scopes (protocol.md rule 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeleteScope {
    History,
    SpoolMetadata,
    Calibration,
    Network,
    AllUserData,
}

impl DeleteScope {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "history" => Some(DeleteScope::History),
            "spool_metadata" => Some(DeleteScope::SpoolMetadata),
            "calibration" => Some(DeleteScope::Calibration),
            "network" => Some(DeleteScope::Network),
            "all_user_data" => Some(DeleteScope::AllUserData),
            _ => None,
        }
    }

    pub const fn wire(self) -> &'static str {
        match self {
            DeleteScope::History => "history",
            DeleteScope::SpoolMetadata => "spool_metadata",
            DeleteScope::Calibration => "calibration",
            DeleteScope::Network => "network",
            DeleteScope::AllUserData => "all_user_data",
        }
    }
}

/// Host-test storage double with injected faults.
#[derive(Clone, Debug)]
pub struct MemoryStorage {
    slots: [Option<heapless::Vec<u8, { MAX_OBJECT_BYTES + HEADER_BYTES }>>; 16],
    pub fail_next_write: bool,
    /// When true, the next write stores a corrupted copy (bit flip).
    pub corrupt_next_write: bool,
    /// Simulate an interrupted write: store only the first half of bytes.
    pub truncate_next_write: bool,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            slots: [const { None }; 16],
            fail_next_write: false,
            corrupt_next_write: false,
            truncate_next_write: false,
        }
    }

    /// Read the raw bytes stored under a key (test introspection).
    pub fn raw(&self, key: u8) -> Option<&[u8]> {
        self.slots[usize::from(key & 0x0F)].as_deref()
    }
}

impl Storage for MemoryStorage {
    fn write_object(&mut self, key: u8, bytes: &[u8]) -> StorageResult<()> {
        if self.fail_next_write {
            self.fail_next_write = false;
            return Err(StorageError::Fault);
        }
        let mut stored: heapless::Vec<u8, { MAX_OBJECT_BYTES + HEADER_BYTES }> =
            heapless::Vec::from_slice(bytes).map_err(|_| StorageError::TooLarge)?;
        if self.corrupt_next_write {
            self.corrupt_next_write = false;
            if let Some(last) = stored.last_mut() {
                *last ^= 0xFF;
            }
        }
        if self.truncate_next_write {
            self.truncate_next_write = false;
            let keep = stored.len() / 2;
            stored.truncate(keep);
        }
        self.slots[usize::from(key & 0x0F)] = Some(stored);
        Ok(())
    }

    fn read_object(&mut self, key: u8, out: &mut [u8]) -> StorageResult<usize> {
        let Some(stored) = &self.slots[usize::from(key & 0x0F)] else {
            return Err(StorageError::Corrupt);
        };
        if stored.len() > out.len() {
            return Err(StorageError::TooLarge);
        }
        out[..stored.len()].copy_from_slice(stored);
        Ok(stored.len())
    }

    fn erase_object(&mut self, key: u8) -> StorageResult<()> {
        self.slots[usize::from(key & 0x0F)] = None;
        Ok(())
    }
}

/// Persistence wrapper implementing the two-slot commit protocol on top of
/// any `Storage`. `read` prefers the newly-written slot and falls back to
/// the previous committed slot, which is what makes interrupted writes
/// recoverable.
#[derive(Clone, Debug)]
pub struct SlotStore<S: Storage> {
    primary: S,
    staging: S,
}

impl<S: Storage> SlotStore<S> {
    pub fn new(primary: S, staging: S) -> Self {
        Self { primary, staging }
    }

    /// Commit `payload`: encode + write to staging first, then primary.
    /// A failure during the primary write leaves the staging copy, which
    /// `read` will adopt; a failure before that leaves the old primary.
    pub fn commit(&mut self, key: u8, payload: &[u8]) -> StorageResult<()> {
        let mut buf = [0u8; MAX_OBJECT_BYTES + HEADER_BYTES];
        let written = encode_object(payload, &mut buf)?;
        self.staging.write_object(key, &buf[..written])?;
        // If the primary write fails, the staged copy is authoritative for
        // the next read; the previous primary stays intact until then.
        self.primary.write_object(key, &buf[..written])?;
        Ok(())
    }

    /// Read the newest valid copy, falling back across slots.
    pub fn read(&mut self, key: u8, out: &mut [u8]) -> StorageResult<usize> {
        let mut local = [0u8; MAX_OBJECT_BYTES + HEADER_BYTES];
        if let Ok(len) = self.staging.read_object(key, &mut local)
            && let Ok(payload) = decode_object(&local[..len])
        {
            if payload.len() > out.len() {
                return Err(StorageError::TooLarge);
            }
            out[..payload.len()].copy_from_slice(payload);
            return Ok(payload.len());
        }
        if let Ok(len) = self.primary.read_object(key, &mut local)
            && let Ok(payload) = decode_object(&local[..len])
        {
            if payload.len() > out.len() {
                return Err(StorageError::TooLarge);
            }
            out[..payload.len()].copy_from_slice(payload);
            return Ok(payload.len());
        }
        Err(StorageError::Corrupt)
    }

    /// Erase both slots.
    pub fn erase(&mut self, key: u8) -> StorageResult<()> {
        self.staging.erase_object(key)?;
        self.primary.erase_object(key)?;
        Ok(())
    }

    /// After a successful commit of the same content, drop the duplicate
    /// staging copy so later stale staging data is never preferred.
    pub fn settle(&mut self, key: u8) -> StorageResult<()> {
        self.staging.erase_object(key)
    }
}

/// Serialize the event log to JSON for export/backup.
pub fn write_events_json(events: &EventLog, out: &mut String<8192>) -> core::fmt::Result {
    use core::fmt::Write;
    let _ = out.push_str("{\"schema_version\":\"0.1\",\"events\":[");
    for (index, event) in events.events().iter().enumerate() {
        if index > 0 {
            let _ = out.push(',');
        }
        write!(
            out,
            "{{\"kind\":\"{}\",\"at_ms\":{},\"detail\":",
            event.kind.wire(),
            event.at_ms
        )?;
        crate::json::write_str(out, &event.detail).map_err(|_| core::fmt::Error)?;
        let _ = out.push('}');
    }
    let _ = out.push_str("]}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_RETENTION_DAYS, Event, EventKind, EventLog, MAX_OBJECT_BYTES, MemoryStorage,
        Retention, SlotStore, Storage, StorageError, decode_object, encode_object, fnv1a, keys,
    };

    #[test]
    fn checksum_detects_bit_flip() {
        let payload = b"calibration-payload";
        assert_eq!(fnv1a(payload), fnv1a(b"calibration-payload"));
        assert_ne!(fnv1a(payload), fnv1a(b"calibration-payloae"));
    }

    #[test]
    fn encode_decode_round_trip() {
        let mut buf = [0u8; MAX_OBJECT_BYTES + 16];
        let payload = b"spool-empty-mass:430";
        let n = encode_object(payload, &mut buf).unwrap();
        assert_eq!(decode_object(&buf[..n]).unwrap(), &payload[..]);
    }

    #[test]
    fn corrupt_or_truncated_object_rejected() {
        let mut buf = [0u8; MAX_OBJECT_BYTES + 16];
        let n = encode_object(b"abcd", &mut buf).unwrap();
        // Truncated (interrupted write) copy.
        assert_eq!(decode_object(&buf[..n / 2]), Err(StorageError::Corrupt));
        // Bit flip in the payload.
        buf[n - 1] ^= 0x01;
        assert_eq!(decode_object(&buf[..n]), Err(StorageError::Corrupt));
    }

    #[test]
    fn interrupted_commit_keeps_previous_data() {
        let mut store = SlotStore::new(MemoryStorage::new(), MemoryStorage::new());
        store.commit(keys::CALIBRATION, b"good-v1").unwrap();
        store.settle(keys::CALIBRATION).unwrap();

        // Interrupted write (truncates) into the staging slot.
        store.staging.truncate_next_write = true;
        // The primary write also fails: only the staged partial exists.
        store.primary.fail_next_write = true;
        assert_eq!(
            store.commit(keys::CALIBRATION, b"good-v2"),
            Err(StorageError::Fault)
        );

        // Read must still return the previous committed value.
        let mut out = [0u8; 256];
        let len = store.read(keys::CALIBRATION, &mut out).unwrap();
        assert_eq!(&out[..len], b"good-v1");
    }

    #[test]
    fn corrupt_staging_falls_back_to_committed_primary() {
        let mut store = SlotStore::new(MemoryStorage::new(), MemoryStorage::new());
        store.commit(keys::SPOOL_METADATA, b"v1").unwrap();
        store.settle(keys::SPOOL_METADATA).unwrap();
        // A future interrupted write leaves a corrupt staging entry behind.
        store.staging.corrupt_next_write = true;
        store.primary.fail_next_write = true;
        assert_eq!(
            store.commit(keys::SPOOL_METADATA, b"v2"),
            Err(StorageError::Fault)
        );
        // The corrupt staging copy must not win over the valid primary.
        let mut out = [0u8; 256];
        let len = store.read(keys::SPOOL_METADATA, &mut out).unwrap();
        assert_eq!(&out[..len], b"v1");
    }

    #[test]
    fn storage_fault_propagates() {
        let mut store = SlotStore::new(MemoryStorage::new(), MemoryStorage::new());
        store.staging.fail_next_write = true;
        assert_eq!(store.commit(keys::CONFIG, b"x"), Err(StorageError::Fault));
    }

    #[test]
    fn oversized_object_rejected() {
        let mut store = SlotStore::new(MemoryStorage::new(), MemoryStorage::new());
        let huge = [0u8; MAX_OBJECT_BYTES + 1];
        assert_eq!(
            store.commit(keys::CONFIG, &huge),
            Err(StorageError::TooLarge)
        );
    }

    #[test]
    fn erase_removes_all_versions() {
        let mut store = SlotStore::new(MemoryStorage::new(), MemoryStorage::new());
        store.commit(keys::EVENTS, b"v").unwrap();
        store.erase(keys::EVENTS).unwrap();
        let mut out = [0u8; 64];
        assert_eq!(
            store.read(keys::EVENTS, &mut out),
            Err(StorageError::Corrupt)
        );
    }

    #[test]
    fn event_log_evicts_oldest_bounded() {
        let mut log = EventLog::new();
        for i in 0..70u64 {
            let mut detail = heapless::String::new();
            use core::fmt::Write;
            write!(detail, "ev{i}").unwrap();
            log.push(Event {
                kind: EventKind::UserNote,
                at_ms: i,
                detail,
            });
        }
        assert_eq!(log.len(), 64);
        assert_eq!(log.dropped_count(), 6);
        assert_eq!(log.events().first().unwrap().detail, "ev6");
        assert_eq!(log.events().last().unwrap().detail, "ev69");
    }

    #[test]
    fn retention_bounds_and_minimum() {
        let mut retention = Retention::default();
        assert_eq!(retention.days, DEFAULT_RETENTION_DAYS);
        assert!(retention.set_days(29).is_err());
        assert!(retention.set_days(30).is_ok());
        assert_eq!(retention.max_aggregates(), 720);
        assert!(retention.set_days(91).is_err());
        assert_eq!(retention.days, 30, "failed set must not mutate");
    }

    #[test]
    fn raw_memory_storage_round_trip() {
        let mut mem = MemoryStorage::new();
        let mut buf = [0u8; 64];
        let n = encode_object(b"hello", &mut buf).unwrap();
        mem.write_object(keys::CONFIG, &buf[..n]).unwrap();
        let mut read = [0u8; 64];
        let len = mem.read_object(keys::CONFIG, &mut read).unwrap();
        assert_eq!(decode_object(&read[..len]).unwrap(), b"hello");
        assert!(mem.raw(keys::CONFIG).is_some());
    }
}
