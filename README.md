# ReverseImageSearch-rs

ReverseImageSearch-rs 是一个 Rust 实现的多引擎图片搜索聚合工具。支持通过图片 URL、本地文件在多个搜索引擎中进行以图搜图。

## 功能特点

- 支持多个搜索引擎:
  - [SauceNAO](https://saucenao.com/)
  - [Ascii2d](https://ascii2d.net/)
  - [Google](https://www.google.com/)
  - [Google Lens](https://lens.google.com/)
  - [IQDB](https://iqdb.org/)
  - [Yandex](https://yandex.com/images/)
  - [Soutubot](https://soutubot.moe/)

- 支持多种搜索方式:
  - URL 搜索
  - 本地文件搜索
- 异步实现，性能优异
- 统一的结果格式
- 错误处理完善
- 支持代理配置
- 部分引擎支持展示结果页链接

## 安装

目前项目尚未发布到 crates.io，你可以通过以下方式使用：

1. 通过 Git 依赖安装：

```toml
[dependencies]
reverse-image-search = { git = "https://github.com/luoxuebinfei/ReverseImageSearch-rs" }
```

2. 或者克隆到本地后通过路径依赖：

```toml
[dependencies]
reverse-image-search = { path = "../ReverseImageSearch-rs" }
```

3. 如果你想自己构建：

```bash
# 克隆仓库
git clone https://github.com/luoxuebinfei/ReverseImageSearch-rs
cd ReverseImageSearch-rs

# 构建
cargo build

# 运行测试
cargo test

# 运行示例
cargo run --example basic
```

## 使用示例

```rust
use reverse_image_search::{
    engines::{Ascii2d, Google, GoogleLens, Iqdb, SauceNao, Soutubot, Yandex},
    types::SearchOptions,
    ImageSearch,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化搜索选项
    let options = SearchOptions::default();

    // 创建搜索引擎实例
    let saucenao = SauceNao::new(Some("your_api_key".to_string()));
    let ascii2d = Ascii2d::new();

    // URL 搜索
    let url = "https://example.com/image.jpg";
    let (page_url, results) = ascii2d.search_url(url, &options).await?;

    // 文件搜索
    let (page_url, results) = ascii2d.search_file("path/to/image.jpg", &options).await?;

    Ok(())
}
```

## 搜索引擎说明

### SauceNAO
- 需要 API key
- 适合动漫图片搜索
- 结果准确度高

### Ascii2d
- 无需 API key
- 支持色彩搜索和特征搜索
- 适合动漫图片搜索
- 有CF验证

### Google & Google Lens
- 无需 API key
- 通用图片搜索
- 部分功能不可用
- Google Lens 仅支持URL搜索的结果页链接

### IQDB
- 无需 API key
- 适合动漫图片搜索
- 支持多个数据源

### Yandex
- 无需 API key
- 通用图片搜索
- 结果丰富
- URL和文件搜索都支持结果页链接

### Soutubot
- 无需 API key
- 适合 E-Hentai/ExHentai 搜索
- 可能需要处理 CloudFlare 验证
- 支持结果页链接

## 配置选项

```rust
pub struct SearchOptions {
    pub proxy: Option<String>,         // 代理设置
    pub timeout: Option<u64>,          // 超时设置
    pub min_similarity: Option<f32>,   // 最小相似度
    pub hide_explicit: bool,           // 是否隐藏成人内容
}
```

## 搜索结果格式

```rust
pub struct SearchResult {
    pub title: Option<String>,         // 标题
    pub url: String,                   // URL
    pub thumbnail: Option<String>,     // 缩略图
    pub similarity: Option<f32>,       // 相似度
    pub source: String,                // 来源
    pub index: Option<String>,         // 索引
    pub additional_info: Option<AdditionalInfo>, // 额外信息
}

pub struct AdditionalInfo {
    pub author: Option<String>,        // 作者
    pub author_url: Option<String>,    // 作者链接
    pub source_url: Option<String>,    // 来源链接
    pub created_at: Option<String>,    // 创建时间
    pub tags: Vec<String>,             // 标签
    pub size: Option<(u32, u32)>,      // 图片尺寸
    pub ext_urls: Vec<String>,         // 额外链接
}
```

搜索结果页链接

```rust
let (page_url, results) = google_lens.search_url(url, &options).await?;
```


## 错误处理

所有错误都通过 `Error` 枚举统一处理：

```rust
pub enum Error {
    Request(reqwest::Error),           // HTTP 请求错误
    Url(url::ParseError),             // URL 解析错误
    Io(std::io::Error),               // IO 错误
    Json(serde_json::Error),          // JSON 解析错误
    Image(image::error::ImageError),   // 图片处理错误
    Base64(base64::DecodeError),      // Base64 解码错误
    Engine(String),                    // 搜索引擎错误
    RateLimit,                         // 速率限制
    InvalidResponse(String),           // 无效响应
    UrlEncode(serde_urlencoded::ser::Error), // URL 编码错误
}
```

## 开发说明

1. 克隆仓库
2. 安装依赖: `cargo build`
3. 运行测试: `cargo test`
4. 运行示例: `cargo run --example basic`

## 注意事项

1. 部分搜索引擎可能需要代理
2. 注意遵守各搜索引擎的使用条款
3. SauceNAO 需要 API key，可以从其官网获取
4. 建议设置适当的超时和重试机制
5. 某些引擎可能会遇到 CloudFlare 验证

## 开发工具

- [Cursor IDE](https://www.cursor.com/)

## 鸣谢

- [PicImageSearch](https://github.com/kitUIN/PicImageSearch) - 根据此项目进行二次开发
- [google-lens-python](https://github.com/krishna2206/google-lens-python) - 本项目中 Google Lens 的实现来源

## License

MIT License