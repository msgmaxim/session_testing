use std::collections::HashMap;

use futures::future::join_all;
use rand::{SeedableRng, prelude::{SliceRandom, StdRng}, thread_rng};

use crate::{loki::{self, ServiceNode}, sn_api};



/// Maps session clients to swarms
pub struct SwarmMapping {
    client_pks: Vec<String>,
    swarm_mapping: HashMap<String, Vec<ServiceNode>>
}

async fn inner_task(node: &ServiceNode, pk: String) -> (String, Vec<ServiceNode>) {

    let nodes = sn_api::get_swarm_for_pk(node, &pk.to_owned()).await.expect("Could not get pk for node");

    (pk, nodes)
}

impl SwarmMapping {

    pub async fn init(node: &ServiceNode) -> Self {

        let mut rng = StdRng::seed_from_u64(0);

        let n: usize = 100;

        let mut tasks = vec![];

        for i in 0..n {

            let pk = loki::PubKey::gen_random(&mut rng, &loki::MAINNET).to_string();

            let task = inner_task(&node, pk.to_owned());
            tasks.push(task);
        }

        let res = join_all(tasks).await;

        let mut client_pks = Vec::new();
        let mut swarm_mapping = HashMap::new();
        for (pk, nodes) in res {

            client_pks.push(pk.clone());
            swarm_mapping.insert(pk, nodes);

        }


        SwarmMapping {
            client_pks,
            swarm_mapping
        }
    }

    pub fn get_one(&self) -> (String, Vec<ServiceNode>) {

        let mut rng = thread_rng();

        let pk = self.client_pks.choose(&mut rng).unwrap().to_owned();

        let nodes = self.swarm_mapping.get(&pk).unwrap();

        (pk, nodes.to_owned())
    }


}

