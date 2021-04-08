use std::{
    convert::TryInto,
    sync::Arc,
    time::{Duration, SystemTime},
};

use rand::{
    prelude::{SliceRandom, StdRng},
    SeedableRng,
};
use serde_json::json;

use parking_lot::Mutex;

use crate::{
    fileserver_api,
    fileserver_api::DEV_FILESERVER,
    loki::{self, Network},
    loki::{LokiServer, ServiceNode},
    node_pool::NodePool,
    onions::NextHop,
    onions::{send_onion_req, OnionPath},
    session_server_client::FileServerInterface,
    session_server_client::{OpenGroupInterface, SessionServerClient},
    sn_api,
    swarm_mapping::SwarmMapping,
};

fn sleep_ms(millis: u64) {
    std::thread::sleep(std::time::Duration::from_millis(millis));
}

fn target_message(rng: &mut StdRng, network: &Network) -> String {
    let pk = loki::PubKey::gen_random(rng, &network);

    let params = json!({
        "method": "get_snodes_for_pubkey",
        "params": {
            "pubKey": &pk.to_string(),
        }
    });

    params.to_string()
}

fn store_message(pk: &str) -> String {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        * 1000;

    let ttl: u64 = 60_000; // ms

    let data = "TODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODOTODO";

    let nonce = crate::proof_of_work::compute_nonce(timestamp, ttl, &pk, &data);

    let params = json!({
        "method": "store",
        "params": {
            "pubKey": &pk,
            "ttl": format!("{}", ttl),
            "nonce": &base64::encode(&nonce),
            "timestamp": format!("{}", timestamp),
            "data": data,
        }
    });

    let payload = params.to_string();

    payload
}

fn get_file(file: &str) -> String {
    let endpoint = format!("loki/v1/f/{}", file);

    let token = "loki";

    let auth_header = format!("Bearer {}", token);

    let payload = serde_json::json!({
        "method": "GET",
        "body": "",
        "headers": {"Authorization": auth_header}, // might have Authorization header
        "endpoint": endpoint
    });

    payload.to_string()
}

async fn fileserver_task(net: &loki::Network) -> Duration {
    let mut server_client = SessionServerClient::init(net, &fileserver_api::DEV_FILESERVER)
        .await
        .expect("Could not create Filserver client");

    // let file_name = "rv1ru9"; // dev.lokinet.org
    // let file_name = "na97ow"; // chat.getsession.org
    let file_name = "qot36t"; // chat-dev.lokinet.org

    let tp = std::time::Instant::now();

    match server_client.get_file(file_name).await {
        Ok(file) => {
            let bin_len = base64::decode(&file).expect("not base64").len();
            println!("file size base64: {}, binary: {}", file.len(), bin_len);
        }
        Err(err) => {
            eprintln!("Could not get file: {}", err);
        }
    }

    tp.elapsed()
}

async fn get_messages_task(net: &loki::Network) -> Duration {
    let mut server_client = SessionServerClient::init(net, &fileserver_api::DEV_OPEN_GROUP_SERVER)
        .await
        .expect("Could not create Filserver client");

    let tp = std::time::Instant::now();

    match server_client.get_messages().await {
        Ok(messages) => {
            // let bin_len = base64::decode(&file).expect("not base64").len();
            for m in messages {
                let m: serde_json::Value = serde_json::from_str(&m).unwrap();
                println!("{} ", m.get("id").unwrap());
            }
        }
        Err(err) => {
            eprintln!("Could not get file: {}", err);
        }
    }

    tp.elapsed()
}

pub async fn test_fileserver_requests(net: &loki::Network) {

    let mut tasks = vec![];

    let count = 1;

    for _ in 0..count {
        let task = get_messages_task(net);
        tasks.push(task);
    }

    let durations = futures::future::join_all(tasks).await;

    let total_duration: u128 = durations.iter().map(|dur| dur.as_millis()).sum();

    let average_ms = total_duration / count;

    println!("Average: {} ms", average_ms);
}

async fn test_onion_path(context: Arc<Mutex<TestContext>>, idx: u64) {
    // let mut context_lock = context.lock();

    let mut nodes = context.lock().node_pool.get_random_nodes(4);

    nodes.pop().unwrap();

    let path: [_; 3] = nodes.try_into().unwrap();

    let (pk, nodes) = context.lock().swarm_mapping.get_one();

    // let mut rng = rand::thread_rng();

    // let mut rng = StdRng::seed_from_u64(idx);

    // let target = nodes.choose(&mut rng).unwrap().to_owned();

    // let target = NextHop::Node(target);

    let target = NextHop::Server(LokiServer {
        host: DEV_FILESERVER.host.to_owned(),
        target: "/loki/v3/lsrpc".to_owned(),
        pubkey_x25519: DEV_FILESERVER.pubkey.to_owned(),
    });

    // let payload = store_message(&pk);

    // let file = "005yfe"; // smallets file (no problem)
    // let file = "we2c37"; // smaller 86% failure rate
    let file = "qot36t"; // ~5 mb (99% failure rate)
    let payload = get_file(&file);

    // std::mem::drop(context_lock);

    let time_now = std::time::Instant::now();

    let payload = payload.as_bytes();

    let res = send_onion_req(path, target, payload, idx).await;

    let res = match res {
        Ok(res) => {
            // if res.len() != 1_318_040 {
            println!("Success, len: {}", res.len());
            // }

            OnionTestResult {
                success: true,
                time: time_now.elapsed(),
                path: None,
            }
        }
        Err(onion_err) => {
            eprintln!("[{}] error: {}", idx, onion_err.message);
            OnionTestResult {
                success: false,
                time: time_now.elapsed(),
                path: Some(onion_err.path),
            }
        }
    };

    context.lock().results.push(res);
}

struct OnionTestResult {
    pub success: bool,
    pub time: std::time::Duration,
    path: Option<OnionPath>,
}

struct TestContext {
    node_pool: NodePool,
    results: Vec<OnionTestResult>,
    network: Network,
    swarm_mapping: SwarmMapping,
}

pub async fn test_onion_requests() {
    // Make n onion requests selecting nodes randomly

    let net = &loki::MAINNET;

    let n = 50;
    const N_PARALLEL: usize = 50;

    let mut last_idx = 0;
    // let net = &loki::LOCAL_NET;

    let mut node_pool = NodePool::init(net).await;

    // node_pool.remove_non_foundation();

    // node_pool.truncate(50);

    let node = &node_pool.get_random_nodes(1)[0];

    let clients = SwarmMapping::init(node).await;

    println!("Session clients are initialized");

    let context = TestContext {
        node_pool,
        results: vec![],
        network: net.to_owned(),
        swarm_mapping: clients,
    };

    let context = Arc::new(Mutex::new(context));

    let context2 = Arc::clone(&context);

    let context = context2;

    let mut prev_executed = 0;

    loop {
        let n_executed = context.lock().results.len();

        if prev_executed != n_executed {
            if n_executed % 100 == 0 {
                println!("Executed: {}/{}", n_executed, n);
            }
            prev_executed = n_executed;
        }
        let n_in_flight = last_idx - n_executed;

        if n_in_flight < N_PARALLEL {
            last_idx += 1;

            let context = Arc::clone(&context);

            tokio::spawn(async move { test_onion_path(context, last_idx as u64).await });
        }

        if last_idx == n as usize {
            break;
        }

        sleep_ms(1);
    }

    let mut results_so_far = 0;

    loop {
        let context = context.lock();

        let results = &context.results;

        let failed = results.iter().filter(|res| !res.success).count();

        if results.len() != results_so_far {
            results_so_far = results.len();
            println!("Failures: {}/{}", failed, results.len());
        }

        if results.len() == n as usize {
            break;
        }

        std::mem::drop(context);

        sleep_ms(1000);
    }

    // Print stats about which nodes failed

    print_stats(context.clone());

    // test_individual_nodes(context.clone()).await;
}

async fn test_node(node: &NextHop, network: &Network) {
    if let NextHop::Node(node) = node {
        let mut rng = StdRng::seed_from_u64(0);

        let pk = loki::PubKey::gen_random(&mut rng, network).to_string();

        let res = sn_api::get_swarm_for_pk(node, &pk).await;

        match res {
            Ok(_res) => {
                println!("{}: OK", node);
            }
            Err(err) => {
                println!("{}: {}", node, err);
            }
        }
    }
}

async fn test_individual_nodes(context: Arc<Mutex<TestContext>>) {
    let context = context.lock();

    let failed_results = context.results.iter().filter(|res| !res.success);

    for res in failed_results {
        let path = res.path.as_ref().expect("No path on error");
        let nodes = [&path.node_1, &path.node_2, &path.node_3, &path.target];

        println!(
            "Failed path: [{}]->[{}]->[{}]->[{}]",
            &nodes[0], &nodes[1], &nodes[2], &nodes[3]
        );

        for node in &nodes {
            test_node(node, &context.network).await;
        }
    }
}

fn print_stats(context: Arc<Mutex<TestContext>>) {
    use std::collections::HashMap;

    let mut failed_nodes = HashMap::<String, u32>::new();

    let context = context.lock();

    for res in &context.results {
        if !res.success {
            let path = res.path.as_ref().expect("No path on error");
            let nodes = [&path.node_1, &path.node_2, &path.node_3, &path.target];

            for n in &nodes {
                match n {
                    NextHop::Node(n) => {
                        let entry = failed_nodes
                            .entry(n.service_node_pubkey.clone())
                            .or_insert(0);
                        *entry += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    let mut failed_nodes = failed_nodes
        .into_iter()
        .map(|(k, v)| (k, v))
        .collect::<Vec<_>>();

    failed_nodes.sort_by(|a, b| a.1.cmp(&b.1));

    for (key, failures) in failed_nodes {
        println!("{}: {}", key, failures);
    }
}
