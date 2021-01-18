use std::{
    convert::TryInto,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime},
};

use parking_lot::RwLock;
use rand::{
    prelude::{SliceRandom, ThreadRng},
    thread_rng, RngCore,
};

use crate::{
    loki::{self, Network, ServiceNode},
    onions::{send_onion_req, NextHop},
    ServeOptions,
};

use futures::join;

use log::{error, info, trace, warn};

use rouille::router;

use serde::Serialize;

#[derive(Debug)]
struct OnionResult {
    time: std::time::SystemTime,
    success: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
struct OnionResultAggregated {
    time: std::time::SystemTime,
    total: u32,
    total_success: u32,
}

// struct CircularBuffer<T> {
//     buffer: Vec<T>,
//     // At what index to insert the next element
//     index: usize,
// }

// impl<T> CircularBuffer<T> {

//     pub fn new(n: usize) -> Self {
//         CircularBuffer {
//             buffer: Vec::<T>::with_capacity(n),
//             index: 0
//         }
//     }

//     pub fn push(&mut self, x: T) {
//         if index ==
//     }

// }

const BUFFER_LIMIT: usize = 720;

#[derive(Debug)]
/// Note the use of a double buffer
struct OnionResults {
    recent_results: Vec<OnionResult>,
    results_old: Vec<OnionResultAggregated>,
    results_new: Vec<OnionResultAggregated>,
}

impl OnionResults {
    pub(crate) fn new() -> Self {
        OnionResults {
            recent_results: vec![],
            results_old: vec![],
            results_new: vec![],
        }
    }

    pub(crate) fn push(&mut self, res: OnionResult) {
        self.recent_results.push(res);
    }

    pub(crate) fn aggregate(&mut self) {
        let results = &mut self.recent_results;

        if results.is_empty() {
            warn!("No results to aggregate");
            return;
        }

        let first = results.first().unwrap().time;
        let last = results.last().unwrap().time;

        let dur = last.duration_since(first).expect("Could not subtract time");

        info!(
            "Aggregating {} results, duration: {}",
            results.len(),
            dur.as_secs()
        );

        let total_success = results.iter().filter(|x| x.success).count() as u32;

        let agg_res = OnionResultAggregated {
            time: last,
            total: results.len() as u32,
            total_success,
        };

        results.clear();

        if self.results_new.len() == BUFFER_LIMIT {
            std::mem::swap(&mut self.results_old, &mut self.results_new);
            self.results_new.clear();
        }

        self.results_new.push(agg_res);

        assert!(self.results_new.len() <= BUFFER_LIMIT);
    }
}

struct Context {
    net: Network,
    node_pool: Vec<ServiceNode>,
    onion_results: OnionResults,
}

impl Context {
    pub fn new(net: Network) -> Self {
        Context {
            node_pool: vec![],
            net,
            onion_results: OnionResults::new(),
        }
    }
}

pub async fn start(net: Network, options: ServeOptions) {
    std::panic::set_hook(Box::new(|msg| {
        error!("Panicked with: {}", msg);
        std::process::exit(101); // Rust's panics use 101 by default
    }));

    let ctx = Context::new(net);

    let ctx = Arc::new(RwLock::new(ctx));

    // TODO: Start the http server

    let ctx_clone = ctx.clone();

    let _task = tokio::task::spawn_blocking(move || {
        serve_http(ctx_clone, options);
    });

    // TODO: Initiate periodic testing of nodes

    let fut2 = start_testing(ctx);

    join!(fut2);
}

fn serve_http(ctx: Arc<RwLock<Context>>, options: ServeOptions) {
    let port: u16 = options.port;

    let address = format!("0.0.0.0:{}", port);

    println!("Serving http on: {}", port);

    rouille::start_server(&address, move |req| {
        router!(req,

            (GET) (/data) => {

                let results = &ctx.read().onion_results;

                let mut old = results.results_old.clone();
                let mut new = results.results_new.clone();

                old.append(&mut new);

                // let res = serde_json::to_string(&old);

                let res = rouille::Response::json(&old);

                res.with_additional_header("Access-Control-Allow-Origin", "*")
            },
            _ => {
                let response = rouille::match_assets(&req, "./html");

                if response.is_success() {
                    return response.with_additional_header("Access-Control-Allow-Origin", "*");
                }

                return rouille::Response::text("404 error").with_status_code(404);
            }
        )
    });
}

async fn periodically_refresh_node_pool(ctx: Arc<RwLock<Context>>) {
    const PERIOD: Duration = Duration::from_secs(600);

    let net = ctx.read().net.clone();

    loop {
        match loki::get_n_service_nodes(0, &net).await {
            Ok(nodes) => {
                println!("Updated nodes, count: {}", nodes.len());
                ctx.write().node_pool = nodes;
            }
            Err(err) => {
                eprintln!("Failed to update nodes from seed: {}", err);
            }
        }

        async_std::task::sleep(PERIOD).await;
    }
}

fn test_payload(rng: &mut ThreadRng, network: &Network) -> String {
    let pk = loki::PubKey::gen_random(rng, &network);

    let params = serde_json::json!({
        "method": "get_snodes_for_pubkey",
        "params": {
            "pubKey": &pk.to_string(),
        }
    });

    params.to_string()
}

async fn onion_req_task(ctx: Arc<RwLock<Context>>) -> bool {
    let mut nodes: Vec<_> = {
        let mut rng = rand::thread_rng();

        ctx.read()
            .node_pool
            .choose_multiple(&mut rng, 4)
            .cloned()
            .collect()
    };

    let target: ServiceNode = nodes.pop().expect("Node should exist");

    let path: [_; 3] = nodes.try_into().unwrap();

    trace!(
        "Testing [{} -> {} -> {}] -> {}",
        &path[0],
        &path[1],
        &path[2],
        &target
    );

    let target = NextHop::Node(target);

    let payload = {
        let mut rng = rand::thread_rng();

        test_payload(&mut rng, &ctx.read().net)
    };

    send_onion_req(path, target, payload.as_bytes(), 0)
        .await
        .is_ok()
}

async fn run_onion_req_task(ctx: Arc<RwLock<Context>>, in_flight: Arc<Mutex<u32>>) {
    let success = onion_req_task(ctx.clone()).await;

    *in_flight.lock().unwrap() -= 1;

    let res = OnionResult {
        success,
        time: SystemTime::now(),
    };

    ctx.write().onion_results.push(res);
}

async fn sleep_ms(n: u64) {
    async_std::task::sleep(std::time::Duration::from_millis(n)).await;
}

async fn onion_request_testing(ctx: Arc<RwLock<Context>>) {
    // How many parallel tests allowed
    const MAX_IN_FLIGHT: u32 = 10;

    let in_flight = Arc::new(Mutex::new(0u32));

    loop {
        if ctx.read().node_pool.len() == 0 {
            info!("Node pool is empty, skipping this iteration");
            sleep_ms(1000).await;
            continue;
        }

        if *in_flight.lock().unwrap() >= MAX_IN_FLIGHT {
            trace!("Too many requests already in flight, skipping this iteration");
            sleep_ms(1000).await;
            continue;
        }

        *in_flight.lock().unwrap() += 1;

        let ctx_clone = ctx.clone();
        let in_flight_clone = in_flight.clone();

        tokio::spawn(async move { run_onion_req_task(ctx_clone, in_flight_clone).await });
    }
}

async fn aggregate_results(ctx: Arc<RwLock<Context>>) {
    loop {
        ctx.write().onion_results.aggregate();

        // Run every minute
        sleep_ms(60_000).await;
    }
}

async fn start_testing(ctx: Arc<RwLock<Context>>) {
    let fut = periodically_refresh_node_pool(ctx.clone());

    let fut2 = onion_request_testing(ctx.clone());

    let fut3 = aggregate_results(ctx);

    join!(fut, fut2, fut3);

    // periodically update node pool
}
