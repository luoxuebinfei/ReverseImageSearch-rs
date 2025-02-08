use anyhow::Result;
use reverse_image_search::{
    engines::{Ascii2d, Google, GoogleLens, Iqdb, SauceNao, Soutubot, Yandex},
    types::{SearchOptions, SearchResult},
    ImageSearch,
};
use std::env;
use std::path::Path;

async fn test_engine<T: ImageSearch>(
    engine: &T,
    url: &str,
    file_path: &str,
    options: &SearchOptions,
) -> Result<()> {
    println!("\n测试 {} 引擎:", engine.name());

    // URL 搜索
    println!("\nURL 搜索结果:");
    match engine.search_url(url, options).await {
        Ok((page_url, results)) => {
            if !page_url.is_empty() {
                println!("结果页面: {}", page_url);
            }
            print_results(&results);
        }
        Err(e) => println!("URL 搜索错误: {}", e),
    }

    // 文件搜索
    println!("\n文件搜索结果:");
    match engine.search_file(file_path, options).await {
        Ok((page_url, results)) => {
            if !page_url.is_empty() {
                println!("结果页面: {}", page_url);
            }
            print_results(&results);
        }
        Err(e) => println!("文件搜索错误: {}", e),
    }

    Ok(())
}

fn print_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("未找到结果");
        return;
    }

    for (i, result) in results.iter().enumerate() {
        println!("\n结果 #{}:", i + 1);
        println!("标题: {}", result.title.as_deref().unwrap_or("未知"));
        println!("URL: {}", result.url);
        println!("缩略图: {}", result.thumbnail.as_deref().unwrap_or("无"));
        if let Some(similarity) = result.similarity {
            println!("相似度: {}%", similarity);
        }
        println!("来源: {}", result.source);
        if let Some(index) = &result.index {
            println!("索引: {}", index);
        }
        if let Some(info) = &result.additional_info {
            if let Some(author) = &info.author {
                println!("作者: {}", author);
            }
            if let Some(author_url) = &info.author_url {
                println!("作者链接: {}", author_url);
            }
            if !info.tags.is_empty() {
                println!("标签: {}", info.tags.join(", "));
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // 加载环境变量
    dotenv::dotenv().ok();

    // 初始化日志
    env_logger::init();

    // 获取 API key
    let api_key = env::var("SAUCENAO_API_KEY").ok();

    // 创建搜索实例
    let saucenao = SauceNao::new(api_key);
    let ascii2d = Ascii2d::new();
    let google = Google::new();
    let google_lens = GoogleLens::new();
    let iqdb = Iqdb::new();
    let yandex = Yandex::default();
    let options = SearchOptions::default();

    let client = reqwest::Client::new();

    // 初始化搜索引擎
    let soutubot = Soutubot::new(client);

    // 图片路径
    let img_path = Path::new("test_img/2.png");
    // 图片链接搜索
    let url =
        "https://telegraph-image-92x.pages.dev/file/bc7e6e4cc62f37b159357-6ad3189e5bc5d25dee.png";

    // 测试每个引擎
    test_engine(&saucenao, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&ascii2d, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&google, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&google_lens, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&iqdb, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&yandex, url, img_path.to_str().unwrap(), &options).await?;
    test_engine(&soutubot, url, img_path.to_str().unwrap(), &options).await?;

    Ok(())
}