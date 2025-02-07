use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Failed to parse URL: {0}")]
    Url(#[from] url::ParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Image error: {0}")]
    Image(#[from] image::error::ImageError),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Search engine error: {0}")]
    Engine(String),

    #[error("Rate limit exceeded")]
    RateLimit,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("URL encoding error: {0}")]
    UrlEncode(#[from] serde_urlencoded::ser::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
