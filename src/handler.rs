use axum::{
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, Method, Request, StatusCode},
    response::{IntoResponse, Response},
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio_util::io::ReaderStream; // 需在 Cargo.toml 确认有 tokio-util

use crate::router::MockRouter;

/// 共享的应用状态
pub struct AppState {
    pub router: MockRouter,
}

/// 全局统一请求处理函数
pub async fn mock_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    headers: HeaderMap,
    Query(params): Query<HashMap<String, String>>,
    req: Request<Body>,
) -> impl IntoResponse {
    // let path = req.uri().path();
    // 1. 【关键】将需要的数据克隆出来，断开与 req 的借用关系
    let path = req.uri().path().to_string();
    let method_str = method.as_str().to_string();
    // 1. 将 Axum HeaderMap 转换为简单的 HashMap 供 Router 使用

    // 2. 现在可以安全地消耗 req 了，因为上面没有指向 req 内部的引用了
    let body_bytes = match axum::body::to_bytes(req.into_body(), 10 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Read body error"))
                .unwrap();
        }
    };

    let incoming_json: Option<serde_json::Value> = serde_json::from_slice(&body_bytes).ok();

    let mut req_headers = HashMap::new();
    for (name, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            req_headers.insert(name.as_str().to_string(), v.to_string());
        }
    }
    // 2. 执行匹配逻辑
    if let Some(rule) =
        state
            .router
            .match_rule(&method_str, &path, &params, &req_headers, &incoming_json)
    {
        // 3. 处理模拟延迟
        if let Some(ref settings) = rule.settings {
            if let Some(delay) = settings.delay_ms {
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }

        // 4. 构建响应基础信息
        let status = StatusCode::from_u16(rule.response.status).unwrap_or(StatusCode::OK);
        let mut response_builder = Response::builder().status(status);

        // 注入 YAML 定义的 Header
        if let Some(ref h) = rule.response.headers {
            for (k, v) in h {
                // println!("{}:{}",k.clone(), v.clone());
                response_builder = response_builder.header(k, v);
            }
        }

        // 5. 执行 Smart Body 协议逻辑
        if let Some(file_path) = rule.response.get_file_path() {
            // A. 文件模式：异步打开文件并转换为流，实现低内存占用
            match tokio::fs::File::open(file_path).await {
                Ok(file) => {
                    let stream = ReaderStream::new(file);
                    let body = Body::from_stream(stream);
                    response_builder.body(body).unwrap()
                }
                Err(_) => Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(format!(
                        "Mock Error: File not found at {}",
                        file_path
                    )))
                    .unwrap(),
            }
        } else {
            // B. 内联模式：直接返回字符串内容
            response_builder
                .body(Body::from(rule.response.body.clone()))
                .unwrap()
        }
    } else {
        println!("请求头{:?}", req_headers.clone());
        // 匹配失败返回 404
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("No mock rule matched this request"))
            .unwrap()
    }
}
