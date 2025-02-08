use crate::error::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::io::Cursor;
use std::path::Path;


pub async fn image_to_base64<P: AsRef<Path>>(path: P) -> Result<String> {
    let img = image::open(path)?;
    // 转换为 RGB 并调整大小到 250x250
    let img = img.into_rgb8();
    let img =
        image::DynamicImage::ImageRgb8(img).resize(250, 250, image::imageops::FilterType::Lanczos3);

    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)?;
    Ok(BASE64.encode(&buf))
}

pub fn bytes_to_base64(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

pub fn base64_to_bytes(base64: &str) -> Result<Vec<u8>> {
    Ok(BASE64.decode(base64)?)
}

pub fn url_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

pub fn html_unescape(s: &str) -> String {
    html_escape::decode_html_entities(s).into_owned()
}

pub fn extract_numbers(s: &str) -> Vec<i64> {
    let re = regex::Regex::new(r"\d+").unwrap();
    re.find_iter(s)
        .filter_map(|m| m.as_str().parse().ok())
        .collect()
}

pub fn normalize_url(url: &str) -> Result<String> {
    if !url.starts_with("http") {
        return Ok(format!("https:{}", url));
    }
    Ok(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_encode() {
        assert_eq!(url_encode("hello world"), "hello%20world");
        assert_eq!(url_encode("你好"), "%E4%BD%A0%E5%A5%BD");
    }

    #[test]
    fn test_html_unescape() {
        assert_eq!(html_unescape("&lt;div&gt;"), "<div>");
        assert_eq!(html_unescape("&quot;hello&quot;"), "\"hello\"");
    }

    #[test]
    fn test_extract_numbers() {
        assert_eq!(extract_numbers("abc123def456"), vec![123, 456]);
        assert_eq!(extract_numbers("no numbers"), Vec::<i64>::new());
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(
            normalize_url("//example.com").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            normalize_url("http://example.com").unwrap(),
            "http://example.com"
        );
    }
}
