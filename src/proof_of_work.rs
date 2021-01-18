use byteorder::{BigEndian, ByteOrder, WriteBytesExt};
use sha2::{Digest, Sha512};
use std::mem;

// ttl is in seconds
fn compute_target(ttl: u64, payload_len: u32, difficulty: u32) -> u64 {
    let total_len = (payload_len + 8) as u64;
    let ttl_mult = ttl * (total_len as u64);
    let inner_frac = ttl_mult / (std::u16::MAX as u64);

    let target = std::u64::MAX / (difficulty as u64 * (total_len + inner_frac));

    target
}

pub fn compute_nonce(timestamp: u64, ttl: u64, pubkey: &str, data: &str) -> [u8; 8] {
    let payload = format!("{}{}{}{}", timestamp, ttl, pubkey, data);
    // let payload_copy = payload.clone();

    let difficulty = 1;

    let target = compute_target(ttl / 1000, payload.len() as u32, difficulty);

    let mut nonce: u64 = 0;

    let mut nonce_bytes = [0u8; mem::size_of::<u64>()];

    loop {
        let mut hasher = Sha512::new();

        hasher.input(&payload);

        let mut result = hasher.result()[..].to_vec();
        nonce_bytes
            .as_mut()
            .write_u64::<BigEndian>(nonce)
            .expect("Unable to write");

        let mut payload = vec![];

        payload.extend_from_slice(&mut nonce_bytes);
        payload.append(&mut result);

        let mut hasher = Sha512::new();

        hasher.input(&payload);

        let hash = hasher.result()[..].to_vec();

        let hash_64 = BigEndian::read_u64(&hash);

        // let hash_hex = hex::encode(&hash);
        let hash_hex = format!("{:016x}", hash_64);

        let target_hex = format!("{:016x}", target);

        // hash to hex

        let res = BigEndian::read_u64(&hash_hex.as_bytes());
        let target = BigEndian::read_u64(&target_hex.as_bytes());

        if res < target {
            break;
        }

        nonce += 1;
    }

    nonce_bytes
}

#[test]
fn test_compute_target() {
    let ttl: u64 = 86400;
    let payload_len = 625;
    let difficulty = 10;

    let mut expected: [u8; 8] = [0, 4, 119, 164, 35, 224, 222, 64];

    let epxected = BigEndian::read_u64(&mut expected);

    let computed = compute_target(ttl, payload_len, difficulty);

    assert_eq!(epxected, computed);
}
