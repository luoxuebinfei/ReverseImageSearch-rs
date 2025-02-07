use super::ImageSearch;
use crate::error::{Error, Result};
use crate::network::Network;
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::multipart;
use scraper::{Html, Selector};

const API_URL: &str = "https://iqdb.org";

pub struct Iqdb {
    network: Network,
}

impl Iqdb {
    pub fn new() -> Self {
        Self {
            network: Network::new(),
        }
    }

    async fn parse_results(&self, html: &str) -> Result<Vec<SearchResult>> {
        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // 匹配结果表格
        let tables_selector = Selector::parse("#pages > div > table").unwrap();
        let tables: Vec<_> = document.select(&tables_selector).collect();

        // 跳过第一个表格（上传的图片）
        if tables.len() <= 1 {
            return Ok(results);
        }

        for table in tables.iter().skip(1) {
            let mut tr_list: Vec<_> = table.select(&Selector::parse("tr").unwrap()).collect();

            // 检查是否有匹配结果
            let content = tr_list[0]
                .select(&Selector::parse("th").unwrap())
                .next()
                .map(|th| th.text().collect::<String>());

            if content.as_deref() == Some("No relevant matches") {
                continue;
            }

            // 如果有 th，需要跳过第一行
            if content.is_some() {
                tr_list = tr_list[1..].to_vec();
            }

            if tr_list.len() < 4 {
                continue;
            }

            // 提取 URL 和缩略图
            let url = tr_list[0]
                .select(&Selector::parse("td > a").unwrap())
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|href| {
                    if href.starts_with("//") {
                        format!("https:{}", href)
                    } else {
                        href.to_string()
                    }
                });

            let thumbnail = tr_list[0]
                .select(&Selector::parse("td > a > img").unwrap())
                .next()
                .and_then(|img| img.value().attr("src"))
                .map(|src| format!("https://iqdb.org{}", src));

            // 提取来源
            let source = tr_list[1]
                .select(&Selector::parse("img").unwrap())
                .next()
                .and_then(|img| img.next_sibling())
                .and_then(|text| text.value().as_text())
                .map(|text| text.trim().to_string())
                .unwrap_or_default();

            // 提取尺寸
            let size = tr_list[2]
                .select(&Selector::parse("td").unwrap())
                .next()
                .map(|td| td.text().collect::<String>())
                .unwrap_or_default();

            // 提取相似度
            let similarity = tr_list[3]
                .select(&Selector::parse("td").unwrap())
                .next()
                .map(|td| td.text().collect::<String>())
                .and_then(|text| {
                    text.strip_suffix("% similarity")
                        .and_then(|s| s.parse::<f32>().ok())
                        .map(|n| n / 100.0)
                });

            if let Some(url) = url {
                results.push(SearchResult {
                    title: Some(format!("[{}] {}", source, size)),
                    url,
                    thumbnail,
                    similarity,
                    source: "IQDB".to_string(),
                    index: Some(results.len().to_string()),
                    additional_info: Some(AdditionalInfo::default()),
                });
            }
        }

        Ok(results)
    }
}

#[async_trait]
impl ImageSearch for Iqdb {
    fn name(&self) -> &'static str {
        "IQDB"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        // 构建表单数据
        let form = multipart::Form::new().text("url", url.to_string());

        // 发送请求
        let response = self.network.post_multipart(API_URL, form).await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "IQDB returned status code: {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let results = self.parse_results(&html).await?;

        Ok(("".to_string(), results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        // 构建 multipart 表单
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name("image.jpg")
            .mime_str("image/jpeg")?;

        let form = multipart::Form::new().part("file", part);

        // 发送请求
        let response = self.network.post_multipart(API_URL, form).await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "上传失败: 状态码 {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let results = self.parse_results(&html).await?;

        Ok(("".to_string(), results))
    }

    async fn search_file(
        &self,
        file_path: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = tokio::fs::read(file_path).await?;
        self.search_bytes(&bytes, options).await
    }

    async fn search_base64(
        &self,
        base64: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = STANDARD.decode(base64)?;
        self.search_bytes(&bytes, options).await
    }
}
