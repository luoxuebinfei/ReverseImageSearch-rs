use super::ImageSearch;
use crate::error::{Error, Result};
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use async_trait::async_trait;
use regex;
use reqwest::multipart;
use scraper::{Html, Selector};
use serde_json::Value;

const API_URL: &str = "https://lens.google.com";

pub struct GoogleLens {}

impl GoogleLens {
    pub fn new() -> Self {
        Self {}
    }

    fn build_client(redirect: bool) -> Result<reqwest::Client> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:103.0) Gecko/20100101 Firefox/103.0"
                .parse()
                .unwrap(),
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

        let mut builder = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_store(true);

        if !redirect {
            builder = builder.redirect(reqwest::redirect::Policy::none());
        }

        builder.build().map_err(Error::from)
    }

    fn get_prerender_script(&self, html: &str) -> Result<Value> {
        let document = Html::parse_document(html);
        let script_selector = Selector::parse("script").unwrap();

        // 找到包含数据的脚本
        let script_text = document
            .select(&script_selector)
            .find(|script| {
                let text = script.text().collect::<String>();
                text.contains("AF_initDataCallback")
                    || text.contains("(function(){var m=")
                    || text.contains("key: 'ds:1'")
            })
            .map(|script| script.text().collect::<String>())
            .ok_or_else(|| Error::Engine("未找到包含数据的脚本".to_string()))?;

        // 尝试提取数据
        if let Some(start_index) = script_text.find("(function(){var m=") {
            let js_text = &script_text[start_index..];
            let js_text = js_text.replace("(function(){var m=", "");

            // 移除尾部的 window 脚本
            let js_text = if let Some(end_index) = js_text.find(";window.") {
                &js_text[..end_index]
            } else {
                &js_text
            };

            // 清理尾部
            let js_text = js_text.trim_end_matches("});");
            let js_text = js_text
                .split("};")
                .next()
                .map(|s| format!("{}}}", s))
                .ok_or_else(|| Error::Engine("无法提取有效的 JSON 对象".to_string()))?;

            serde_json::from_str(&js_text)
                .map_err(|e| Error::Engine(format!("JSON 解析失败: {}", e)))
        } else if let Some(start_index) = script_text.find("AF_initDataCallback") {
            let js_text = &script_text[start_index..];
            if let Some(data_start) = js_text.find("data:") {
                let js_text = &js_text[data_start + 5..];
                if let Some(end_index) = js_text.find("sideChannel:") {
                    let data_text = js_text[..end_index]
                        .trim()
                        .trim_end_matches(',')
                        .to_string();
                    serde_json::from_str(&data_text)
                        .map_err(|e| Error::Engine(format!("JSON 解析失败: {}", e)))
                } else {
                    Err(Error::Engine("无法找到数据结束位置".to_string()))
                }
            } else if let Some(data_start) = js_text.find("[[") {
                let js_text = &js_text[data_start..];
                if let Some(end_index) = js_text.find("]]") {
                    let data_text = format!("{}", &js_text[..end_index + 2]);
                    serde_json::from_str(&data_text)
                        .map_err(|e| Error::Engine(format!("JSON 解析失败: {}", e)))
                } else {
                    Err(Error::Engine("无法找到数据结束位置".to_string()))
                }
            } else {
                Err(Error::Engine("无法找到数据起始位置".to_string()))
            }
        } else {
            Err(Error::Engine("未找到有效的数据格式".to_string()))
        }
    }

    fn parse_prerender_script(&self, prerender_script: Value) -> Result<Value> {
        let mut data = serde_json::json!({
            "match": null,
            "similar": []
        });

        // 尝试提取最佳匹配
        if let Some(best_match) = prerender_script
            .get(0)
            .and_then(|v| v.get(1))
            .and_then(|v| v.get(8))
            .and_then(|v| v.get(12))
            .and_then(|v| v.get(0))
            .and_then(|v| v.get(0))
        {
            if let Ok(match_data) = (|| -> Result<Value> {
                Ok(serde_json::json!({
                    "title": best_match[0],
                    "thumbnail": best_match[2][0][0],
                    "pageURL": best_match[2][0][4]
                }))
            })() {
                data["match"] = match_data;
            }
        }

        // 获取视觉匹配结果
        let visual_matches = if data["match"] != serde_json::Value::Null {
            prerender_script
                .get(1)
                .and_then(|v| v.get(1))
                .and_then(|v| v.get(8))
                .and_then(|v| v.get(8))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get(12))
        } else {
            prerender_script
                .get(0)
                .and_then(|v| v.get(1))
                .and_then(|v| v.get(8))
                .and_then(|v| v.get(8))
                .and_then(|v| v.get(0))
                .and_then(|v| v.get(12))
        };

        // 处理相似结果
        if let Some(matches) = visual_matches.and_then(|v| v.as_array()) {
            let similar = matches
                .iter()
                .map(|match_item| {
                    let thumbnail = match_item
                        .get(0)
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    let price = match_item
                        .get(0)
                        .and_then(|v| v.get(7))
                        .and_then(|v| v.get(1))
                        .and_then(|v| v.as_str())
                        .map(|p| {
                            regex::Regex::new(r"[^\d.]")
                                .unwrap()
                                .replace_all(p, "")
                                .to_string()
                        });

                    let currency = match_item
                        .get(0)
                        .and_then(|v| v.get(7))
                        .and_then(|v| v.get(5))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    serde_json::json!({
                        "title": match_item.get(3),
                        "similarity score": match_item.get(1),
                        "thumbnail": thumbnail,
                        "pageURL": match_item.get(5),
                        "sourceWebsite": match_item.get(14),
                        "price": price,
                        "currency": currency
                    })
                })
                .collect::<Vec<_>>();

            data["similar"] = serde_json::Value::Array(similar);
        }

        Ok(data)
    }
}

#[async_trait]
impl ImageSearch for GoogleLens {
    fn name(&self) -> &'static str {
        "Google Lens"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client(true)?;
        let search_url = format!("{}/uploadbyurl?url={}&hl=en&gl=us", API_URL, url);

        let response = client
            .get(&search_url)
            .header(reqwest::header::REFERER, API_URL)
            .send()
            .await?;

        let html = response.text().await?;

        // 检查是否被重定向到验证页面
        if html.contains("Our systems have detected unusual traffic") {
            return Err(Error::Engine("Google 要求验证，请稍后再试".to_string()));
        }

        let prerender_script = self.get_prerender_script(&html)?;
        let data = self.parse_prerender_script(prerender_script)?;

        let mut results = Vec::new();

        // 添加最佳匹配
        if let Some(best_match) = data.get("match") {
            results.push(SearchResult {
                title: best_match
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                url: best_match
                    .get("pageURL")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default(),
                thumbnail: best_match
                    .get("thumbnail")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                similarity: Some(100.0),
                source: "Google Lens".to_string(),
                index: None,
                additional_info: Some(AdditionalInfo::default()),
            });
        }

        // 添加相似结果
        if let Some(similar) = data.get("similar").and_then(|v| v.as_array()) {
            for item in similar {
                let mut additional_info = AdditionalInfo::default();

                if let Some(website) = item.get("sourceWebsite").and_then(|v| v.as_str()) {
                    additional_info.source_url = Some(website.to_string());
                }

                if let Some(price) = item.get("price").and_then(|v| v.as_str()) {
                    additional_info.tags.push(format!("价格: {}", price));
                }

                if let Some(currency) = item.get("currency").and_then(|v| v.as_str()) {
                    additional_info.tags.push(format!("货币: {}", currency));
                }

                results.push(SearchResult {
                    title: item.get("title").and_then(|v| v.as_str()).map(String::from),
                    url: item
                        .get("pageURL")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                        .unwrap_or_default(),
                    thumbnail: item
                        .get("thumbnail")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    similarity: item
                        .get("similarity score")
                        .and_then(|v| v.as_f64())
                        .map(|v| v as f32),
                    source: "Google Lens".to_string(),
                    index: None,
                    additional_info: Some(additional_info),
                });
            }
        }

        Ok((search_url, results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let client = Self::build_client(false)?;

        // 先访问主页获取初始 cookie
        client.get(API_URL).send().await?;

        // 构建 multipart 表单
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name("image.jpg")
            .mime_str("image/jpeg")?;

        let form = multipart::Form::new()
            .part("encoded_image", part)
            .text("image_content", "");

        // 发送上传请求
        let upload_url = format!("{}/upload", API_URL);
        let response = client
            .post(&upload_url)
            .query(&[("hl", "en"), ("gl", "us")])
            .header(reqwest::header::REFERER, API_URL)
            .multipart(form)
            .send()
            .await?;

        // 检查是否是 302 重定向
        if response.status() == reqwest::StatusCode::FOUND {
            if let Some(location) = response.headers().get(reqwest::header::LOCATION) {
                let search_url = location
                    .to_str()
                    .map_err(|e| Error::Engine(e.to_string()))?;

                // 添加时间戳和浏览器尺寸参数
                let search_url = if search_url.contains('?') {
                    format!(
                        "{}&qsubts={}&biw=1920&bih=911",
                        search_url,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    )
                } else {
                    format!(
                        "{}?qsubts={}&biw=1920&bih=911",
                        search_url,
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_millis()
                    )
                };

                // 跟随重定向
                let response = client
                    .get(&search_url)
                    .header(reqwest::header::REFERER, upload_url)
                    .header(
                        "sec-ch-ua",
                        r#""Not(A:Brand";v="99", "Google Chrome";v="133", "Chromium";v="133""#,
                    )
                    .header("sec-ch-ua-mobile", "?0")
                    .header("sec-ch-ua-platform", "Windows")
                    .header("sec-fetch-dest", "document")
                    .header("sec-fetch-mode", "navigate")
                    .header("sec-fetch-site", "same-origin")
                    .header("sec-fetch-user", "?1")
                    .header("upgrade-insecure-requests", "1")
                    .send()
                    .await?;

                let html = response.text().await?;

                // 检查是否被重定向到验证页面
                if html.contains("Our systems have detected unusual traffic") {
                    return Err(Error::Engine("Google 要求验证，请稍后再试".to_string()));
                }

                let prerender_script = self.get_prerender_script(&html)?;
                let data = self.parse_prerender_script(prerender_script)?;

                // 处理搜索结果
                let mut results = Vec::new();

                // 添加最佳匹配
                if let Some(best_match) = data.get("match") {
                    results.push(SearchResult {
                        title: best_match
                            .get("title")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        url: best_match
                            .get("pageURL")
                            .and_then(|v| v.as_str())
                            .map(String::from)
                            .unwrap_or_default(),
                        thumbnail: best_match
                            .get("thumbnail")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                        similarity: Some(100.0),
                        source: "Google Lens".to_string(),
                        index: None,
                        additional_info: Some(AdditionalInfo::default()),
                    });
                }

                // 添加相似结果
                if let Some(similar) = data.get("similar").and_then(|v| v.as_array()) {
                    for item in similar {
                        let mut additional_info = AdditionalInfo::default();

                        if let Some(website) = item.get("sourceWebsite").and_then(|v| v.as_str()) {
                            additional_info.source_url = Some(website.to_string());
                        }

                        if let Some(price) = item.get("price").and_then(|v| v.as_str()) {
                            additional_info.tags.push(format!("价格: {}", price));
                        }

                        if let Some(currency) = item.get("currency").and_then(|v| v.as_str()) {
                            additional_info.tags.push(format!("货币: {}", currency));
                        }

                        results.push(SearchResult {
                            title: item.get("title").and_then(|v| v.as_str()).map(String::from),
                            url: item
                                .get("pageURL")
                                .and_then(|v| v.as_str())
                                .map(String::from)
                                .unwrap_or_default(),
                            thumbnail: item
                                .get("thumbnail")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            similarity: item
                                .get("similarity score")
                                .and_then(|v| v.as_f64())
                                .map(|v| v as f32),
                            source: "Google Lens".to_string(),
                            index: None,
                            additional_info: Some(additional_info),
                        });
                    }
                }

                Ok((search_url, results))
            } else {
                Err(Error::Engine("重定向响应中缺少 Location 头".to_string()))
            }
        } else {
            // 如果不是 302 重定向，返回错误
            let status = response.status();
            let text = response.text().await?;
            Err(Error::Engine(format!(
                "上传失败: 期望 302 重定向，但收到 {} - {}",
                status, text
            )))
        }
    }

    async fn search_base64(
        &self,
        base64: &str,
        options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        let bytes = crate::utils::base64_to_bytes(base64)?;
        self.search_bytes(&bytes, options).await
    }
}
