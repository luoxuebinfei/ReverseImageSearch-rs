use anyhow::Result;
use pic_image_search::{
    engines::{Google, Iqdb, Yandex},
    types::{SearchOptions, SearchResult},
    Ascii2d, GoogleLens, ImageSearch, SauceNao,
};
use std::env;
use std::path::Path;


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

    // 图片路径
    let img_path = Path::new("test_img/1.png");
    // 图片链接搜索
    let url =
        "https://telegraph-image-92x.pages.dev/file/b2d14d15c859f1c8187b6-332c352a0967488a88.png";

    // 测试iqdb的文件搜索
    let result = iqdb.search_file(img_path.to_str().unwrap(), &options).await?;
    println!("搜索结果页面: {}", result.0);
    for (i, result) in result.1.iter().enumerate() {
        println!("\n结果 #{}:", i + 1);
        println!("标题: {}", result.title.as_deref().unwrap_or("未知"));
        println!("URL: {}", result.url);
    }
    Ok(())
}