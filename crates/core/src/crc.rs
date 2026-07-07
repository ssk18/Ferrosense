//! CRC-16/CCITT checksum — detects corrupted packets before we trust them.
//!
//! It computes the remainder when the bytes (treated as one long binary number)
//! are XOR-divided by the polynomial `0x1021`, starting from `0xFFFF`. Sender and
//! receiver run the same function; a mismatch means the packet was corrupted.

/// Compute the CRC-16/CCITT (polynomial `0x1021`, initial value `0xFFFF`) over `data`.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF; // the running remainder register, starts all-ones

    for &byte in data {
        crc ^= (byte as u16) << 8; // fold this byte into the top 8 bits of the register

        for _ in 0..8 {
            // process the byte's 8 bits, most-significant first
            if (crc & 0x8000) != 0 {
                // top bit is 1 → shift left, then XOR the polynomial
                crc = (crc << 1) ^ 0x1021;
            } else {
                // top bit is 0 → just shift left
                crc <<= 1;
            }
        }
    }

    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_standard_check_vector() {
        // The official CRC-16/CCITT-FALSE test: "123456789" must give 0x29B1.
        // If our loop is correct, this is the value it produces.
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn empty_input_is_the_initial_value() {
        // No bytes processed → the register is still its 0xFFFF start value.
        assert_eq!(crc16(b""), 0xFFFF);
    }

    #[test]
    fn one_flipped_bit_changes_the_crc() {
        let intact = crc16(&[0xAA, 0x02, 0x01]);
        let corrupted = crc16(&[0xAA, 0x02, 0x00]); // last byte differs by one bit
        assert_ne!(intact, corrupted); // corruption is detected
    }
}
