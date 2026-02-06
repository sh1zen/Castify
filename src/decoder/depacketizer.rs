use bytes::BytesMut;
use bytes::BufMut;

/// Reassembles H.264 NAL units from RTP packets (RFC 6184)
/// into complete Annex B access units.
pub struct H264Depacketizer {
    buffer: BytesMut,
    seen_idr: bool,
}

const START_CODE: [u8; 4] = [0, 0, 0, 1];

impl H264Depacketizer {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            seen_idr: false,
        }
    }

    /// Feed one RTP payload + marker bit.
    /// Returns `Some(access_unit)` when a complete access unit is ready.
    pub fn push(&mut self, payload: &[u8], marker: bool) -> Option<Vec<u8>> {
        if payload.is_empty() {
            return None;
        }

        let nal_type = payload[0] & 0x1F;

        match nal_type {
            // Single NAL unit (types 1-23)
            1..=23 => {
                self.buffer.put_slice(&START_CODE);
                self.buffer.put_slice(payload);
            }
            // STAP-A (type 24)
            24 => {
                let mut offset = 1; // skip aggregation header
                while offset + 2 <= payload.len() {
                    let nalu_len = u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
                    offset += 2;
                    if offset + nalu_len > payload.len() {
                        break;
                    }
                    self.buffer.put_slice(&START_CODE);
                    self.buffer.put_slice(&payload[offset..offset + nalu_len]);
                    offset += nalu_len;
                }
            }
            // FU-A (type 28)
            28 => {
                if payload.len() < 2 {
                    return None;
                }
                let fu_indicator = payload[0];
                let fu_header = payload[1];
                let start = (fu_header & 0x80) != 0;

                if start {
                    // Reconstruct the NAL header: NRI from indicator, type from FU header
                    let nal_header = (fu_indicator & 0xE0) | (fu_header & 0x1F);
                    self.buffer.put_slice(&START_CODE);
                    self.buffer.put_u8(nal_header);
                }

                if payload.len() > 2 {
                    self.buffer.put_slice(&payload[2..]);
                }
            }
            _ => {
                // Unknown NAL type, ignore
                return None;
            }
        }

        if marker {
            self.drain_au()
        } else {
            None
        }
    }

    /// Drain the buffer and return the complete access unit if valid.
    fn drain_au(&mut self) -> Option<Vec<u8>> {
        if self.buffer.is_empty() {
            return None;
        }

        let data = self.buffer.split().freeze().to_vec();

        if !self.seen_idr {
            // Check if this AU contains an IDR (NAL type 5)
            if contains_nal_type(&data, 5) {
                self.seen_idr = true;
            } else {
                // Discard frames before the first IDR
                return None;
            }
        }

        Some(data)
    }
}

/// Scan Annex B data for a NAL unit with the given type.
fn contains_nal_type(data: &[u8], target_type: u8) -> bool {
    let mut i = 0;
    while i + 4 < data.len() {
        // Look for start codes (0x00 0x00 0x00 0x01)
        if data[i] == 0 && data[i + 1] == 0 && data[i + 2] == 0 && data[i + 3] == 1 {
            if i + 4 < data.len() && (data[i + 4] & 0x1F) == target_type {
                return true;
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    false
}
