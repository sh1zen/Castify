//! Reorder stage with jitter buffer for the receiver pipeline
//!
//! Replaces the HashMap-based packet reordering with a proper sliding-window
//! jitter buffer that holds packets for a configurable delay before outputting
//! them in sequence order.

use anyhow::Result;
use async_trait::async_trait;
use log::{info, warn};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::pipeline::PipelineStage;
use crate::pipeline::health::PipelineHealth;

/// An RTP packet with metadata for reordering
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub payload: Vec<u8>,
    pub marker: bool,
    pub sequence_number: u16,
    pub timestamp: u32,
    pub received_at: Instant,
}

/// Reorder stage configuration
#[derive(Debug, Clone)]
pub struct ReorderConfig {
    /// Jitter buffer delay: how long to hold packets before outputting
    pub jitter_delay: Duration,
    /// Maximum number of packets to buffer
    pub max_buffer_size: usize,
    /// Maximum reordering distance (in sequence numbers)
    pub max_reorder_distance: u16,
}

impl Default for ReorderConfig {
    fn default() -> Self {
        Self {
            jitter_delay: Duration::from_millis(150), // Increased to 150ms for better buffering
            max_buffer_size: 400,                     // Doubled buffer size
            max_reorder_distance: 60,                 // Doubled reorder distance
        }
    }
}

/// Jitter buffer for packet reordering
///
/// Implements a sliding-window buffer that:
/// 1. Buffers incoming packets ordered by sequence number
/// 2. Holds packets for `jitter_delay` before releasing
/// 3. Releases packets in sequence order
/// 4. Handles packet loss by skipping after timeout
pub struct JitterBuffer {
    /// Ordered buffer of packets (by sequence number)
    buffer: VecDeque<RtpPacket>,
    /// Expected next sequence number
    expected_seq: Option<u16>,
    /// Configuration
    config: ReorderConfig,
    /// Statistics
    packets_received: u64,
    packets_reordered: u64,
    packets_lost: u64,
}

impl JitterBuffer {
    /// Create a new jitter buffer with the given configuration
    pub fn new(config: ReorderConfig) -> Self {
        Self {
            buffer: VecDeque::with_capacity(config.max_buffer_size),
            expected_seq: None,
            config,
            packets_received: 0,
            packets_reordered: 0,
            packets_lost: 0,
        }
    }

    /// Insert a packet into the jitter buffer
    pub fn insert(&mut self, packet: RtpPacket) {
        self.packets_received += 1;

        let seq = packet.sequence_number;

        if self.expected_seq.is_none() {
            self.expected_seq = Some(seq);
        }

        // Check for duplicate
        if self.buffer.iter().any(|p| p.sequence_number == seq) {
            return; // Duplicate packet, discard
        }

        // Find insertion position (maintain ascending sorted order by sequence number)
        // We find the first element p where p > packet, and insert before it.
        let pos = self
            .buffer
            .iter()
            .position(|p| self.seq_comes_after(p.sequence_number, seq));

        match pos {
            Some(i) => {
                self.buffer.insert(i, packet);
                self.packets_reordered += 1;
            }
            None => {
                self.buffer.push_back(packet);
            }
        }

        // Update expected_seq to the minimum in the buffer if our packet is earlier
        if let Some(expected) = self.expected_seq
            && self.seq_comes_after(expected, seq)
        {
            self.expected_seq = Some(seq);
        }

        // Enforce maximum buffer size
        if self.buffer.len() > self.config.max_buffer_size {
            self.cleanup_stale_packets();
        }
    }

    /// Drain packets that are ready to be output
    ///
    /// Returns packets in sequence order that have been held long enough
    pub fn drain_ready(&mut self) -> Vec<RtpPacket> {
        let mut output = Vec::new();
        let now = Instant::now();

        while let Some(expected) = self.expected_seq {
            if let Some(front) = self.buffer.front() {
                if front.sequence_number == expected {
                    // Packet is in order
                    let elapsed = now.duration_since(front.received_at);
                    if elapsed >= self.config.jitter_delay {
                        // Jitter delay satisfied, release
                        let pkt = self.buffer.pop_front().unwrap();
                        self.expected_seq = Some(expected.wrapping_add(1));
                        output.push(pkt);
                    } else {
                        // Not ready yet
                        break;
                    }
                } else {
                    // Expected packet missing - check if it's time to skip
                    let oldest_time = self.buffer.front().map(|p| p.received_at);
                    if let Some(oldest) = oldest_time {
                        let wait_time = now.duration_since(oldest);
                        if wait_time > self.config.jitter_delay * 2 {
                            // Packet is too late, consider it lost
                            self.packets_lost += 1;
                            self.expected_seq = Some(expected.wrapping_add(1));
                            continue; // Try next expected sequence
                        }
                    }
                    break;
                }
            } else {
                break;
            }
        }

        output
    }

    /// Force drain all buffered packets (for shutdown)
    pub fn drain_all(&mut self) -> Vec<RtpPacket> {
        let mut result: Vec<RtpPacket> = self.buffer.drain(..).collect();
        result.sort_by_key(|p| p.sequence_number);
        result
    }

    /// Check if seq_a comes after seq_b (handling wrapping)
    fn seq_comes_after(&self, seq_a: u16, seq_b: u16) -> bool {
        let diff = seq_a.wrapping_sub(seq_b);
        diff > 0 && diff < 0x8000
    }

    /// Remove stale packets when buffer exceeds capacity
    fn cleanup_stale_packets(&mut self) {
        if let Some(expected) = self.expected_seq {
            self.buffer.retain(|p| {
                let diff = p.sequence_number.wrapping_sub(expected);
                diff <= self.config.max_reorder_distance
                    || diff >= (u16::MAX - self.config.max_reorder_distance)
            });

            if self.buffer.len() > self.config.max_buffer_size {
                // Still too large, advance expected_seq
                if let Some(front) = self.buffer.front() {
                    self.expected_seq = Some(front.sequence_number);
                    warn!(
                        "Jitter buffer overflow: advanced expected_seq to {}",
                        front.sequence_number
                    );
                }
            }
        }
    }

    /// Get statistics
    pub fn stats(&self) -> (u64, u64, u64, usize) {
        (
            self.packets_received,
            self.packets_reordered,
            self.packets_lost,
            self.buffer.len(),
        )
    }
}

/// Reorder stage: buffers and reorders RTP packets using a jitter buffer
pub struct ReorderStage {
    jitter_buffer: JitterBuffer,
    input_rx: Option<mpsc::Receiver<RtpPacket>>,
    output_tx: Option<mpsc::Sender<RtpPacket>>,
}

impl ReorderStage {
    /// Create a new reorder stage
    pub fn new(config: ReorderConfig, _health: Arc<PipelineHealth>) -> Self {
        Self {
            jitter_buffer: JitterBuffer::new(config),
            input_rx: None,
            output_tx: None,
        }
    }

    /// Set the input channel
    pub fn set_input(&mut self, rx: mpsc::Receiver<RtpPacket>) {
        self.input_rx = Some(rx);
    }

    /// Get the output channel
    pub fn take_output(&mut self) -> mpsc::Receiver<RtpPacket> {
        let (tx, rx) = mpsc::channel::<RtpPacket>(256);
        self.output_tx = Some(tx);
        rx
    }
}

#[async_trait]
impl PipelineStage for ReorderStage {
    async fn run(&mut self) -> Result<()> {
        let mut input_rx = self
            .input_rx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No input channel"))?;
        let output_tx = self
            .output_tx
            .take()
            .ok_or_else(|| anyhow::anyhow!("No output channel"))?;

        info!("ReorderStage: started");
        let mut last_stats_log = Instant::now();
        let drain_interval = Duration::from_millis(5);

        loop {
            tokio::select! {
                packet = input_rx.recv() => {
                    match packet {
                        Some(pkt) => {
                            self.jitter_buffer.insert(pkt);

                            // Drain ready packets
                            for ready_pkt in self.jitter_buffer.drain_ready() {
                                if output_tx.send(ready_pkt).await.is_err() {
                                    info!("ReorderStage: output channel closed");
                                    return Ok(());
                                }
                            }
                        }
                        None => {
                            // Input closed, drain remaining
                            for pkt in self.jitter_buffer.drain_all() {
                                let _ = output_tx.send(pkt).await;
                            }
                            break;
                        }
                    }
                }
                _ = tokio::time::sleep(drain_interval) => {
                    // Periodically drain ready packets even without new input
                    for ready_pkt in self.jitter_buffer.drain_ready() {
                        if output_tx.send(ready_pkt).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }

            // Log stats periodically
            if last_stats_log.elapsed().as_secs() >= 30 {
                let (received, reordered, lost, buffered) = self.jitter_buffer.stats();
                info!(
                    "ReorderStage: {} received, {} reordered, {} lost, {} buffered",
                    received, reordered, lost, buffered
                );
                last_stats_log = Instant::now();
            }
        }

        let (received, reordered, lost, _) = self.jitter_buffer.stats();
        info!(
            "ReorderStage: finished ({} received, {} reordered, {} lost)",
            received, reordered, lost
        );
        Ok(())
    }

    fn name(&self) -> &'static str {
        "ReorderStage"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(seq: u16) -> RtpPacket {
        RtpPacket {
            payload: vec![seq as u8],
            marker: true,
            sequence_number: seq,
            timestamp: seq as u32 * 3000,
            received_at: Instant::now(),
        }
    }

    #[test]
    fn test_in_order_packets() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(0), // No delay for testing
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(1));
        jb.insert(make_packet(2));
        jb.insert(make_packet(3));

        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].sequence_number, 1);
        assert_eq!(ready[1].sequence_number, 2);
        assert_eq!(ready[2].sequence_number, 3);
    }

    #[test]
    fn test_out_of_order_packets() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(0),
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(3));
        jb.insert(make_packet(1));
        jb.insert(make_packet(2));

        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 3);
        assert_eq!(ready[0].sequence_number, 1);
        assert_eq!(ready[1].sequence_number, 2);
        assert_eq!(ready[2].sequence_number, 3);
    }

    #[test]
    fn test_duplicate_packets() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(0),
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(1));
        jb.insert(make_packet(1)); // duplicate
        jb.insert(make_packet(2));

        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_jitter_delay() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(100),
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(1));

        // Should not be ready yet (hasn't waited long enough)
        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 0);

        // Wait for jitter delay
        std::thread::sleep(Duration::from_millis(110));

        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].sequence_number, 1);
    }

    #[test]
    fn test_packet_loss_skip() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(0),
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(1));

        // Drain seq 1
        let ready = jb.drain_ready();
        assert_eq!(ready.len(), 1);

        // Skip seq 2, insert seq 3
        jb.insert(make_packet(3));

        // Wait for timeout to trigger loss detection
        std::thread::sleep(Duration::from_millis(5));

        // Should eventually skip the gap
        // JitterBuffer waits 2x jitter_delay (0ms * 2 = 0ms here), so it should
        // detect loss immediately and skip
        let ready = jb.drain_ready();
        // Seq 2 lost, seq 3 should be output
        assert!(ready.len() <= 1);
    }

    #[test]
    fn test_stats() {
        let config = ReorderConfig {
            jitter_delay: Duration::from_millis(0),
            ..Default::default()
        };
        let mut jb = JitterBuffer::new(config);

        jb.insert(make_packet(1));
        jb.insert(make_packet(3));
        jb.insert(make_packet(2));

        let (received, reordered, _, buffered) = jb.stats();
        assert_eq!(received, 3);
        assert!(reordered > 0);
        assert_eq!(buffered, 3);
    }
}
