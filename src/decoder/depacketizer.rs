use bytes::BufMut;
use bytes::BytesMut;

/// Reassembles H.264 NAL units from RTP packets (RFC 6184)
/// into complete Annex B access units.
pub struct H264Depacketizer {
    buffer: BytesMut,
    seen_idr: bool,
    in_fua_fragment: bool,
}

const START_CODE: [u8; 4] = [0, 0, 0, 1];

impl Default for H264Depacketizer {
    fn default() -> Self {
        Self::new()
    }
}

impl H264Depacketizer {
    pub fn new() -> Self {
        Self {
            buffer: BytesMut::new(),
            seen_idr: false,
            in_fua_fragment: false,
        }
    }

    /// Reset the depacketizer state, discarding all buffered data.
    /// After reset, frames are discarded until the next IDR.
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.seen_idr = false;
        self.in_fua_fragment = false;
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
                self.push_nal(payload);
            }
            // STAP-A (type 24)
            24 => {
                let mut offset = 1; // skip aggregation header
                while offset + 2 <= payload.len() {
                    let nalu_len =
                        u16::from_be_bytes([payload[offset], payload[offset + 1]]) as usize;
                    offset += 2;
                    if offset + nalu_len > payload.len() {
                        break;
                    }
                    self.push_nal(&payload[offset..offset + nalu_len]);
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
                let end = (fu_header & 0x40) != 0;

                if start {
                    if self.in_fua_fragment {
                        // Previous fragment was incomplete (lost end packet) — discard it
                        log::warn!(
                            "FU-A: new start while previous fragment incomplete, discarding buffer"
                        );
                        self.buffer.clear();
                    }
                    self.in_fua_fragment = true;
                    // Reconstruct the NAL header: NRI from indicator, type from FU header
                    let nal_header = (fu_indicator & 0xE0) | (fu_header & 0x1F);
                    self.push_nal_header(nal_header);
                } else if !self.in_fua_fragment {
                    // Middle/end packet without a start — fragment was lost
                    return None;
                }

                if payload.len() > 2 {
                    self.buffer.put_slice(&payload[2..]);
                }

                if end {
                    self.in_fua_fragment = false;
                }
            }
            _ => {
                // Unknown NAL type, ignore
                return None;
            }
        }

        if marker {
            self.in_fua_fragment = false;
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

    #[inline]
    fn push_nal_header(&mut self, header: u8) {
        self.buffer.put_slice(&START_CODE);
        self.buffer.put_u8(header);
    }

    #[inline]
    fn push_nal(&mut self, payload: &[u8]) {
        self.buffer.put_slice(&START_CODE);
        self.buffer.put_slice(payload);
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
