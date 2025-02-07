use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: Option<String>,
    pub url: String,
    pub thumbnail: Option<String>,
    pub similarity: Option<f32>,
    pub source: String,
    pub index: Option<String>,
    pub additional_info: Option<AdditionalInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdditionalInfo {
    pub author: Option<String>,
    pub author_url: Option<String>,
    pub source_url: Option<String>,
    pub created_at: Option<String>,
    pub tags: Vec<String>,
    pub size: Option<(u32, u32)>,
    pub ext_urls: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub enum SearchEngine {
    Ascii2d,
    Baidu,
    Bing,
    EHentai,
    Google,
    Iqdb,
    SauceNao,
    Tineye,
    TraceMoe,
    Yandex,
}

impl fmt::Display for SearchEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchEngine::Ascii2d => write!(f, "Ascii2d"),
            SearchEngine::Baidu => write!(f, "Baidu"),
            SearchEngine::Bing => write!(f, "Bing"),
            SearchEngine::EHentai => write!(f, "E-Hentai"),
            SearchEngine::Google => write!(f, "Google"),
            SearchEngine::Iqdb => write!(f, "IQDB"),
            SearchEngine::SauceNao => write!(f, "SauceNAO"),
            SearchEngine::Tineye => write!(f, "TinEye"),
            SearchEngine::TraceMoe => write!(f, "TraceMoe"),
            SearchEngine::Yandex => write!(f, "Yandex"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub proxy: Option<String>,
    pub timeout: Option<u64>,
    pub min_similarity: Option<f32>,
    pub hide_explicit: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            proxy: None,
            timeout: None,
            min_similarity: Some(50.0),
            hide_explicit: false,
        }
    }
}
