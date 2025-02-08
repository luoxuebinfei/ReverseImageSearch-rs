use crate::error::Error;
use crate::error::Result;
use crate::types::{SearchOptions, SearchResult};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use reqwest::header::HeaderValue;
use reqwest::multipart::{Form, Part};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Soutubot {
    client: reqwest::Client,
}

impl Soutubot {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl super::ImageSearch for Soutubot {
    fn name(&self) -> &'static str {
        "Soutubot"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = reqwest::get(url).await?.bytes().await?;
        self.search_bytes(&bytes, _options).await
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let results = search(&self.client, bytes).await?;
        Ok(results)
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct SoutubotResponse {
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: String,
    #[serde(default)]
    data: Vec<SoutubotResult>,
    #[serde(rename = "executionTime", default)]
    execution_time: f32,
    #[serde(rename = "imageUrl", default)]
    image_url: String,
    #[serde(rename = "searchOption", default)]
    search_option: String,
    #[serde(rename = "id", default)]
    id: String,
}



#[derive(Debug, Serialize, Deserialize)]
struct SoutubotResult {
    #[serde(default)]
    similarity: f32,
    #[serde(default)]
    title: String,
    #[serde(rename = "previewImageUrl", default)]
    preview_image_url: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    language: String,
    #[serde(rename = "subjectPath", default)]
    subject_path: String,
    #[serde(rename = "pagePath", default)]
    page_path: Option<String>,
}

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36";

fn apikey(ua: &str) -> String {
    // 获取四舍五入后的当前 UNIX 时间戳
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
        .round() as i128;

    // 计算时间戳组成部分
    let t = now.pow(2);
    let ua_len = ua.len() as i128;
    let ua_square = ua_len.pow(2);
    let mut timestamp = t + ua_square + 4746193387776_i128;

    // 对齐到百位
    timestamp -= timestamp % 100;

    // Base64 编码处理
    let timestamp_str = timestamp.to_string();
    let b64_str = general_purpose::STANDARD.encode(timestamp_str.as_bytes());

    // 去除尾部等号并反转字符串
    let stripped = b64_str.trim_end_matches('=');
    let reversed: String = stripped.chars().rev().collect();

    reversed
}

pub async fn search(client: &reqwest::Client, image_bytes: &[u8]) -> Result<(String, Vec<SearchResult>)> {
    // 生成 API key
    let api_key = apikey(USER_AGENT);

    // 构建请求头
    let headers = {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("sec-ch-ua", HeaderValue::from_static(USER_AGENT));
        headers.insert("dnt", HeaderValue::from_static("1"));
        headers.insert("sec-ch-ua-mobile", HeaderValue::from_static("?0"));
        headers.insert(
            "accept",
            HeaderValue::from_static("application/json, text/plain, */*"),
        );
        headers.insert(
            "x-requested-with",
            HeaderValue::from_static("XMLHttpRequest"),
        );
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&api_key).map_err(|e| Error::InvalidResponse(e.to_string()))?,
        );
        headers.insert(
            "sec-ch-ua-platform",
            HeaderValue::from_static("\"Windows\""),
        );
        headers.insert("origin", HeaderValue::from_static("https://soutubot.moe"));
        headers.insert("sec-fetch-site", HeaderValue::from_static("same-origin"));
        headers.insert("sec-fetch-mode", HeaderValue::from_static("cors"));
        headers.insert("sec-fetch-dest", HeaderValue::from_static("empty"));
        headers.insert("referer", HeaderValue::from_static("https://soutubot.moe/"));
        headers.insert(
            "accept-language",
            HeaderValue::from_static("zh-CN,zh;q=0.9"),
        );
        headers
    };

    // 构建 multipart form
    let form = Form::new().text("factor", "1.2").part(
        "file",
        Part::bytes(image_bytes.to_vec())
            .file_name("image")
            .mime_str("application/octet-stream")?,
    );

    // 发送请求
    let response = client
        .post("https://soutubot.moe/api/search")
        .headers(headers)
        .multipart(form)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(Error::InvalidResponse(format!("HTTP {}", status)).into());
    }

    let soutubot_response: SoutubotResponse = response.json().await?;
    // json格式 {"data":[],"id":"2025020814015112","factor":1.4,"imageUrl":"https:\/\/img.76888268.xyz\/img\/8abba7d56ebab7885b2a68cf0d57c742.webp","searchOption":"api 1.4 Liner 64","executionTime":2.6}
    // 结果页链接
    let result_page_url = format!("https://soutubot.moe/results/{}", soutubot_response.id);

    // 转换结果
    let results = soutubot_response
        .data
        .into_iter()
        .map(|result| {
            let detail_url = if result.source == "nhentai" {
                format!("https://www.{}.net{}", result.source, result.subject_path)
            } else {
                format!(
                    "https://e-hentai.org{}\nhttps://exhentai.org{}",
                    result.subject_path, result.subject_path
                )
            };

            SearchResult {
                title: Some(result.title),
                url: detail_url,
                thumbnail: Some(result.preview_image_url),
                similarity: Some(result.similarity),
                source: result.source,
                index: None,
                additional_info: None,
            }
        })
        .collect();

    Ok((result_page_url, results))
}
