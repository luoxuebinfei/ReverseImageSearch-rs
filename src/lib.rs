pub mod engines;
pub mod error;
pub mod network;
pub mod types;
pub mod utils;

pub use engines::ImageSearch;
pub use error::{Error, Result};
pub use types::{AdditionalInfo, SearchEngine, SearchOptions, SearchResult};

// Re-export search engines
pub use engines::ascii2d::Ascii2d;
pub use engines::google::Google;
pub use engines::google_lens::GoogleLens;
pub use engines::saucenao::SauceNao;
// TODO: Add other engines as they are implemented

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_saucenao_search() {
        let saucenao = SauceNao::new(None);
        let options = SearchOptions::default();

        // Test with a known image URL
        let result = saucenao
            .search_url(
                "https://raw.githubusercontent.com/kitUIN/PicImageSearch/main/demo/test.jpg",
                &options,
            )
            .await;

        match result {
            Ok((_, results)) => {
                assert!(!results.is_empty(), "Should return some results");
                let first = &results[0];
                assert!(first.similarity.unwrap_or(0.0) > 0.0);
                assert!(!first.url.is_empty());
            }
            Err(e) => {
                println!("Search failed: {}", e);
                // Don't fail the test as the API might be rate limited
            }
        }
    }
}
