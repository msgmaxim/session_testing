use crate::{
    loki::{Network, ServiceNode},
    node_pool::NodePool,
};

use std::sync::{Arc, Mutex};

#[derive(Debug, serde::Deserialize)]
struct ServerStats {
    height: u32,
    previous_period_store_requests: u64,
    total_stored: u64,
}

fn get_stats_from_one(node: &ServiceNode) -> Result<ServerStats, ()> {
    let ctx = zmq::Context::new();

    let socket = ctx.socket(zmq::DEALER).unwrap();

    let server_key = hex::decode(&node.pubkey_x25519).unwrap();

    // let server_key = hex::decode("7d11e6664535eb50c2e42dff7d8508b948e57ad977d51bb82e560f8c2e858d36").unwrap();

    socket.set_curve_serverkey(&server_key).unwrap();

    let seckey =
        hex::decode("8b1d51b0423dc2f69e01a55bc8a1332fa1691906ab9552a0a86630abaeaf7cec").unwrap();

    let pubkey =
        hex::decode("b43bd5a03768b756584a9143f04c41a08b54f3e9f5c900a9b2f6bf60a6d9160e").unwrap();

    socket.set_curve_publickey(&pubkey).unwrap();
    socket.set_curve_secretkey(&seckey).unwrap();

    let ip = &node.public_ip;
    let port = &node.storage_lmq_port;

    let address = format!("tcp://{}:{}", ip, port);
    // let address = format!("tcp://144.76.164.202:20242");

    socket.connect(&address).expect("Could not connect");

    socket
        .send_multipart(
            ["service.get_stats".as_bytes(), "tag123".as_bytes()].iter(),
            0,
        )
        .expect("could not send over zmq");

    // loop {
    let data = socket.recv_multipart(0).expect("Could not receive");

    dbg!(String::from_utf8_lossy(&data[0]));

    if data.len() != 3 || data[0] != "REPLY".as_bytes() || data[1] != "tag123".as_bytes() {
        return Err(());
    } else {
        let stats: ServerStats = serde_json::from_slice(&data[2]).unwrap();

        return Ok(stats);
    }

    //     std::thread::sleep(std::time::Duration::from_secs(1));
    // }
}

pub async fn get_foundation_nodes_stats(net: &Network) {
    // get a list of all foundation nodes

    let mut node_pool = NodePool::init(net).await;

    let total_swarms = node_pool.swarm_count();

    println!("Total swarms: {}", total_swarms);

    node_pool.remove_non_foundation();

    // zmq binding does not expose async methods, so just run each in separate threads:

    let results: Option<Vec<Result<ServerStats, ()>>> = Some(vec![]);

    let results = Arc::new(Mutex::new(results));

    let mut handles = vec![];

    for n in node_pool.get_all_nodes() {
        let n = n.clone();
        let results_copy = results.clone();

        let handle = std::thread::spawn(move || {
            let res = get_stats_from_one(&n);
            match &mut *results_copy.lock().unwrap() {
                Some(results) => { results.push(res) },
                None => panic!("Expected results to be Some(...)"),
            }
        });

        handles.push(handle);
    }

    for h in handles {
        h.join().unwrap();
    }

    let results = results.lock().unwrap().take().unwrap();

    println!("Got results total: {}", results.len());

    let results_ok: Vec<_> = results
        .into_iter()
        .filter(|x| x.is_ok())
        .map(|x| x.unwrap())
        .collect();
    println!("Got results OK total: {}", results_ok.len());

    let stored_total = results_ok.iter().fold(0, |acc, r| acc + r.total_stored);

    println!("Total stored: {}", stored_total);

    println!("Stored on node (avg): {}", stored_total / results_ok.len() as u64);

    println!("Estimated total stored: {}", (stored_total * total_swarms as u64) / results_ok.len() as u64);
}
