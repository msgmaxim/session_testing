use async_trait::async_trait;

mod clearnet_client;
mod onion_client;

pub use clearnet_client::ClearnetClient;
pub use onion_client::OnionClient;

pub struct Request {
    pub url: String,
    pub method: String,
    pub body: String,
}

#[async_trait]
pub trait HttpClient {
    async fn send(&mut self, req: Request) -> Result<String, String>;
}
