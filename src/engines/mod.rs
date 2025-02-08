use crate::error::Result;
use crate::types::{SearchOptions, SearchResult};
use async_trait::async_trait;

#[async_trait]
pub trait ImageSearch: Send + Sync {
    /// Get the name of the search engine
    fn name(&self) -> &'static str;

    /// Search for an image using its URL
    async fn search_url(
        &self,
        url: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)>;

    /// Search for an image using a local file path
    async fn search_file(
        &self,
        file_path: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = tokio::fs::read(file_path).await?;
        self.search_bytes(&bytes, options).await
    }

    /// Search for an image using raw bytes
    async fn search_bytes(
        &self,
        _bytes: &[u8],
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        Ok(("".to_string(), self.search_url("", options).await?.1))
    }

    /// Search for an image using base64 encoded data
    async fn search_base64(
        &self,
        base64: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = crate::utils::base64_to_bytes(base64)?;
        self.search_bytes(&bytes, options).await
    }
}

pub mod ascii2d;
pub mod google;
pub mod google_lens;
pub mod iqdb;
pub mod saucenao;
pub mod soutubot;
pub mod yandex;
// pub mod baidu;
// pub mod bing;
// pub mod ehentai;
// pub mod tineye;
// pub mod tracemoe;

pub use ascii2d::Ascii2d;
pub use google::Google;
pub use google_lens::GoogleLens;
pub use iqdb::Iqdb;
pub use saucenao::SauceNao;
pub use soutubot::Soutubot;
pub use yandex::Yandex;
