use crate::{
    fileserver_api::FileServer,
    loki::{self, LokiServer, ServiceNode},
    node_pool::NodePool,
    onions::NextHop,
};

use super::Request;

#[derive(Debug)]
pub struct OnionClient {
    node_pool: NodePool,
}

impl OnionClient {
    pub async fn init(net: &loki::Network) -> Self {
        let node_pool = NodePool::init(net).await;

        OnionClient { node_pool }
    }

    pub async fn onion_to_node(
        &mut self,
        req: Request,
        dest: ServiceNode,
    ) -> Result<String, String> {
        let target = NextHop::Node(dest);

        let payload = req.body.as_bytes();

        let path = self.node_pool.get_random_path();

        let res = crate::onions::send_onion_req(path, target, payload, 0).await;

        match res {
            Ok(res) => Ok(res),
            Err(err) => {
                eprintln!("Could not send: {}", err.message);
                Err(err.message)
            }
        }
    }

    pub async fn onion_to_server(
        &mut self,
        server: &FileServer,
        payload: serde_json::Value,
    ) -> Result<String, String> {
        let host = server.host;
        let server_key = server.pubkey;

        let target = NextHop::Server(LokiServer {
            host: host.to_owned(),
            target: "/loki/v3/lsrpc".to_owned(),
            pubkey_x25519: server_key.to_owned(),
        });

        let payload_str = payload.to_string();
        let payload = payload_str.as_bytes();

        let path = self.node_pool.get_random_path();

        let res = crate::onions::send_onion_req(path, target, payload, 0).await;

        match res {
            Ok(res) => {
                if res.len() < 3000 {
                    dbg!(&res);
                }
                Ok(res)
            }
            Err(err) => {
                eprintln!("Could not send: {}", err.message);
                Err(err.message)
            }
        }
    }
}
