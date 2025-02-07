use super::ImageSearch;
use crate::error::{Error, Result};
use crate::network::Network;
use crate::types::{AdditionalInfo, SearchOptions, SearchResult};
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use reqwest::header::{
    HeaderMap, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, COOKIE, DNT, REFERER,
    USER_AGENT,
};
use reqwest::multipart;
use scraper::{Html, Selector};
use serde_json::Value;
use std::fs;

#[derive(Debug)]
pub struct Yandex {
    network: Network,
    base_url: String,
}

impl Default for Yandex {
    fn default() -> Self {
        Self::new("https://yandex.com")
    }
}

impl Yandex {
    pub fn new(base_url: &str) -> Self {
        Self {
            network: Network::new(),
            base_url: format!("{}/images/search", base_url),
        }
    }

    fn build_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_static(
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36",
            ),
        );
        headers.insert(REFERER, HeaderValue::from_static("https://yandex.com"));
        headers.insert(
            ACCEPT,
            HeaderValue::from_static(
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
            ),
        );
        headers.insert(
            ACCEPT_LANGUAGE,
            HeaderValue::from_static("zh-CN,zh;q=0.9,en-US;q=0.8,en;q=0.7"),
        );
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("max-age=0"));
        headers.insert(DNT, HeaderValue::from_static("1"));

        // 添加 Cookie
        headers.insert(
            COOKIE,
            HeaderValue::from_static("is_gdpr=0; is_gdpr_b=CNfYHxD8pwIoAg==; yandex_login=luoxuebinfei; yandexuid=4469646701730833589; yashr=6420963171730833589; receive-cookie-deprecation=1; L=XRZKBXsBTEZ6V1hfCVtFXWV7T1RkRWR2IjMLLxcvKwVWJRIZ.1732387245.15960.360833.41b26fb256271d6bdf81822b8adc46d5; font_loaded=YSv1; bltsr=1; KIykI=1; my=YwA=; ys=udn.cDpsdW94dWViaW5mZWk=#wprid.1737530593637953-16302891694983095392-balancer-l7leveler-kubr-yp-klg-199-BAL; Session_id=3:1737619595.5.0.1732387245930:T8puwQ:1282.1.2:1|1354280184.0.2.0:3.3:1732387245|11:10188134.641159.Tb8kijOidztkex2hHPhF_TNHF2Y; sessar=1.1198.CiBdPpK-v4JFFpRRA9W6sg8W1axG5bs0WdCvvrO3yYFRjA.4cOAuee_Z0_mxTwoDMGq4bFodZSLLkAWolyPzUW0hUs; sessionid2=3:1737619595.5.0.1732387245930:T8puwQ:1282.1.2:1|1354280184.0.2.0:3.3:1732387245|11:10188134.641159.fakesign0000000000000000000; i=0W8pwEtmL2fNP4Bfo+45Ac+dWaP0jNp/od8VwK+z7PoweyG3wGuuglpaTBJVohp38yMgO5B+ppaFA2gl2lfZ9lRyzTM=; _yasc=ly1J5plgvtniKU7ua6LxbUfO7Jb9G9wdLEP+7gxsL0XmM2y8JSbHkkjS4s1nkZaZgpPCl2vDlI/WY8lE; bh=EkEiTm90KEE6QnJhbmQiO3Y9Ijk5IiwgIkdvb2dsZSBDaHJvbWUiO3Y9IjEzMyIsICJDaHJvbWl1bSI7dj0iMTMzIhoFIng4NiIiDyIxMzMuMC42OTQzLjUzIioCPzAyAiIiOgkiV2luZG93cyJCCCIxNS4wLjAiSgQiNjQiUlsiTm90KEE6QnJhbmQiO3Y9Ijk5LjAuMC4wIiwgIkdvb2dsZSBDaHJvbWUiO3Y9IjEzMy4wLjY5NDMuNTMiLCAiQ2hyb21pdW0iO3Y9IjEzMy4wLjY5NDMuNTMiWgI/MGDu+Zi9Bmoe3Mrh/wiS2KGxA5/P4eoD+/rw5w3r//32D6K4zocI; yp=2052890594.pcs.0#2047747245.udn.cDpsdW94dWViaW5mZWk=#1753387600.szm.1:1920x1080:1903x911#1744132360.atds.1#1740208523.csc.1"),
        );

        headers
    }

    fn parse_html(html: &str) -> Result<Vec<SearchResult>> {
        // 检查维护信息
        if html.contains("The service is under construction") {
            return Err(Error::Engine("Yandex 图片搜索服务正在维护中".to_string()));
        }

        let document = Html::parse_document(html);
        let mut results = Vec::new();

        // 匹配 JSON 数据
        let data_div_selector = Selector::parse("div.Root[id^='CbirSites_infinite']").unwrap();
        if let Some(data_div) = document.select(&data_div_selector).next() {
            if let Some(data_state) = data_div.value().attr("data-state") {
                if let Ok(json) = serde_json::from_str::<Value>(data_state) {
                    if let Some(sites) = json.get("sites").and_then(|v| v.as_array()) {
                        for (index, site) in sites.iter().enumerate() {
                            // 基本信息
                            let url = site.get("url").and_then(|v| v.as_str()).unwrap_or_default();
                            let title = site
                                .get("title")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();
                            let domain = site
                                .get("domain")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();
                            let content = site
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default();

                            // 缩略图
                            let thumbnail = site
                                .get("thumb")
                                .and_then(|v| v.get("url"))
                                .and_then(|v| v.as_str())
                                .map(|src| {
                                    if src.starts_with("//") {
                                        format!("https:{}", src)
                                    } else {
                                        src.to_string()
                                    }
                                });

                            // 尺寸信息
                            let (size, size_str) = if let Some(img) = site.get("originalImage") {
                                let width = img.get("width").and_then(|v| v.as_u64());
                                let height = img.get("height").and_then(|v| v.as_u64());
                                match (width, height) {
                                    (Some(w), Some(h)) => {
                                        (Some((w as u32, h as u32)), Some(format!("{}x{}", w, h)))
                                    }
                                    _ => (None, None),
                                }
                            } else {
                                (None, None)
                            };

                            if !url.is_empty() {
                                results.push(SearchResult {
                                    title: Some(format!(
                                        "[{}] {} - {}",
                                        domain,
                                        title,
                                        size_str.unwrap_or_default()
                                    )),
                                    url: url.to_string(),
                                    thumbnail,
                                    similarity: None,
                                    source: "Yandex".to_string(),
                                    index: Some(index.to_string()),
                                    additional_info: Some(AdditionalInfo {
                                        author: None,
                                        author_url: None,
                                        source_url: Some(format!("https://{}", domain)),
                                        created_at: None,
                                        tags: Vec::new(),
                                        size,
                                        ext_urls: vec![],
                                    }),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    async fn parse_results(&self, html: &str) -> Result<Vec<SearchResult>> {
        let results = Self::parse_html(html)?;
        Ok(results)
    }

    async fn save_html(html: &str, prefix: &str) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("{}_{}.html", prefix, timestamp);
        fs::write(&filename, html)?;
        Ok(())
    }
}

#[async_trait]
impl ImageSearch for Yandex {
    fn name(&self) -> &'static str {
        "Yandex"
    }

    async fn search_url(
        &self,
        url: &str,
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        // 构建请求头
        let headers = Self::build_headers();

        // 构建请求 URL
        let search_url = format!(
            "{}?rpt=imageview&url={}&cbir_page=sites",
            self.base_url, url
        );

        // 发送请求
        let response = self.network.get_with_headers(&search_url, headers).await?;
        let response_url = response.url().to_string();

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "Yandex returned status code: {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let results = self.parse_results(&html).await?;

        Ok((response_url, results))
    }

    async fn search_bytes(
        &self,
        bytes: &[u8],
        _options: &SearchOptions,
    ) -> Result<(String, Vec<SearchResult>)> {
        // 构建请求头
        let headers = Self::build_headers();

        // 构建 multipart 表单
        let part = multipart::Part::bytes(bytes.to_vec())
            .file_name("image.jpg")
            .mime_str("image/jpeg")?;

        let form = multipart::Form::new().text("prg", "1").part("upfile", part);

        // 发送请求 - 参数放在 URL 中
        let search_url = format!("{}?rpt=imageview&cbir_page=sites", self.base_url);
        let response = self
            .network
            .post_multipart_with_headers(&search_url, form, headers)
            .await?;
        let response_url = response.url().to_string();

        if !response.status().is_success() {
            return Err(Error::Engine(format!(
                "上传失败: 状态码 {}",
                response.status()
            )));
        }

        let html = response.text().await?;
        let results = self.parse_results(&html).await?;

        Ok((response_url, results))
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
