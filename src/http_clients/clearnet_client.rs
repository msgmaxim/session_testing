use super::{HttpClient, Request};

use async_trait::async_trait;

pub struct ClearnetClient {
    client: reqwest::Client,
}

impl ClearnetClient {
    pub fn new() -> Self {
        // TODO: I might want to limit this to 
        // snode requests only
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .expect("building reqwest client");
        ClearnetClient { client }
    }
}

#[async_trait]
impl HttpClient for ClearnetClient {
    async fn send(&mut self, req: Request) -> Result<String, String> {
        assert_eq!(req.method, "POST");

        let res = self
            .client
            .post(&req.url)
            .body(req.body)
            .send()
            .await
            .map_err(|err| err.to_string())?;

        res.text().await.map_err(|err| err.to_string())
    }
}
