use async_trait::async_trait;

mod onion_client;
mod clearnet_client;

pub use onion_client::OnionClient;
pub use clearnet_client::ClearnetClient;


pub struct Request {
    pub url: String,
    pub method: String,
    pub body: String,
}

#[async_trait]
pub trait HttpClient {

    async fn send(&mut self, req: Request) -> Result<String, String>;
}