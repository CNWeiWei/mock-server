use std::net::SocketAddr;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use axum::{routing::any, Router};
use notify_debouncer_mini::{new_debouncer, notify::*};

use mock_server::loader::MockLoader;
use mock_server::router::MockRouter;
use mock_server::handler::{mock_handler, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let mocks_dir = Path::new("./mocks");
    if !mocks_dir.exists() {
        std::fs::create_dir_all(mocks_dir).unwrap();
    }

    // 1. 初始加载
    println!("Scanning mocks directory...");
    let index = MockLoader::load_all_from_dir(mocks_dir);
    let shared_state = Arc::new(AppState {
        router: RwLock::new(MockRouter::new(index)),
    });

    // 2. 设置热加载监听器
    let state_for_watcher = shared_state.clone();
    let watch_path = mocks_dir.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();

    // 200ms 防抖，防止编辑器保存文件时产生多次干扰
    let mut debouncer = new_debouncer(Duration::from_millis(200), tx).unwrap();
    debouncer.watcher().watch(&watch_path, RecursiveMode::Recursive).unwrap();

    // 启动异步任务监听文件变动
    tokio::spawn(async move {
        while let Ok(res) = rx.recv() {
            match res {
                Ok(_) => {
                    println!("🔄 Detecting changes in mocks/, reloading...");
                    let new_index = MockLoader::load_all_from_dir(&watch_path);
                    // 获取写锁 (Write Lock) 更新索引
                    let mut writer = state_for_watcher.router.write().expect("Failed to acquire write lock");
                    *writer = MockRouter::new(new_index);
                    println!("✅ Mocks reloaded successfully.");
                }
                Err(e) => eprintln!("Watcher error: {:?}", e),
            }
        }
    });

    // 3. 配置 Axum 路由
    let app = Router::new()
        .fallback(any(mock_handler))
        .with_state(shared_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 Server running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}