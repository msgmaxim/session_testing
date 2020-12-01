
use serde_json::{Value, json};

use crate::{ecdh, http_clients::{ClearnetClient, HttpClient, OnionClient, Request}, loki::{LOCAL_NET, Network, ServiceNode}, onions::{NextHop, OnionPath}};


pub async fn onion_request_v2(
    client: &reqwest::Client,
    path: &OnionPath,
    payload: &[u8],
) -> Result<String, String> {
    let (payload, decryption_key) = crate::onions_core::v2::onion_request(path, payload).await;

    // Send to node 1

    // The first node is alays of type `Node`
    let first_node = match &path.node_1 {
        NextHop::Node(n) => n,
        NextHop::Server(_) => panic!("First node must not be a server"),
    };

    let target = "onion_req/v2";

    let url = format!(
        "https://{}:{}/{}",
        first_node.public_ip, first_node.storage_port, target
    );

    // println!("encrypted size: {}", payload.len());

    let time_now = std::time::Instant::now();

    let res = client
        .post(&url)
        .body(payload)
        .send()
        .await
        .map_err(|e| format!("Could not send request: {}", e))?;

    // println!("Request roundtrip: {}ms", time_now.elapsed().as_millis());

    let status = res.status();

    let success = status.is_success();

    let res_body = res
        .text()
        .await
        .map_err(|_e| "could not get response body")?;

    if !success {
        return Err(format!(
            "😵 Onion request failed: [{}] <{}>",
            status, &res_body
        ));
    }

    decrypt(res_body, &decryption_key).ok_or("Decryption error".to_owned())
}

fn decrypt(ciphertext: String, key: &Vec<u8>) -> Option<String> {
    ecdh::aes_gcm_decrypt(ciphertext, key)
}

/// This is how the Snode Entry result looks like received from SN
#[derive(serde::Deserialize)]
struct ServiceNodeInner {
    address: String,
    ip: String,
    port: String,
    pubkey_ed25519: String,
    pubkey_x25519: String,
}

// #[serde(alias = "ip")]
// pub public_ip: String,
// #[serde(alias = "port")]
// pub storage_port: u16,
// pub storage_lmq_port: u16,
// pub service_node_pubkey: String,
// pub operator_address: String,
// pub pubkey_x25519: String,
// pub pubkey_ed25519: String,
// pub swarm_id: u64,

impl From<ServiceNodeInner> for ServiceNode {
    fn from(sn: ServiceNodeInner) -> Self {
        ServiceNode {
            public_ip: sn.ip,
            storage_port: sn.port.parse().expect("Port is not u16"),
            storage_lmq_port: 0,
            service_node_pubkey: "".to_string(),
            operator_address: "".to_string(),
            pubkey_x25519: sn.pubkey_x25519,
            pubkey_ed25519: sn.pubkey_ed25519,
            swarm_id: 0,
        }
    }
}


pub async fn get_swarm_for_pk(
    sn: &ServiceNode,
    pk: &str,
) -> Result<Vec<ServiceNode>, &'static str> {
    let url = format!(
        "https://{}:{}/storage_rpc/v1",
        &sn.public_ip, &sn.storage_port
    );

    let params = json!({
        "method": "get_snodes_for_pubkey",
        "params": {
            "pubKey": &pk,
        }
    });

    let payload = params.to_string();


    let mut client = ClearnetClient::new();

    // let mut client = OnionClient::init(&LOCAL_NET).await;

    let req = Request {
        url,
        method: "POST".to_string(),
        body: payload,
    };

    let res_text = client.send(req).await.map_err(|_| "Could not contact node")?;

    let v: Value = serde_json::from_str(&res_text).map_err(|_| "body is not json")?;

    let array = v["snodes"].clone();


    let nodes: Vec<ServiceNodeInner> = serde_json::from_value(array).map_err(|_| "Could not parse Service Node entries")?;

    let nodes : Vec<ServiceNode> = nodes.into_iter().map(|sn| sn.into()).collect();

    Ok(nodes)
}