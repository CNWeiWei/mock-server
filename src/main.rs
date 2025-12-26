use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use axum::{routing::any, Router};
use mock_server::loader::MockLoader;
use mock_server::router::MockRouter;
use mock_server::handler::{mock_handler, AppState};

#[tokio::main]
async fn main() {
    // 1. 初始化日志（建议添加，方便观察加载情况）
    // 需要在 Cargo.toml 添加 tracing-subscriber = "0.3"
    tracing_subscriber::fmt::init();

    // 2. 递归加载所有的 YAML 配置文件
    let mocks_dir = Path::new("./mocks");
    if !mocks_dir.exists() {
        println!("Warning: 'mocks/' directory not found. Creating it...");
        std::fs::create_dir_all(mocks_dir).unwrap();
    }

    println!("Scanning mocks directory...");
    let index = MockLoader::load_all_from_dir(mocks_dir);

    // 3. 构建路由引擎并包装为共享状态
    let router_engine = MockRouter::new(index);
    let shared_state = Arc::new(AppState {
        router: router_engine,
    });

    // 4. 配置 Axum 路由
    // 使用 any(mock_handler) 意味着它会接管所有 HTTP 方法和所有路径的请求
    let app = Router::new()
        .fallback(any(mock_handler))
        .with_state(shared_state);

    // 5. 启动服务
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Rust Mock Server is running on http://{}", addr);
    println!("Ready to handle requests based on your YAML definitions.");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}