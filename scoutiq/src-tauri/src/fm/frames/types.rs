//! Fingerprints for differential comparison (not cryptographic).

pub fn fnv1a_64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn digest_hex(data: &[u8]) -> String {
    format!("{:016x}", fnv1a_64(data))
}
