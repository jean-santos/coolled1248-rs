/// Escape bytes 0x01, 0x02 and 0x3 wit an aditional byte. (i.e 0x01 turns into 0x02 0x05)
pub fn escape_byets_in_place(out: &mut [u8], current_bytes_wrote: usize) -> usize {
    let mut last_bytes_wrote = current_bytes_wrote;
    for idx in 0..out.len() {
        if out[idx] > 0 && out[idx] < 4 {
            out[idx..].copy_within(0..last_bytes_wrote - idx, 1);
            out[idx] = 0x02;
            out[idx + 1] ^= 0x4;
            last_bytes_wrote += 1;
        }
    }
    last_bytes_wrote
}

/// Escape bytes 0x01, 0x02 and 0x3 wit an aditional byte. (i.e 0x01 turns into 0x02 0x05)
pub fn escpae_bytes<F: FnMut(u8)>(mut func: F, data: u8) {
    if data > 0 && data < 4 {
        func(0x02);
        func(data ^ 0x4);
    } else {
        func(data);
    }
}

pub fn calculate_checksum(data: &[u8]) -> u8 {
    data.iter().fold(0, |acc, e| acc ^ e)
}