use super::ImageSearch;
use crate::error::{Error, Result};
use crate::network::Network;
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use crate::utils::{base64_to_bytes, bytes_to_base64, normalize_url};
use async_trait::async_trait;
use serde::Deserialize;

const API_URL: &str = "https://saucenao.com/search.php";

pub struct SauceNao {
    network: Network,
    api_key: Option<String>,
}

impl SauceNao {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            network: Network::new(),
            api_key,
        }
    }
}

#[derive(Debug, Deserialize)]
struct SauceNaoResponse {
    header: Header,
    results: Option<Vec<ResultItem>>,
}

#[derive(Debug, Deserialize)]
struct Header {
    status: i32,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultItem {
    header: ResultHeader,
    data: ResultData,
}

#[derive(Debug, Deserialize)]
struct ResultHeader {
    similarity: String,
    thumbnail: String,
    index_id: i32,
    index_name: String,
}

#[derive(Debug, Deserialize)]
struct ResultData {
    ext_urls: Option<Vec<String>>,
    title: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    source: Option<String>,
    created_at: Option<String>,
}

#[async_trait]
impl ImageSearch for SauceNao {
    fn name(&self) -> &'static str {
        "SauceNAO"
    }

    async fn search_url(
        &self,
        url: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let mut params = vec![
            ("url", url.to_string()),
            ("output_type", "2".to_string()), // JSON output
            ("numres", "16".to_string()),
        ];

        if let Some(ref api_key) = self.api_key {
            params.push(("api_key", api_key.clone()));
        }

        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Safari/537.36")
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.insert(reqwest::header::ACCEPT, "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".parse().unwrap());
                headers.insert(reqwest::header::ACCEPT_LANGUAGE, "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap());
                headers.insert(reqwest::header::REFERER, "https://saucenao.com/".parse().unwrap());
                headers
            })
            .build()?;

        let search_url = format!("{}?url={}", API_URL, url);

        let response = client
            .get(&format!(
                "{}?{}",
                API_URL,
                url::form_urlencoded::Serializer::new(String::new())
                    .extend_pairs(params)
                    .finish()
            ))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "SauceNAO returned status code: {}",
                response.status()
            )));
        }

        let sauce_response: SauceNaoResponse = response.json().await?;

        if sauce_response.header.status < 0 {
            return Err(Error::Engine(
                sauce_response
                    .header
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let results = sauce_response.results.unwrap_or_default();
        let min_similarity = options.min_similarity.unwrap_or(0.0);

        let results = results
            .into_iter()
            .filter(|item| item.header.similarity.parse::<f32>().unwrap_or(0.0) >= min_similarity)
            .map(|item| {
                let similarity = item.header.similarity.parse::<f32>().unwrap_or(0.0);
                let mut urls = item.data.ext_urls.unwrap_or_default();
                let source = item.data.source.clone();
                let url = if urls.is_empty() {
                    source.clone().unwrap_or_default()
                } else {
                    urls.remove(0)
                };

                SearchResult {
                    title: item.data.title,
                    url: normalize_url(&url).unwrap_or(url),
                    thumbnail: Some(item.header.thumbnail),
                    similarity: Some(similarity),
                    source: item.header.index_name,
                    index: Some(item.header.index_id.to_string()),
                    additional_info: Some(AdditionalInfo {
                        author: item.data.author_name,
                        author_url: item.data.author_url,
                        source_url: source,
                        created_at: item.data.created_at,
                        ext_urls: urls,
                        ..Default::default()
                    }),
                }
            })
            .collect();

        Ok((search_url, results))
    }

    async fn search_base64(
        &self,
        base64: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let mut form = reqwest::multipart::Form::new()
            .text("output_type", "2")
            .text("numres", "16")
            .text("api_key", self.api_key.clone().unwrap_or_default())
            .text("dbmask", "999") // 使用所有数据库
            .text("minsim", options.min_similarity.unwrap_or(80.0).to_string());

        // 将 base64 转换回二进制
        let image_data = base64_to_bytes(base64)?;

        // 添加文件部分
        let part = reqwest::multipart::Part::bytes(image_data)
            .file_name("image.png")
            .mime_str("image/png")?;
        form = form.part("file", part);

        let response = self.network.post_multipart(API_URL, form).await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "SauceNAO returned status code: {} - {}",
                response.status(),
                response.text().await?
            )));
        }

        let sauce_response: SauceNaoResponse = response.json().await?;

        if sauce_response.header.status < 0 {
            return Err(Error::Engine(
                sauce_response
                    .header
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let results = sauce_response.results.unwrap_or_default();
        let min_similarity = options.min_similarity.unwrap_or(0.0);

        let results = results
            .into_iter()
            .filter(|item| item.header.similarity.parse::<f32>().unwrap_or(0.0) >= min_similarity)
            .map(|item| {
                let similarity = item.header.similarity.parse::<f32>().unwrap_or(0.0);
                let mut urls = item.data.ext_urls.unwrap_or_default();
                let source = item.data.source.clone();
                let url = if urls.is_empty() {
                    source.clone().unwrap_or_default()
                } else {
                    urls.remove(0)
                };

                SearchResult {
                    title: item.data.title,
                    url: normalize_url(&url).unwrap_or(url),
                    thumbnail: Some(item.header.thumbnail),
                    similarity: Some(similarity),
                    source: item.header.index_name,
                    index: Some(item.header.index_id.to_string()),
                    additional_info: Some(AdditionalInfo {
                        author: item.data.author_name,
                        author_url: item.data.author_url,
                        source_url: source,
                        created_at: item.data.created_at,
                        ext_urls: urls,
                        ..Default::default()
                    }),
                }
            })
            .collect();

        Ok(("".to_string(), results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        self.search_base64(&bytes_to_base64(bytes), options).await
    }

    async fn search_file(
        &self,
        file_path: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let mut form = reqwest::multipart::Form::new()
            .text("output_type", "2")
            .text("numres", "16")
            .text("api_key", self.api_key.clone().unwrap_or_default())
            .text("dbmask", "999") // 使用所有数据库
            .text("minsim", options.min_similarity.unwrap_or(80.0).to_string());

        // 添加文件部分
        let part = reqwest::multipart::Part::bytes(tokio::fs::read(file_path).await?)
            .file_name("image.png")
            .mime_str("image/png")?;
        form = form.part("file", part);

        let response = self.network.post_multipart(API_URL, form).await?;
        // 打印response的url
        println!("response的url: {}", response.url());

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "SauceNAO returned status code: {} - {}",
                response.status(),
                response.text().await?
            )));
        }

        let sauce_response: SauceNaoResponse = response.json().await?;

        if sauce_response.header.status < 0 {
            return Err(Error::Engine(
                sauce_response
                    .header
                    .message
                    .unwrap_or_else(|| "Unknown error".to_string()),
            ));
        }

        let results = sauce_response.results.unwrap_or_default();
        let min_similarity = options.min_similarity.unwrap_or(0.0);

        let results = results
            .into_iter()
            .filter(|item| item.header.similarity.parse::<f32>().unwrap_or(0.0) >= min_similarity)
            .map(|item| {
                let similarity = item.header.similarity.parse::<f32>().unwrap_or(0.0);
                let mut urls = item.data.ext_urls.unwrap_or_default();
                let source = item.data.source.clone();
                let url = if urls.is_empty() {
                    source.clone().unwrap_or_default()
                } else {
                    urls.remove(0)
                };

                SearchResult {
                    title: item.data.title,
                    url: normalize_url(&url).unwrap_or(url),
                    thumbnail: Some(item.header.thumbnail),
                    similarity: Some(similarity),
                    source: item.header.index_name,
                    index: Some(item.header.index_id.to_string()),
                    additional_info: Some(AdditionalInfo {
                        author: item.data.author_name,
                        author_url: item.data.author_url,
                        source_url: source,
                        created_at: item.data.created_at,
                        ext_urls: urls,
                        ..Default::default()
                    }),
                }
            })
            .collect();

        Ok(("".to_string(), results))
    }
}
