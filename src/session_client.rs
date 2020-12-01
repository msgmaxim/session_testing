use crate::{http_clients::OnionClient, loki::{self, Network, ServiceNode}, sn_api};


pub struct SessionClient {
    onion_client: OnionClient,
    node_pool: Vec<ServiceNode>,
}

impl SessionClient {

    pub async fn new(net: &Network) -> Self {
        let onion_client = OnionClient::init(net).await;

        let node_pool = loki::get_n_service_nodes(0, net).await;

        SessionClient {
            onion_client,
            node_pool
        }
    }

    pub async fn store_message(&self, pk: &str, message: &[u8]) {

        // 0. Get at least one node to start with;

        let rand_node = &self.node_pool[0];

        // 1. Find the recipient

        let swarm = sn_api::get_swarm_for_pk(rand_node, pk).await;

        dbg!(swarm);

    }
}