use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock}; // 必须引入 RwLock
use tokio_util::io::ReaderStream;

use crate::router::MockRouter;

/// 共享的应用状态，router 现在由 RwLock 保护以支持热重载
pub struct AppState {
    pub router: RwLock<MockRouter>,
}

/// 全局统一请求处理函数
pub async fn mock_handler(
    State(state): State<Arc<AppState>>, // State 必须是第一个或靠前的参数
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    req: Request<Body>, // Request<Body> 必须是最后一个参数
) -> impl IntoResponse {
    let path = req.uri().path().to_string(); // 先提取 path

    // 1. 将 Axum HeaderMap 转换为简单的 HashMap
    let mut req_headers = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            req_headers.insert(name.as_str().to_string(), v.to_string());
        }
    }

    // 2. 执行匹配逻辑：先获取读锁 (Read Lock)
    let maybe_rule = {
        let router = state.router.read().expect("Failed to acquire read lock");
        router.match_rule(method.as_str(), &path, &params, &req_headers).cloned()
        // 此处使用 .cloned() 以便尽早释放读锁，避免阻塞热重载写锁
    };

    if let Some(rule) = maybe_rule {
        // 3. 处理模拟延迟
        if let Some(ref settings) = rule.settings {
            if let Some(delay) = settings.delay_ms {
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }

        // 4. 构建响应
        let status = StatusCode::from_u16(rule.response.status).unwrap_or(StatusCode::OK);
        let mut response_builder = Response::builder().status(status);

        if let Some(ref h) = rule.response.headers {
            for (k, v) in h {
                response_builder = response_builder.header(k, v);
            }
        }

        // 5. Smart Body 逻辑
        if let Some(file_path) = rule.response.get_file_path() {
            match tokio::fs::File::open(file_path).await {
                Ok(file) => {
                    let stream = ReaderStream::new(file);
                    response_builder.body(Body::from_stream(stream)).unwrap()
                }
                Err(_) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!("File not found: {}", file_path)))
                    .unwrap(),
            }
        } else {
            response_builder.body(Body::from(rule.response.body.clone())).unwrap()
        }
    } else {
        // 匹配失败
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("No mock rule matched this request"))
            .unwrap()
    }
}