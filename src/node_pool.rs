use std::collections::HashSet;

use rand::{
    prelude::{SliceRandom, StdRng},
    SeedableRng,
};

use crate::loki::{self, ServiceNode};

#[derive(Debug)]
pub struct NodePool {
    node_pool: Vec<ServiceNode>,
    rng: StdRng,
}

impl NodePool {
    /// Initialize from network's seed
    pub async fn init(net: &loki::Network) -> Self {
        let node_pool = loki::get_n_service_nodes(0, net)
            .await
            .expect("Could not initialize node pool");

        // println!("Node pool: {:#?}", node_pool);

        let rng = StdRng::seed_from_u64(0);

        NodePool { node_pool, rng }
    }

    pub fn remove_non_foundation(&mut self) {
        println!("Nodes total: {}", self.node_pool.len());

        self.node_pool.retain(|n| n.operator_address == "LDoptfyQB3YHbS9cnt2wHdTTj2wtZGPuM48evCFwZpomVajQw4eJ6mDCpXeUNTxsqbTiYytnqEDQNin3XGwp3nReMooMaWG");

        println!("Foundation nodes: {}", self.node_pool.len());
    }

    pub fn truncate(&mut self, len: usize) {
        self.node_pool.truncate(len);
    }

    pub fn get_random_path(&mut self) -> [ServiceNode; 3] {
        let mut iter = self.node_pool.choose_multiple(&mut self.rng, 3).cloned();

        [
            iter.next().unwrap(),
            iter.next().unwrap(),
            iter.next().unwrap(),
        ]
    }

    pub fn get_random_nodes(&mut self, n: usize) -> Vec<ServiceNode> {
        self.node_pool
            .choose_multiple(&mut self.rng, n)
            .cloned()
            .collect()
    }

    pub fn get_all_nodes(&self) -> &Vec<ServiceNode> {
        &self.node_pool
    }

    pub fn swarm_count(&self) -> usize {
        let mut swarms : HashSet<u64> = HashSet::new();

        for n in &self.node_pool {
            swarms.insert(n.swarm_id);
        }

        swarms.len()
    }
}
