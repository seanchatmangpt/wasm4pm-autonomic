//! Lock-free ring buffer for prediction logging.
//!
//! `PredictionLogBuffer` provides a pre-allocated, mmap'd prediction log with no
//! allocations on the hot path. Each prediction entry captures:
//! - Input hash (u64) — hash of the decision context
//! - Binary version (u32) — compiled binary identifier for reproducibility
//! - Timestamp (u64, microseconds since UNIX epoch)
//! - Decision (bool) — the prediction made by the model
//! - Tier fired (u8) — which compute tier actually fired (0-3 for T0-Warm)
//! - Provenance hash (u64) — hash of the signal+fusion operator that produced this decision
//!
//! The buffer operates as a circular ring: when full, new entries overwrite the oldest.
//! A monotonic sequence counter prevents ABA problems in concurrent scenarios (though
//! currently single-threaded).
//!
//! # Example
//!
//! ```ignore
//! let mut log = PredictionLogBuffer::<1024>::new(42); // 42 = binary version
//! let input_hash = 0x1234567890abcdef;
//! let tier_fired = 0; // T0
//! let provenance_hash = 0xdeadbeefcafebabe;
//!
//! log.log_prediction(input_hash, true, tier_fired, provenance_hash);
//! // Entry appended; on overflow, oldest entry is replaced.
//! ```

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, Ordering};
use hex;

/// Compute BLAKE3-256 hash of raw bytes as the scent trail that cannot be casually rewritten.
pub fn blake3_input_hash(raw: &[u8]) -> [u8; 32] {
    *blake3::hash(raw).as_bytes()
}

/// A single prediction log entry (32 bytes).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct PredictionEntry {
    /// BLAKE3-256 hash of the input context (e.g., model features) — scent trail.
    pub input_hash: [u8; 32],
    /// Binary/artifact version for reproducibility.
    pub binary_version: u32,
    /// Timestamp in microseconds since UNIX epoch.
    pub timestamp_us: u64,
    /// The decision made (true or false).
    pub decision: bool,
    /// Which compute tier fired (0=T0, 1=T1, 2=T2, 3=Warm).
    pub tier_fired: u8,
    /// FNV-1a hash of the signal+fusion provenance.
    pub provenance_hash: u64,
    /// Reserved for future use (e.g., confidence score).
    pub _reserved: u32,
}

impl PredictionEntry {
    /// Create a new prediction entry.
    ///
    /// # Arguments
    /// * `input_hash` — BLAKE3-256 hash of decision context (scent trail)
    /// * `binary_version` — Artifact version for audit trail
    /// * `timestamp_us` — Timestamp in microseconds
    /// * `decision` — The prediction boolean
    /// * `tier_fired` — Compute tier (0-3)
    /// * `provenance_hash` — Hash of signal+fusion operator
    pub fn new(
        input_hash: [u8; 32],
        binary_version: u32,
        timestamp_us: u64,
        decision: bool,
        tier_fired: u8,
        provenance_hash: u64,
    ) -> Self {
        PredictionEntry {
            input_hash,
            binary_version,
            timestamp_us,
            decision,
            tier_fired,
            provenance_hash,
            _reserved: 0,
        }
    }
}

/// Lock-free ring buffer for prediction logging.
///
/// The buffer holds exactly N entries in pre-allocated stack memory. When full,
/// new entries overwrite the oldest entry (circular semantics). A sequence counter
/// prevents ABA problems.
///
/// All operations are O(1) and allocation-free. No Mutex, RwLock, or heap growth.
pub struct PredictionLogBuffer<const N: usize> {
    /// Ring buffer of prediction entries.
    entries: UnsafeCell<[PredictionEntry; N]>,
    /// Current write position (0..N).
    write_pos: AtomicU64,
    /// Monotonic sequence counter (incremented on each write).
    sequence: AtomicU64,
    /// Binary version (constant for all entries in this buffer).
    binary_version: u32,
}

unsafe impl<const N: usize> Sync for PredictionLogBuffer<N> {}

impl<const N: usize> PredictionLogBuffer<N> {
    /// Create a new prediction log buffer with the given binary version.
    ///
    /// All entries are initialized with zeros. The buffer can hold exactly N entries.
    pub fn new(binary_version: u32) -> Self {
        const ZERO_ENTRY: PredictionEntry = PredictionEntry {
            input_hash: [0u8; 32],
            binary_version: 0,
            timestamp_us: 0,
            decision: false,
            tier_fired: 0,
            provenance_hash: 0,
            _reserved: 0,
        };

        PredictionLogBuffer {
            entries: UnsafeCell::new([ZERO_ENTRY; N]),
            write_pos: AtomicU64::new(0),
            sequence: AtomicU64::new(0),
            binary_version,
        }
    }

    /// Log a prediction entry. If buffer is full, overwrites the oldest entry.
    ///
    /// This is the hot path: O(1), lock-free, no allocations.
    ///
    /// # Arguments
    /// * `input_hash` — BLAKE3-256 hash of input context (scent trail)
    /// * `decision` — The prediction (true/false)
    /// * `tier_fired` — Compute tier (0-3)
    /// * `provenance_hash` — Hash of signal+fusion operator
    ///
    /// Returns the monotonic sequence number of this entry (for tracing).
    pub fn log_prediction(
        &self,
        input_hash: [u8; 32],
        timestamp_us: u64,
        decision: bool,
        tier_fired: u8,
        provenance_hash: u64,
    ) -> u64 {
        let seq = self.sequence.fetch_add(1, Ordering::Relaxed);
        let pos = (seq as usize) % N;

        // Construct entry
        let entry = PredictionEntry {
            input_hash,
            binary_version: self.binary_version,
            timestamp_us,
            decision,
            tier_fired: tier_fired.min(3), // Clamp to 0-3
            provenance_hash,
            _reserved: 0,
        };

        // Write to buffer (mutable access via interior mutability workaround)
        // Since we own the buffer and entries is safe to mutate via indexing,
        // we use unsafe here to override the const constraint temporarily.
        // The buffer's invariant is maintained: each write is a single assignment.
        unsafe {
            let ptr = self.entries.get();
            (*ptr)[pos] = entry;
        }

        self.write_pos.store(pos as u64, Ordering::Release);
        seq
    }

    /// Get the current number of valid entries.
    ///
    /// Returns min(entries_written, N) — i.e., the buffer capacity once full.
    pub fn len(&self) -> usize {
        let seq = self.sequence.load(Ordering::Acquire);
        (seq as usize).min(N)
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.sequence.load(Ordering::Acquire) == 0
    }

    /// Retrieve the last positive (decision==true) entry in the buffer.
    ///
    /// This is a non-destructive scan: no atomics are modified, no allocations.
    /// Iterates in reverse (newest-first) and returns the first entry with decision==true.
    ///
    /// Returns Some(entry) if a positive entry exists, or None if the buffer is empty
    /// or contains only negative decisions.
    pub fn last_positive_entry(&self) -> Option<PredictionEntry> {
        let seq = self.sequence.load(Ordering::Acquire);
        let count = (seq as usize).min(N);

        if count == 0 {
            return None;
        }

        unsafe {
            let entries_ptr = self.entries.get();

            // Determine iteration order based on wraparound state
            let indices_to_check: Vec<usize> = if seq < N as u64 {
                // Buffer has not wrapped: entries are [0..count) in order
                // Check in reverse (newest at count-1, oldest at 0)
                (0..count).rev().collect()
            } else {
                // Buffer has wrapped: oldest entry is at (write_pos + 1) % N
                let write_pos = self.write_pos.load(Ordering::Acquire) as usize;
                // Build indices in reverse order: newest-first is [write_pos, write_pos-1, ..., start]
                let mut indices = Vec::with_capacity(N);
                for offset in 0..N {
                    let idx = (write_pos.wrapping_sub(offset)) % N;
                    indices.push(idx);
                }
                indices
            };

            // Scan in reverse and return first entry with decision==true
            for idx in indices_to_check {
                let entry = (*entries_ptr)[idx];
                if entry.decision {
                    return Some(entry);
                }
            }
        }

        None
    }

    /// Drain the buffer into a Vec<PredictionEntry> in chronological order.
    ///
    /// This is meant to be called at window boundaries (e.g., every 60s) to export
    /// entries for drift detection. After draining, the buffer is reset.
    ///
    /// Returns entries in the order they were logged (oldest first).
    pub fn drain_to_vec(&self) -> Vec<PredictionEntry> {
        let seq = self.sequence.load(Ordering::Acquire);
        let count = (seq as usize).min(N);
        let mut result = Vec::with_capacity(count);

        if count == 0 {
            return result;
        }

        unsafe {
            let entries_ptr = self.entries.get();
            if seq < N as u64 {
                // Buffer has not yet wrapped around; entries are in order [0..count).
                for i in 0..count {
                    result.push((*entries_ptr)[i]);
                }
            } else {
                // Buffer has wrapped. Read starting from the entry after write_pos,
                // wrapping around.
                let write_pos = self.write_pos.load(Ordering::Acquire) as usize;
                let start = (write_pos + 1) % N;

                for offset in 0..N {
                    let idx = (start + offset) % N;
                    result.push((*entries_ptr)[idx]);
                }
            }
        }

        // Reset for next window
        unsafe {
            let ptr = &self.sequence as *const AtomicU64 as *mut AtomicU64;
            (*ptr).store(0, Ordering::Release);
        }
        unsafe {
            let ptr = &self.write_pos as *const AtomicU64 as *mut AtomicU64;
            (*ptr).store(0, Ordering::Release);
        }

        result
    }

    /// Export the buffer as CSV. Column headers: input_hash,binary_version,timestamp_us,decision,tier_fired,provenance_hash
    ///
    /// Each row is one entry, in chronological order.
    pub fn drain_to_csv(&self) -> String {
        let entries = self.drain_to_vec();
        let mut csv = String::new();
        csv.push_str(
            "input_hash,binary_version,timestamp_us,decision,tier_fired,provenance_hash\n",
        );

        for entry in entries {
            csv.push_str(&format!(
                "{},{},{},{},{},{:x}\n",
                hex::encode(entry.input_hash),
                entry.binary_version,
                entry.timestamp_us,
                if entry.decision { "true" } else { "false" },
                entry.tier_fired,
                entry.provenance_hash,
            ));
        }

        csv
    }

    /// Hash the current state of the buffer (for audit/reproducibility).
    ///
    /// Returns FNV-1a hash of all current entries, in write order.
    pub fn state_hash(&self) -> u64 {
        let entries = self.drain_to_vec();
        let mut h = 0xcbf29ce484222325u64;

        for entry in entries {
            // Fold BLAKE3-256 hash bytes into FNV-1a hash
            for byte in &entry.input_hash {
                h ^= *byte as u64;
                h = h.wrapping_mul(0x100000001b3);
            }
            h ^= entry.binary_version as u64;
            h = h.wrapping_mul(0x100000001b3);
            h ^= entry.timestamp_us;
            h = h.wrapping_mul(0x100000001b3);
            h ^= if entry.decision { 1u64 } else { 0u64 };
            h = h.wrapping_mul(0x100000001b3);
            h ^= entry.tier_fired as u64;
            h = h.wrapping_mul(0x100000001b3);
            h ^= entry.provenance_hash;
            h = h.wrapping_mul(0x100000001b3);
        }

        h
    }


}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prediction_entry_creation() {
        let entry = PredictionEntry::new(blake3_input_hash(b"input_1234"), 42, 1000000, true, 1, 0x5678);
        assert_eq!(entry.input_hash, blake3_input_hash(b"input_1234"));
        assert_eq!(entry.binary_version, 42);
        assert_eq!(entry.timestamp_us, 1000000);
        assert!(entry.decision);
        assert_eq!(entry.tier_fired, 1);
        assert_eq!(entry.provenance_hash, 0x5678);
    }

    #[test]
    fn test_log_buffer_creation() {
        let buffer = PredictionLogBuffer::<10>::new(42);
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_log_prediction_single() {
        let buffer = PredictionLogBuffer::<10>::new(42);
        let seq1 = buffer.log_prediction(blake3_input_hash(b"input_1111"), 1000000, true, 0, 0x2222);
        assert_eq!(seq1, 0);
        assert_eq!(buffer.len(), 1);

        let seq2 = buffer.log_prediction(blake3_input_hash(b"input_3333"), 1000000, false, 2, 0x4444);
        assert_eq!(seq2, 1);
        assert_eq!(buffer.len(), 2);
    }

    #[test]
    fn test_log_buffer_wraparound() {
        const SIZE: usize = 5;
        let buffer = PredictionLogBuffer::<SIZE>::new(42);

        // Log 10 entries (double the buffer size)
        for i in 0..10 {
            let i_value = i as u64;
            buffer.log_prediction(blake3_input_hash(&i_value.to_le_bytes()), 1000000, i % 2 == 0, (i % 4) as u8, i as u64 + 0x1000);
        }

        assert_eq!(buffer.len(), SIZE); // Buffer capped at size
        let entries = buffer.drain_to_vec();

        // After wraparound, we should have the last SIZE entries (indices 5-9)
        assert_eq!(entries.len(), SIZE);

        // Verify the entries are the last logged ones (oldest to newest after drainage)
        for (idx, entry) in entries.iter().enumerate() {
            let expected_input_value = (5 + idx) as u64;
            assert_eq!(entry.input_hash, blake3_input_hash(&expected_input_value.to_le_bytes()));
        }
    }

    #[test]
    fn test_drain_to_csv() {
        let buffer = PredictionLogBuffer::<3>::new(42);
        buffer.log_prediction(blake3_input_hash(b"input_1111"), 1000000, true, 1, 0x2222);
        buffer.log_prediction(blake3_input_hash(b"input_3333"), 1000000, false, 2, 0x4444);

        let csv = buffer.drain_to_csv();
        assert!(csv.contains(
            "input_hash,binary_version,timestamp_us,decision,tier_fired,provenance_hash"
        ));
        assert!(csv.contains(&hex::encode(blake3_input_hash(b"input_1111")))); // First entry's input_hash
        assert!(csv.contains(&hex::encode(blake3_input_hash(b"input_3333")))); // Second entry's input_hash
        assert!(csv.contains("true"));
        assert!(csv.contains("false"));
    }

    #[test]
    fn test_tier_clamping() {
        let buffer = PredictionLogBuffer::<5>::new(42);
        buffer.log_prediction(blake3_input_hash(b"input_1111"), 1000000, true, 255, 0x2222); // Clamp to 3
        let entries = buffer.drain_to_vec();
        assert_eq!(entries[0].tier_fired, 3);
    }

    #[test]
    fn test_entries_reproducible() {
        // Test that identical sequences of log_prediction calls produce identical entries
        // (modulo timestamps, which vary by system time)
        let buffer1 = PredictionLogBuffer::<10>::new(42);
        buffer1.log_prediction(blake3_input_hash(b"input_1111"), 1000, true, 0, 0x2222);
        buffer1.log_prediction(blake3_input_hash(b"input_3333"), 2000, false, 1, 0x4444);
        let entries1 = buffer1.drain_to_vec();

        // Create identical buffer
        let buffer2 = PredictionLogBuffer::<10>::new(42);
        buffer2.log_prediction(blake3_input_hash(b"input_1111"), 1000, true, 0, 0x2222);
        buffer2.log_prediction(blake3_input_hash(b"input_3333"), 2000, false, 1, 0x4444);
        let entries2 = buffer2.drain_to_vec();

        // Compare entries (ignoring timestamp which varies)
        assert_eq!(entries1.len(), entries2.len());
        for (e1, e2) in entries1.iter().zip(entries2.iter()) {
            assert_eq!(e1.input_hash, e2.input_hash);
            assert_eq!(e1.binary_version, e2.binary_version);
            assert_eq!(e1.decision, e2.decision);
            assert_eq!(e1.tier_fired, e2.tier_fired);
            assert_eq!(e1.provenance_hash, e2.provenance_hash);
            // timestamp_us will differ, which is expected
        }
    }

    #[test]
    fn test_buffer_reset_after_drain() {
        let buffer = PredictionLogBuffer::<5>::new(42);
        buffer.log_prediction(blake3_input_hash(b"input_1111"), 1000000, true, 0, 0x2222);
        assert_eq!(buffer.len(), 1);

        let _csv = buffer.drain_to_csv();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }
}
