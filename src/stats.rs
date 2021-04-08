use crate::{loki::Network, node_pool::NodePool};



pub async fn get_foundation_nodes_stats(net: &Network) {

    // get a list of all foundation nodes

    let mut node_pool = NodePool::init(net).await;

    node_pool.remove_non_foundation();



}