use crate::error::Result;
use bytes::Bytes;
use log::debug;
use reqwest::header::HeaderMap;
use reqwest::{Client, Response};
use std::time::Duration;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36";

#[derive(Clone)]
pub struct Network {
    client: Client,
}

impl std::fmt::Debug for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Network").finish()
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::new()
    }
}

impl Network {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent(DEFAULT_USER_AGENT)
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }

    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    pub async fn get(&self, url: &str) -> Result<Response> {
        debug!("GET request to: {}", url);
        Ok(self.client.get(url).send().await?)
    }

    pub async fn get_with_headers(&self, url: &str, headers: HeaderMap) -> Result<Response> {
        debug!("GET request to: {} with headers: {:?}", url, headers);
        Ok(self.client.get(url).headers(headers).send().await?)
    }

    pub async fn post(&self, url: &str, body: Vec<u8>) -> Result<Response> {
        debug!("POST request to: {}", url);
        Ok(self.client.post(url).body(body).send().await?)
    }

    pub async fn post_json<T: serde::Serialize>(&self, url: &str, json: &T) -> Result<Response> {
        debug!("POST JSON request to: {}", url);
        Ok(self.client.post(url).json(json).send().await?)
    }

    pub async fn get_bytes(&self, url: &str) -> Result<Bytes> {
        Ok(self.get(url).await?.bytes().await?)
    }

    pub async fn post_multipart(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
    ) -> Result<Response> {
        debug!("POST multipart request to: {}", url);
        Ok(self.client.post(url).multipart(form).send().await?)
    }

    pub async fn post_multipart_with_headers(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
        headers: HeaderMap,
    ) -> Result<Response> {
        debug!(
            "POST multipart request to: {} with headers: {:?}",
            url, headers
        );
        Ok(self
            .client
            .post(url)
            .headers(headers)
            .multipart(form)
            .send()
            .await?)
    }

    pub fn set_proxy(&mut self, proxy_url: &str) -> Result<()> {
        self.client = Client::builder()
            .timeout(DEFAULT_TIMEOUT)
            .user_agent(DEFAULT_USER_AGENT)
            .proxy(reqwest::Proxy::all(proxy_url)?)
            .build()?;
        Ok(())
    }
}
