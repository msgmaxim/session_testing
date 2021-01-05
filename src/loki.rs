
use std::fmt::{self, Debug};

use rand::{RngCore, prelude::StdRng};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, Debug, Clone)]
pub struct ServiceNode {
    #[serde(alias = "ip")]
    pub public_ip: String,
    #[serde(alias = "port")]
    pub storage_port: u16,
    pub storage_lmq_port: u16,
    pub service_node_pubkey: String,
    pub operator_address: String,
    pub pubkey_x25519: String,
    pub pubkey_ed25519: String,
    pub swarm_id: u64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LokiServer {
    pub host: String,
    pub target: String,
    pub pubkey_x25519: String,
}

impl fmt::Display for ServiceNode {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // port is most useful when testing locally, might change this for mainnet/testnet
        write!(f, "{}:{}", self.public_ip, self.storage_port)
    }

}

impl fmt::Display for LokiServer {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // port is most useful when testing locally, might change this for mainnet/testnet
        write!(f, "{}{}", self.host, self.target)
    }
}

#[derive(Clone)]
pub struct Network {
    pub seed_url: &'static str,
    pub is_testnet: bool,
}

pub const LOCAL_NET: Network = Network {
    seed_url: "http://localhost:22129/json_rpc",
    is_testnet: false
};

pub const TESTNET: Network = Network {
    seed_url: "http://public.loki.foundation:38157/json_rpc",
    is_testnet: true,
};

pub const MAINNET: Network = Network {
    seed_url: "http://public.loki.foundation:22023/json_rpc",
    is_testnet: false,
};

pub async fn get_n_service_nodes(limit: u32, network: &Network) -> Vec<ServiceNode> {
    let client = reqwest::Client::new();

    let params = json!({
        "jsonrpc": "2.0",
        "id": "0",
        "method": "get_n_service_nodes",
        "params": {
            "limit": &limit,
            "fields": {
                "public_ip": true,
                "storage_port": true,
                "storage_lmq_port": true,
                "service_node_pubkey": true,
                "operator_address": true,
                "pubkey_x25519": true,
                "pubkey_ed25519": true,
                "swarm_id": true,
            },
            "active_only": true,
        },
    });

    let res = client
        .post(network.seed_url)
        .json(&params)
        .send()
        .await
        .expect("Failed to send get_n_service_nodes");

    let res_text = res.text().await.expect("obtaining text from response");

    let v: Value = serde_json::from_str(&res_text).expect("parsing json");

    let array = &v["result"]["service_node_states"];

    serde_json::from_value(array.clone()).expect("from json to value")
}

#[derive(Clone)]
pub struct PubKey {
    data: [u64; 4],
    is_testnet: bool,
}

impl Debug for PubKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PubKey: <{}>", self.to_string())
    }
}


impl PubKey {
    pub fn new(data: &str, is_testnet: bool) -> Option<PubKey> {
        if data.len() != 64 {
            return None;
        }

        let pk0 = u64::from_str_radix(&data[0..16], 16).unwrap();
        let pk1 = u64::from_str_radix(&data[16..32], 16).unwrap();
        let pk2 = u64::from_str_radix(&data[32..48], 16).unwrap();
        let pk3 = u64::from_str_radix(&data[48..64], 16).unwrap();

        Some(PubKey {
            data: [pk0, pk1, pk2, pk3],
            is_testnet,
        })
    }

    pub fn gen_random(rng: &mut RngCore, network: &Network) -> PubKey {
        let pk = [
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u64(),
        ];

        PubKey { data: pk, is_testnet: network.is_testnet }
    }

    pub fn to_string(&self) -> String {

        if self.is_testnet {
            format!(
                "{:016x}{:016x}{:016x}{:016x}",
                self.data[0], self.data[1], self.data[2], self.data[3]
            )
        } else {
            format!(
                "05{:016x}{:016x}{:016x}{:016x}",
                self.data[0], self.data[1], self.data[2], self.data[3]
            )
        }
    }
}