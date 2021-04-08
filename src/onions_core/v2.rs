use crate::{ecdh, loki::{LokiServer, LokiServerV2, ServiceNode}, onions::{NextHop, OnionPath}};
use byteorder::{LittleEndian, WriteBytesExt};
use serde_json::json;

pub async fn onion_request(path: &OnionPath, payload: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let ctx_1 = match &path.target {
        NextHop::Node(node) => encrypt_for_target_node(&node, &payload),
        NextHop::Server(server) => encrypt_for_target_server(&server, &payload),
        NextHop::ServerV2(server) => encrypt_for_target_server_v2(&server, &payload),
    };
    // Encrypt for node 3
    let ctx_2 = encrypt_for_relay(&path.node_3, &path.target, &ctx_1);
    // Encrypt for node 2
    let ctx_3 = encrypt_for_relay(&path.node_2, &path.node_3, &ctx_2);
    // Encrypt for node 1
    let ctx_4 = encrypt_for_relay(&path.node_1, &path.node_2, &ctx_3);

    let payload = payload_for_guard_node(&ctx_4);

    (payload, ctx_1.secret_key)
}

struct EncryptionContext {
    // send this to the peer
    ciphertext: Vec<u8>,
    // use this to decrypt response
    secret_key: Vec<u8>,
    // send this to the peer
    ephemeral_pubkey: ring::agreement::PublicKey,
}

// ctx is the result of previous encryption, it contains the actual ciphertext and the ephemeral key used
fn encrypt_for_relay(
    relay: &NextHop,
    next_hop: &NextHop,
    ctx: &EncryptionContext,
) -> EncryptionContext {
    // let mut payload = serialize_blob(&ctx.ciphertext);
    let mut json_payload = json!({
        "ephemeral_key": hex::encode(&ctx.ephemeral_pubkey),
    });

    match next_hop {
        NextHop::Node(sn) => {
            json_payload["destination"] = serde_json::Value::String(sn.pubkey_ed25519.clone());
        }
        NextHop::Server(server) => {
            json_payload["host"] = serde_json::Value::String(server.host.clone());
            json_payload["target"] = serde_json::Value::String(server.target.clone());
        }
        NextHop::ServerV2(server) => {
            json_payload["host"] = serde_json::Value::String(server.host.clone());
            json_payload["target"] = serde_json::Value::String(server.target.clone());
            json_payload["port"] = serde_json::Value::Number(server.port.into());
            json_payload["protocol"] = serde_json::Value::String(server.protocol.clone());
        }
    };

    // println!(
    //     "Encrypt for relay. Payload: {}",
    //     serde_json::to_string_pretty(&json_payload).unwrap()
    // );

    let payload = serialized_combined(&ctx.ciphertext, json_payload);

    // if let NextHop::Server(_server) = &next_hop {
    //     println!("node 3 payload: {:?}", &payload);
    // }

    let (ciphertext, secret_key, ephemeral_pubkey) = ecdh::encrypt_gcm(&relay, &payload);

    EncryptionContext {
        ciphertext,
        secret_key,
        ephemeral_pubkey,
    }
}

fn encrypt_for_target_node(sn: &ServiceNode, payload: &[u8]) -> EncryptionContext {
    // Yes, this is json around json

    let params = json!({
        "headers": ""
    });

    let payload = serialized_combined(payload, params);

    let plaintext = payload;

    let (ciphertext, secret_key, ephemeral_pubkey) =
        ecdh::encrypt_gcm(&NextHop::Node(sn.clone()), &plaintext);

    EncryptionContext {
        ciphertext,
        secret_key,
        ephemeral_pubkey,
    }
}

fn encrypt_for_target_server(server: &LokiServer, payload: &[u8]) -> EncryptionContext {
    let plaintext = std::str::from_utf8(payload).expect("non-utf8");

    let (ciphertext, secret_key, ephemeral_pubkey) =
        ecdh::encrypt_gcm(&NextHop::Server(server.clone()), plaintext.as_bytes());

    EncryptionContext {
        ciphertext,
        secret_key,
        ephemeral_pubkey,
    }
}

fn encrypt_for_target_server_v2(server: &LokiServerV2, payload: &[u8]) -> EncryptionContext {
    let plaintext = std::str::from_utf8(payload).expect("non-utf8");

    let (ciphertext, secret_key, ephemeral_pubkey) =
        ecdh::encrypt_gcm(&NextHop::ServerV2(server.clone()), plaintext.as_bytes());

    EncryptionContext {
        ciphertext,
        secret_key,
        ephemeral_pubkey,
    }
}

fn payload_for_guard_node(ctx: &EncryptionContext) -> Vec<u8> {
    // println!(
    //     "Printing for guard node, ekey: {}",
    //     hex::encode(&ctx.ephemeral_pubkey)
    // );
    // println!("Ciphertext len: {}", ctx.ciphertext.len());

    let json = json!({
        "ephemeral_key": hex::encode(&ctx.ephemeral_pubkey),
    });

    serialized_combined(&ctx.ciphertext, json)
}

fn serialized_combined(ciphertext: &[u8], json: serde_json::Value) -> Vec<u8> {
    let mut payload = serialize_blob(&ciphertext);

    let mut json_bin = json.to_string().as_bytes().to_owned();

    payload.append(&mut json_bin);

    payload
}

fn serialize_blob(blob: &[u8]) -> Vec<u8> {
    let mut result = vec![];

    // 4 bytes: size N
    let size = blob.len();
    result.write_u32::<LittleEndian>(size as u32).unwrap();
    // Followed by N bytes
    result.append(&mut blob.to_owned());

    result
}
