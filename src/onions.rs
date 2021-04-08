use std::fmt;

use rand::prelude::*;

use crate::{loki::{LokiServer, LokiServerV2, ServiceNode}, sn_api};

pub trait HasX25519 {
    fn pubkey_x25519(&self) -> String;
}

impl<'a> HasX25519 for &'a ServiceNode {
    fn pubkey_x25519(&self) -> String {
        self.pubkey_x25519.clone()
    }
}

impl<'a> HasX25519 for &'a LokiServer {
    fn pubkey_x25519(&self) -> String {
        self.pubkey_x25519.clone()
    }
}

impl<'a> HasX25519 for &'a LokiServerV2 {
    fn pubkey_x25519(&self) -> String {
        self.pubkey_x25519.clone()
    }
}

#[derive(Debug, Clone)]
pub enum NextHop {
    Node(ServiceNode),
    Server(LokiServer),
    /// server entry that includes the port and the protocol to use
    ServerV2(LokiServerV2),
}

impl fmt::Display for NextHop {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // port is most useful when testing locally, might change this for mainnet/testnet
        match self {
            NextHop::Node(node) => write!(f, "{}", node),
            NextHop::Server(server) => write!(f, "{}", server),
            NextHop::ServerV2(server) => write!(f, "{}", server),
        }
    }
}

pub struct OnionPath {
    pub node_1: NextHop,
    pub node_2: NextHop,
    pub node_3: NextHop,
    pub target: NextHop,
}

pub struct OnionError {
    pub message: String,
    pub path: OnionPath,
}

#[derive(serde::Deserialize)]
struct OnionResponse {
    body: String,
    status: u32,
}

pub async fn send_onion_req(
    node_path: [ServiceNode; 3],
    target: NextHop,
    payload: &[u8],
    i: u64,
) -> Result<String, OnionError> {
    let [n1, n2, n3] = node_path;

    let guard_pubkey = &n1.service_node_pubkey;

    // println!(
    //     "[{}] Building {} ({}) -> {} -> {} -> {}",
    //     i, n1, guard_pubkey, &n2, &n3, &target
    // );

    let node_1 = NextHop::Node(n1);
    let node_2 = NextHop::Node(n2);
    let node_3 = NextHop::Node(n3);

    let path = OnionPath {
        node_1,
        node_2,
        node_3,
        target,
    };

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .expect("building reqwest client");

    let res = sn_api::onion_request_v2(&client, &path, &payload).await;

    let res = res.map_err(|err| OnionError { message: err, path })?;

    let res: OnionResponse = serde_json::from_str(&res).expect("Not a valid onion response");

    Ok(res.body)
}

// Theories that I want to test:

// - Memory leaks in service nodes
// - All errors are actually Fileserver/Open Group server related
// - Most other errors are due to a small number of failing nodes (and proper node management should address them)

// What I want to see:

// - Success rate (does it depend on the message type?)
// - Success rate sending to nodes
// - Success rate sending to Loki Servers
// - Success rate without path management
// - Success rate with path management
// - State diagrams and errors from nodes running as part of onion requests

// TODO:

// - Node pool management (assign a bad score for to nodes participating in bad requests)
// - When a path fails, test nodes individually to see what actually happened
// - Path management (select a few paths that work and stick to them)

// TODO until Tuesday:

// - Have a report on regular requests to nodes
// - Have a report on onion requests w/o node management
// - Have a report on onion requests with node management

// Ideas:

// - I could use my local nodes as relays to mainnet nodes and have a closer look at them
// - After I've identified onion requests failures, tests the nodes on failed paths directly
// - A longer tests (running for over an hour) potentially with reduced request frequency
