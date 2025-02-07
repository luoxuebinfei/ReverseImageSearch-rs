use super::ImageSearch;
use crate::error::{Error, Result};
use crate::network::Network;
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use crate::utils::{bytes_to_base64, normalize_url};
use async_trait::async_trait;
use reqwest::multipart;
use scraper::{Html, Selector};

const API_URL: &str = "https://ascii2d.net";

pub struct Ascii2d {
    network: Network,
}

impl Ascii2d {
    pub fn new() -> Self {
        Self {
            network: Network::new(),
        }
    }

    fn build_client() -> Result<reqwest::Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
        );
        headers.insert(reqwest::header::CACHE_CONTROL, "max-age=0".parse().unwrap());
        headers.insert(
            reqwest::header::HeaderName::from_static("dnt"),
            "1".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("priority"),
            "u=0, i".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-ch-ua"),
            r#""Not(A:Brand";v="99", "Google Chrome";v="133", "Chromium";v="133""#
                .parse()
                .unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-ch-ua-mobile"),
            "?0".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-ch-ua-platform"),
            r#""Windows""#.parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-fetch-dest"),
            "document".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-fetch-mode"),
            "navigate".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-fetch-site"),
            "same-origin".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("sec-fetch-user"),
            "?1".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::HeaderName::from_static("upgrade-insecure-requests"),
            "1".parse().unwrap(),
        );

        let cookie_store = reqwest_cookie_store::CookieStoreMutex::default();
        let cookie_store = std::sync::Arc::new(cookie_store);

        Ok(reqwest::Client::builder()
            .default_headers(headers)
            .cookie_provider(std::sync::Arc::clone(&cookie_store))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36")
            .gzip(true)
            .deflate(true)
            .brotli(true)
            .build()?)
    }

    async fn search_with_client(
        &self,
        client: &reqwest::Client,
        url: &str,
    ) -> Result<Vec<SearchResult>> {
        // 首先访问主页获取 cookie
        client.get(API_URL).send().await?;

        // 然后进行色彩搜索
        let form = multipart::Form::new().text("uri", url.to_string());
        let response = client
            .post(&format!("{}/search/uri", API_URL))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "ASCII2D returned status code: {} - {}",
                response.status(),
                response.text().await?
            )));
        }

        let html = response.text().await?;
        let mut results = self.parse_results(&html).await?;

        // 然后进行特征搜索
        let bovw_url = format!(
            "{}/search/bovw/{}",
            API_URL,
            html.split("/bovw/")
                .nth(1)
                .unwrap_or_default()
                .split('"')
                .next()
                .unwrap_or_default()
        );

        let response = client.get(&bovw_url).send().await?;
        if response.status().is_success() {
            let html = response.text().await?;
            results.extend(self.parse_results(&html).await?);
        }

        Ok(results)
    }

    async fn parse_results(&self, html: &str) -> Result<Vec<SearchResult>> {
        let document = Html::parse_document(html);
        let item_selector = Selector::parse(".item-box").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let img_selector = Selector::parse("img").unwrap();
        let detail_selector = Selector::parse(".detail-box").unwrap();
        let hash_selector = Selector::parse(".hash").unwrap();

        let mut results = Vec::new();
        for item in document.select(&item_selector) {
            if let Some(detail_box) = item.select(&detail_selector).next() {
                let mut links = detail_box.select(&link_selector);
                let thumbnail = item
                    .select(&img_selector)
                    .next()
                    .and_then(|img| img.value().attr("src"))
                    .map(|src| format!("{}{}", API_URL, src));

                let hash = item
                    .select(&hash_selector)
                    .next()
                    .map(|h| h.text().collect::<String>())
                    .unwrap_or_default();

                // 第一个链接通常是作者链接
                let (author, author_url) = if let Some(author_link) = links.next() {
                    (
                        Some(author_link.text().collect::<String>()),
                        Some(normalize_url(
                            author_link.value().attr("href").unwrap_or_default(),
                        )?),
                    )
                } else {
                    (None, None)
                };

                // 第二个链接通常是图片链接
                let url = if let Some(source_link) = links.next() {
                    normalize_url(source_link.value().attr("href").unwrap_or_default())?
                } else {
                    continue;
                };

                let title = links
                    .next()
                    .map(|l| l.text().collect::<String>())
                    .unwrap_or_default();

                results.push(SearchResult {
                    title: Some(title),
                    url,
                    thumbnail,
                    similarity: None,
                    source: "ASCII2D".to_string(),
                    index: Some(hash),
                    additional_info: Some(AdditionalInfo {
                        author,
                        author_url,
                        source_url: None,
                        created_at: None,
                        ext_urls: vec![],
                        ..Default::default()
                    }),
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl ImageSearch for Ascii2d {
    fn name(&self) -> &'static str {
        "ASCII2D"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client()?;
        let results = self.search_with_client(&client, url).await?;
        Ok(("".to_string(), results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client()?;

        // 首先访问主页获取 cookie
        client.get(API_URL).send().await?;

        // 然后进行色彩搜索
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name("image.png")
            .mime_str("image/png")?;
        let form = multipart::Form::new().part("file", part);

        let response = client
            .post(&format!("{}/search/file", API_URL))
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "ASCII2D returned status code: {} - {}",
                response.status(),
                response.text().await?
            )));
        }

        let html = response.text().await?;
        let mut results = self.parse_results(&html).await?;

        // 然后进行特征搜索
        let bovw_url = format!(
            "{}/search/bovw/{}",
            API_URL,
            html.split("/bovw/")
                .nth(1)
                .unwrap_or_default()
                .split('"')
                .next()
                .unwrap_or_default()
        );

        let response = client.get(&bovw_url).send().await?;
        if response.status().is_success() {
            let html = response.text().await?;
            results.extend(self.parse_results(&html).await?);
        }

        Ok(("".to_string(), results))
    }

    async fn search_base64(
        &self,
        base64: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let image_data = crate::utils::base64_to_bytes(base64)?;
        self.search_bytes(&image_data, options).await
    }
}
