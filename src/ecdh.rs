use crate::onions::{HasX25519, NextHop};
use ring::{agreement, hmac, rand};

const NONCE_LENGTH: usize = 12;

pub fn encrypt_gcm(target: &NextHop, plaintext: &[u8]) -> (Vec<u8>, Vec<u8>, agreement::PublicKey) {
    // TODO: only initialize once
    let rng = rand::SystemRandom::new();

    let seckey = agreement::EphemeralPrivateKey::generate(&agreement::X25519, &rng)
        .expect("couldn't generate seckey");
    let pubkey = seckey
        .compute_public_key()
        .expect("couldn't compute pubkey");

    let target_key = match target {
        NextHop::Node(n) => n.pubkey_x25519(),
        NextHop::Server(s) => s.pubkey_x25519(),
        NextHop::ServerV2(s) => s.pubkey_x25519(),
    };

    // println!("Encrypting for {:#?} with key {}", target, target_key);

    let peer_pk_bytes = hex::decode(&target_key.as_bytes()).expect("Couldn't decode SN's pubkey");

    if peer_pk_bytes.len() != peer_pk_bytes.len() {
        eprintln!("invalid length for peer target key: {}", target_key.len());
    }

    let peer_pk = agreement::UnparsedPublicKey::new(&agreement::X25519, peer_pk_bytes);

    // Note that this consumes our ephemeral key, we won't be able to reuse it.
    let shared_key = agreement::agree_ephemeral(
        seckey,
        &peer_pk,
        ring::error::Unspecified,
        |_key_material| {
            // In a real application, we'd apply a KDF to the key material and the
            // public keys (as recommended in RFC 7748) and then derive session
            // keys from the result. We omit all that here.
            // Ok(())
            Ok(Vec::from(_key_material))
        },
    );

    let shared_key = match shared_key {
        Ok(key) => key,
        Err(err) => {
            eprintln!("Could not derive shared key: {}", err);
            panic!("Could not derive shared key");
        }
    };

    // Derive key with HKDF
    let salt = "LOKI";

    let s_key = hmac::Key::new(hmac::HMAC_SHA256, salt.as_bytes());
    let symm_key = hmac::sign(&s_key, &shared_key).as_ref().to_vec();

    let iv_and_ciphertext = aes_gcm_encrypt(&plaintext, &symm_key);

    return (iv_and_ciphertext, symm_key, pubkey);
}

pub fn aes_gcm_decrypt(iv_and_ciphertext: String, key: &Vec<u8>) -> Option<String> {
    let iv_and_ciphertext = match base64::decode(&iv_and_ciphertext) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Could not decode ciphertext from base64");
            eprintln!(" ciphertext: <{}>", iv_and_ciphertext);
            return None;
        }
    };

    const TAG_LENGTH: usize = 16;

        // iv_and_ciphertext must be at least NONCE_LENGTH + TAG_LENGTH long
    if iv_and_ciphertext.len() < NONCE_LENGTH + TAG_LENGTH {
        eprintln!("iv_and_ciphertext too short, len: {}", iv_and_ciphertext.len());
        return None;
    }

    let iv = &iv_and_ciphertext[0..NONCE_LENGTH];

    let tag_pos = iv_and_ciphertext.len() - TAG_LENGTH;

    let ciphertext = &iv_and_ciphertext[NONCE_LENGTH..tag_pos];

    let tag = &iv_and_ciphertext[tag_pos..];

    use openssl::symm::{decrypt_aead, Cipher};

    let plaintext = decrypt_aead(
        Cipher::aes_256_gcm(),
        &key,
        Some(&iv),
        &[],
        &ciphertext,
        tag,
    )
    .map_err(|err| {
        eprintln!(
            "Could not decrypt ciphertext, len: {}, error: {}",
            ciphertext.len(),
            err
        );
    })
    .ok()?;

    let plaintext = String::from_utf8_lossy(&plaintext).to_string();

    Some(plaintext)
}

pub fn gen_keypair() -> (
    ring::agreement::EphemeralPrivateKey,
    ring::agreement::PublicKey,
) {
    let rng = rand::SystemRandom::new();

    let seckey = ring::agreement::EphemeralPrivateKey::generate(&ring::agreement::X25519, &rng)
        .expect("couldn't generate seckey");
    let pubkey = seckey
        .compute_public_key()
        .expect("couldn't compute pubkey");

    (seckey, pubkey)
}

fn aes_gcm_encrypt(plaintext: &[u8], shared_key: &Vec<u8>) -> Vec<u8> {
    use openssl::symm::{encrypt_aead, Cipher};
    use ring::rand::SecureRandom;

    // TODO: only initialize once
    let rng = rand::SystemRandom::new();

    let mut iv: [u8; NONCE_LENGTH] = [0; NONCE_LENGTH];
    rng.fill(&mut iv).expect("Failed to generate IV");

    const TAG_LENGTH: usize = 16;

    let mut tag: [u8; TAG_LENGTH] = [0; TAG_LENGTH];

    let mut ciphertext = encrypt_aead(
        Cipher::aes_256_gcm(),
        &shared_key,
        Some(&iv),
        &"".as_bytes(),
        plaintext,
        &mut tag,
    )
    .expect("Failed to encrypt");

    let mut iv_and_ciphertext = iv.to_vec();

    iv_and_ciphertext.append(&mut ciphertext);
    // tag comes after ciphertext
    iv_and_ciphertext.append(&mut tag.to_vec());

    iv_and_ciphertext
}

const IV_LENGTH: usize = 16;

pub fn aes_cbc_decrypt(iv_and_ciphertext: String, sym_key: Vec<u8>) -> Option<String> {
    let iv_and_ciphertext = match base64::decode(&iv_and_ciphertext) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Could not decode ciphertext from base64");
            eprintln!(" ciphertext: <{}>", iv_and_ciphertext);
            return None;
        }
    };

    let iv = &iv_and_ciphertext[0..IV_LENGTH];
    let ciphertext = &iv_and_ciphertext[IV_LENGTH..];

    use openssl::symm::{decrypt, Cipher};

    let plaintext = decrypt(Cipher::aes_256_cbc(), &sym_key, Some(&iv), &ciphertext)
        .expect("Failed to encrypt");

    let plaintext = String::from_utf8_lossy(&plaintext).to_string();

    Some(plaintext)
}

pub fn aes_cbc_derive_and_decrypt(
    iv_and_ciphertext: String,
    seckey: agreement::EphemeralPrivateKey,
    pubkey: &[u8],
) -> Option<String> {
    let peer_pk = ring::agreement::UnparsedPublicKey::new(&ring::agreement::X25519, pubkey);

    let shared_key = ring::agreement::agree_ephemeral(
        seckey,
        &peer_pk,
        ring::error::Unspecified,
        |_key_material| Ok(Vec::from(_key_material)),
    )
    .expect("Failed to derive shared key");

    crate::ecdh::aes_cbc_decrypt(iv_and_ciphertext, shared_key)
}
