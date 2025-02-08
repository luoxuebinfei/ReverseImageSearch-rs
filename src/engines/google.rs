use super::ImageSearch;
use crate::error::{Error, Result};
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use async_trait::async_trait;
use regex;
use reqwest::multipart;
use scraper::{Html, Selector};

const API_URL: &str = "https://www.google.com";

#[derive(Debug)]
pub struct GoogleResponse {
    pub results: Vec<SearchResult>,
    pub pages: Vec<String>,
    pub current_page: usize,
    pub url: String,
}

pub struct Google {}

impl Google {
    pub fn new() -> Self {
        Self {}
    }

    fn build_client() -> Result<reqwest::Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36".parse().unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"
                .parse()
                .unwrap(),
        );
        headers.insert(
            reqwest::header::ACCEPT_LANGUAGE,
            "zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7".parse().unwrap(),
        );

        Ok(reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()?)
    }

    async fn search_with_client(
        &self,
        client: &reqwest::Client,
        url: &str,
    ) -> Result<GoogleResponse> {
        // 构建搜索请求
        let search_url = format!("{}/searchbyimage?&image_url={}&client=Chrome", API_URL, url);
        println!("搜索 URL: {}", search_url);

        let response = client
            .get(&search_url)
            .header(reqwest::header::REFERER, API_URL)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "Google returned status code: {}",
                response.status()
            )));
        }

        let html = response.text().await?;

        // 检查是否被重定向到验证页面
        if html.contains("Our systems have detected unusual traffic") {
            return Err(Error::Engine("Google 要求验证，请稍后再试".to_string()));
        }

        // 尝试解析结果
        let mut response = self.parse_response(&html, &search_url, 1).await?;

        // 如果没有找到缩略图，尝试再次请求
        if response.results.is_empty() || response.results.iter().all(|r| r.thumbnail.is_none()) {
            let response_retry = client.get(&search_url).send().await?;
            if response_retry.status().is_success() {
                response = self
                    .parse_response(&response_retry.text().await?, &search_url, 1)
                    .await?;
            }
        }

        Ok(response)
    }

    async fn parse_response(&self, html: &str, url: &str, page: usize) -> Result<GoogleResponse> {
        let document = Html::parse_document(html);

        // 匹配分页链接
        let page_selector = Selector::parse("a[aria-label~=\"Page\"]").unwrap();
        let mut pages: Vec<String> = document
            .select(&page_selector)
            .filter_map(|el| el.value().attr("href"))
            .map(|href| format!("https://www.google.com{}", href))
            .collect();
        pages.insert(0, url.to_string());

        // 匹配原项目的选择器
        let results_selector = Selector::parse("#search .g").unwrap();
        let title_selector = Selector::parse("h3").unwrap();
        let link_selector = Selector::parse("a").unwrap();
        let img_selector = Selector::parse("img[id^='dimg_']").unwrap();

        // 先尝试提取缩略图
        let mut thumbnail_dict = std::collections::HashMap::new();
        let script_selector = Selector::parse("script").unwrap();
        let base64_regex =
            regex::Regex::new("data:image/(?:jpeg|jpg|png|gif);base64,[^'\"]+").unwrap();
        let id_regex = regex::Regex::new("dimg_[^'\"]+").unwrap();

        for script in document.select(&script_selector) {
            let text = script.text().collect::<String>();
            if let Some(base64_match) = base64_regex.find(&text) {
                let base64 = base64_match.as_str();
                for id_match in id_regex.find_iter(&text) {
                    let id = id_match.as_str();
                    thumbnail_dict.insert(id.to_string(), base64.replace(r"\x3d", "="));
                }
            }
        }

        let mut results = Vec::new();
        for (index, item) in document.select(&results_selector).enumerate() {
            let title = item
                .select(&title_selector)
                .next()
                .map(|el| el.text().collect::<String>());

            let url = item
                .select(&link_selector)
                .next()
                .and_then(|el| el.value().attr("href"))
                .map(ToOwned::to_owned)
                .unwrap_or_default();

            // 从缩略图字典中获取图片
            let thumbnail = item
                .select(&img_selector)
                .next()
                .and_then(|el| el.value().attr("id"))
                .and_then(|id| thumbnail_dict.get(id))
                .cloned();

            if !url.is_empty() {
                results.push(SearchResult {
                    title,
                    url,
                    thumbnail,
                    similarity: None,
                    source: "Google".to_string(),
                    index: Some(index.to_string()),
                    additional_info: Some(AdditionalInfo::default()),
                });
            }
        }

        println!("找到 {} 个结果", results.len());
        Ok(GoogleResponse {
            results,
            pages,
            current_page: page,
            url: url.to_string(),
        })
    }

    pub async fn next_page(&self, response: &GoogleResponse) -> Result<Option<GoogleResponse>> {
        let next_page = response.current_page + 1;
        if next_page > response.pages.len() {
            return Ok(None);
        }

        let client = Self::build_client()?;
        let next_url = &response.pages[next_page - 1];
        let resp = client.get(next_url).send().await?;

        if resp.status().is_success() {
            let parsed = self
                .parse_response(&resp.text().await?, next_url, next_page)
                .await?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }

    pub async fn prev_page(&self, response: &GoogleResponse) -> Result<Option<GoogleResponse>> {
        if response.current_page <= 1 {
            return Ok(None);
        }

        let prev_page = response.current_page - 1;
        let client = Self::build_client()?;
        let prev_url = &response.pages[prev_page - 1];
        let resp = client.get(prev_url).send().await?;

        if resp.status().is_success() {
            let parsed = self
                .parse_response(&resp.text().await?, prev_url, prev_page)
                .await?;
            Ok(Some(parsed))
        } else {
            Ok(None)
        }
    }
}

#[async_trait]
impl ImageSearch for Google {
    fn name(&self) -> &'static str {
        "Google"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client()?;
        let response = self.search_with_client(&client, url).await?;
        Ok((response.url, response.results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client()?;

        // 先访问主页获取 cookie
        client.get(API_URL).send().await?;

        // 构建 multipart 表单
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name("image.jpg")
            .mime_str("image/jpeg")?;

        let form = multipart::Form::new()
            .part("encoded_image", part)
            .text("image_content", "");

        // 发送上传请求
        let response = client
            .post(&format!("{}/searchbyimage/upload", API_URL))
            .query(&[("hl", "en"), ("gl", "us")])
            .header(reqwest::header::REFERER, API_URL)
            .multipart(form)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "上传失败: 状态码 {}",
                response.status()
            )));
        }

        let search_url = response.url().to_string();
        let google_response = self
            .parse_response(&response.text().await?, &search_url, 1)
            .await?;
        Ok((google_response.url, google_response.results))
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
        let data_url = format!("data:image/jpeg;base64,{}", base64);
        self.search_url(&data_url, options).await
    }
}
