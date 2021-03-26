use fileserver_api::FileServer;
use rand::{prelude::StdRng, SeedableRng};

use http_clients::{HttpClient, OnionClient, Request};
use session_client::SessionClient;

mod ecdh;
mod fileserver_api;
mod loki;
mod node_pool;
mod onions;
mod onions_core;
mod proof_of_work;
mod session_client;
mod session_server_client;
mod sn_api;
mod swarm_mapping;
mod tests;

mod http_clients;

mod server;

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct ServeOptions {
    #[structopt(short = "p", long = "port", default_value = "8000")]
    port: u16,
}

#[derive(Debug, StructOpt)]
enum Commands {
    Serve(ServeOptions),
    Fileserver,
    Basic,
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let opt = Commands::from_args();

    let network = loki::MAINNET;

    match opt {
        Commands::Serve(options) => {
            println!("Starting a testing server...");
            server::start(network, options).await;
        }
        Commands::Fileserver => {
            println!("Running fileserver tests");
            tests::test_fileserver_requests().await;
        }
        Commands::Basic => {
            println!("Running basic tests");
        }
    }

    let endpoint = format!("loki/v1/f/abcde");

    let payload = serde_json::json!({
        "method": "GET",
        "body": "",
        "endpoint": endpoint
    });

    let network = loki::LOCAL_NET;

    let mut client = OnionClient::init(&network).await;

    // let server : FileServer = FileServer {
    //     host: "https://chat-dev.lokinet.org",
    //     pubkey: "c5c256d1e1b32f8e20e05b05c47b8ea435b667fb571392db2c6c4f6b1ccf9422",
    // };

    let server : FileServer = FileServer {
        host: "127.0.0.1",
        pubkey: "c5c256d1e1b32f8e20e05b05c47b8ea435b667fb571392db2c6c4f6b1ccf9422",
    };

    let res = client
        .onion_to_server(&server, payload)
        .await
        .map_err(|_| "Could not send").expect("could not send");

    println!("`res` size: {}", res.len());



    return;

    // let token = fileserver_api::get_token(&fileserver_api::DEV_FILESERVER).await.expect("Failed to get token");
    let token = "";

    // dbg!(&token);

    // return;

    // let mut rng = StdRng::seed_from_u64(0);
    let network = loki::LOCAL_NET;

    // let pk = loki::PubKey::gen_random(&mut rng, &network);

    // let nodes = sn_api::get_swarm_for_pk(&node_pool[0], &pk.to_string()).await;

    // dbg!(&nodes);

    // fileserver_api::upload_file_via_onion(node_pool.clone(), &fileserver_api::DEV_FILESERVER).await;

    // This is the file that we Audric and I couldn't download from Session Desktop
    let file = "npoiwi";

    let mut client = OnionClient::init(&network).await;

    // let res = fileserver_api::get_file_via_onion(&mut client, &fileserver_api::DEV_OPEN_GROUP_SERVER, &token, file).await;

    // dbg!(&res);

    // test_session_clients().await;

    // test_clearnet_requests().await;

    // tests::test_onion_requests().await;
}

async fn test_session_clients() {
    // Generate a session identity and send a message to it

    // let alice = SessionClient::new_identity();

    let network = loki::LOCAL_NET;

    let mut rng = StdRng::seed_from_u64(0);

    let pk = loki::PubKey::gen_random(&mut rng, &network);

    let client = SessionClient::new(&network).await;

    let data = vec![1, 2, 3];

    client.store_message(&pk.to_string(), &data).await;
}
