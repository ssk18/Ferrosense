use crate::{Reading, crc16, find_frame, parse};

/// Stateful streaming decoder: feed it raw bytes with [`Decoder::push`], pull out
/// validated readings with [`Decoder::next_reading`].
pub struct Decoder {
    buf: Vec<u8>,
}

impl Decoder {
    pub fn new() -> Self {
        Decoder { buf: Vec::new() }
    }

    /// Append a freshly-arrived chunk of bytes (e.g. one BLE notification).
    pub fn push(&mut self, chunk: &[u8]) {
        self.buf.extend_from_slice(chunk);
    }

    /// Return the next complete, CRC-valid reading, or `None` if the buffer
    /// doesn't hold one yet. Corrupt or unparseable frames are skipped.
    pub fn next_reading(&mut self) -> Option<Reading> {
        loop {
            // A. Resync: drop anything before the first sync byte (0xAA).
            let start = self.buf.iter().position(|&b| b == 0xAA)?;
            self.buf.drain(..start);

            // B. Do we have a whole frame now? (None → wait for more bytes)
            let frame = find_frame(&self.buf)?;

            // C. READ phase — frame borrows self.buf, so pull out OWNED values only.
            let total = frame.len();
            let stored_crc = u16::from_le_bytes([frame[total - 2], frame[total - 1]]);
            let computed_crc = crc16(&frame[1..total - 2]); // over length … seq
            let decoded = if computed_crc == stored_crc {
                parse(frame[2], &frame[3..total - 4]).ok() // Ok → Some, Err → None
            } else {
                None
            };
            // frame's last use was above → its borrow ends here.

            // D. MUTATE phase — now legal to reshape self.buf.
            match decoded {
                Some(reading) => {
                    self.buf.drain(..total); // consume the whole frame
                    return Some(reading);
                }
                None => {
                    self.buf.drain(..1); // bad frame: drop the false sync, loop resyncs
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a valid frame (sync + length + type + payload + seq + real CRC).
    fn frame(msg_type: u8, payload: &[u8], seq: u16) -> Vec<u8> {
        let mut f = vec![0xAA, payload.len() as u8, msg_type];
        f.extend_from_slice(payload);
        f.extend_from_slice(&seq.to_le_bytes());
        let crc = crc16(&f[1..]); // length … seq (everything after the sync so far)
        f.extend_from_slice(&crc.to_le_bytes());
        f
    }

    #[test]
    fn decodes_a_single_frame() {
        let mut d = Decoder::new();
        d.push(&frame(0x01, &[0x64, 0x00], 1)); // Temperature 100 centi-°C
        assert_eq!(d.next_reading(), Some(Reading::Temperature { centi_c: 100 }));
        assert_eq!(d.next_reading(), None); // nothing left
    }

    #[test]
    fn decodes_across_two_pushes() {
        let f = frame(0x02, &[87], 1); // Battery 87%
        let mut d = Decoder::new();
        d.push(&f[..3]); // only part of the frame arrives
        assert_eq!(d.next_reading(), None); // incomplete → wait
        d.push(&f[3..]); // the rest arrives
        assert_eq!(d.next_reading(), Some(Reading::Battery { percent: 87 }));
    }

    #[test]
    fn skips_leading_junk() {
        let mut d = Decoder::new();
        d.push(&[0xFF, 0x00]); // garbage before the frame
        d.push(&frame(0x03, &[], 5)); // Heartbeat
        assert_eq!(d.next_reading(), Some(Reading::Heartbeat));
    }

    #[test]
    fn rejects_a_corrupted_frame() {
        let mut bad = frame(0x01, &[0x64, 0x00], 1);
        bad[3] ^= 0xFF; // flip a payload bit → CRC no longer matches
        let mut d = Decoder::new();
        d.push(&bad);
        assert_eq!(d.next_reading(), None); // corruption caught, no reading emitted
    }

    #[test]
    fn decodes_two_frames_back_to_back() {
        let mut d = Decoder::new();
        d.push(&frame(0x01, &[0x64, 0x00], 1));
        d.push(&frame(0x02, &[50], 2));
        assert_eq!(d.next_reading(), Some(Reading::Temperature { centi_c: 100 }));
        assert_eq!(d.next_reading(), Some(Reading::Battery { percent: 50 }));
        assert_eq!(d.next_reading(), None);
    }
}
