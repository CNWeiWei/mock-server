// 使用项目名（下划线形式）引用 lib 中的内容
use mock_server::loader::MockLoader;

#[tokio::main]
async fn main() {
    // 你的启动逻辑...
    let index = MockLoader::load_all_from_dir(std::path::Path::new("./mocks"));
    println!("服务启动中...");
}