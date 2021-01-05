use crate::{
    fileserver_api::{self, FileServer},
    http_clients::OnionClient,
    loki,
};

use async_trait::async_trait;

#[derive(Debug)]
pub struct SessionServerClient {
    onion_client: OnionClient,
    server: FileServer,
    token: String,
}

#[async_trait]
pub trait FileServerInterface {
    async fn get_file(&mut self, file: &str) -> Result<String, String>;
}

#[async_trait]
pub trait OpenGroupInterface {
    async fn get_messages(&mut self) -> Result<Vec<String>, String>;
}

impl SessionServerClient {
    pub async fn init(net: &loki::Network, server: &FileServer) -> Result<Self, ()> {
        let onion_client = OnionClient::init(net).await;

        // TODO: use onions for this

        let token = fileserver_api::get_token(server).await.map_err(|_| {
            eprintln!("Could not obtain server token");
            ()
        })?;

        Ok(SessionServerClient {
            onion_client,
            server: server.clone(),
            token,
        })
    }
}

fn get_file_payload(file: &str, token: &str) -> serde_json::Value {
    let endpoint = format!("loki/v1/f/{}", file);

    let auth_header = format!("Bearer {}", token);

    let payload = serde_json::json!({
        "method": "GET",
        "body": "",
        "headers": {"Authorization": auth_header}, // might have Authorization header
        "endpoint": endpoint
    });

    payload
}

fn get_messages_payload(token: &str) -> serde_json::Value {
    let endpoint = "channels/1/messages?count=5&since_id=6426";

    let auth_header = format!("Bearer {}", token);

    let payload = serde_json::json!({
        "method": "GET",
        "body": "",
        "headers": {"Authorization": auth_header}, // might have Authorization header
        "endpoint": endpoint
    });

    payload
}

#[async_trait]
impl OpenGroupInterface for SessionServerClient {
    async fn get_messages(&mut self) -> Result<Vec<String>, String> {
        let server = &self.server;

        let payload = get_messages_payload(&self.token);

        let res = self.onion_client.onion_to_server(server, payload).await?;

        let res : serde_json::Value = serde_json::from_str(&res).map_err(|_| "Not valid json".to_owned())?;

        let data = res.get("data").ok_or("No data field".to_owned())?;

        let data = data.as_array().ok_or("Data is not array")?;

        Ok(data.into_iter().map(|x| x.to_string()).collect())
    }
}

async fn get_file_clearnet(file: &str, host: &str, token: &str) -> Result<String, String> {

    let client = reqwest::Client::new();

    let endpoint = format!("loki/v1/f/{}", file);

    let url = format!("https://{}/{}", host, endpoint);

    let auth_header = format!("Bearer {}", token);

    let res = client
        .get(&url)
        .header("Authorization", auth_header)
        .send()
        .await
        .map_err(|_| "Could not send request".to_owned())?;

    let body = res
        .text()
        .await
        .map_err(|_| "Could not get body".to_owned())?;

    let body = Vec::from(body.as_bytes());

    let body = String::from_utf8(body).map_err(|_| "Not utf-8".to_owned())?;

    return Ok(base64::encode(&body));

}

#[async_trait]
impl FileServerInterface for SessionServerClient {
    async fn get_file(&mut self, file: &str) -> Result<String, String> {

        // return get_file_clearnet(file, self.server.host, &self.token).await;

        let server = &self.server;

        let payload = get_file_payload(file, &self.token);

        let res = self.onion_client.onion_to_server(server, payload).await;

        res.map_err(|err| err)
    }
}
