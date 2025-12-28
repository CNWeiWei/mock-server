use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
// 确保 Cargo.toml 中有 serde_json
use serde_json::json;
use mock_server::config::{MockRule, MockSource};
use mock_server::loader::MockLoader;
use mock_server::router::MockRouter;
use std::collections::HashMap;

/// 模块一：验证 Config 反序列化逻辑
#[test]
fn test_config_parsing_scenarios() {
    // 场景 A: 验证单接口配置 (增加 body 结构化校验)
    let yaml_single = r#"
        id: "auth_v1"
        request:
          method: "POST"
          path: "/api/v1/login"
          body: { "user": "admin" }
        response: { status: 200, body: "welcome" }
    "#;
    let source_s: MockSource = serde_yaml::from_str(yaml_single).expect("解析单接口失败");
    let rules = source_s.flatten();
    assert_eq!(rules.len(), 1);
    // 验证 body 是否被成功解析为 Value::Object
    assert!(rules[0].request.body.is_some());
    assert_eq!(rules[0].request.body.as_ref().unwrap()["user"], "admin");

    // 场景 B: 验证多接口配置 (原有逻辑不变)
    let yaml_multi = r#"
        - id: "api_1"
          request: { method: "GET", path: "/health" }
          response: { status: 200, body: "ok" }
        - id: "api_2"
          request: { method: "GET", path: "/version" }
          response: { status: 200, body: "1.0" }
    "#;
    let source_m: MockSource = serde_yaml::from_str(yaml_multi).expect("解析多接口失败");
    assert_eq!(source_m.flatten().len(), 2);

    // 场景 C: 验证 Smart Body 的 file:// 协议字符串解析 (原有逻辑不变)
    let yaml_file = r#"
        id: "export_api"
        request: { method: "GET", path: "/download" }
        response: { status: 200, body: "file://./storage/data.zip" }
    "#;
    let source_f: MockSource = serde_yaml::from_str(yaml_file).unwrap();
    let rule = &source_f.flatten()[0];
    assert!(rule.response.body.starts_with("file://"));
}

/// 模块二：验证 Loader 递归扫描与索引构建 (不涉及 Matcher 逻辑，基本保持不变)
#[test]
fn test_loader_recursive_indexing() {
    let temp_root = tempdir().expect("无法创建临时目录");
    let root_path = temp_root.path();

    let auth_path = root_path.join("v1/auth");
    fs::create_dir_all(&auth_path).unwrap();

    let mut f1 = File::create(auth_path.join("login.yaml")).unwrap();
    writeln!(f1, "id: 'l1'\nrequest: {{ method: 'POST', path: '/api/v1/login' }}\nresponse: {{ status: 200, body: 'ok' }}").unwrap();

    let mut f2 = File::create(root_path.join("sys.yaml")).unwrap();
    writeln!(f2, "- id: 's1'\n  request: {{ method: 'GET', path: '/health' }}\n  response: {{ status: 200, body: 'up' }}").unwrap();

    let index = MockLoader::load_all_from_dir(root_path);

    assert!(index.contains_key("api"));
    assert!(index.contains_key("health"));
    let total: usize = index.values().map(|v| v.len()).sum();
    assert_eq!(total, 2);
}

#[test]
fn test_router_matching_logic() {
    let mut index = HashMap::new();

    // 1. 准备带有 Body 的规则
    let rule_auth = serde_yaml::from_str::<MockSource>(
        r#"
        id: "auth_v1"
        request:
          method: "POST"
          path: "/api/v1/login"
          headers: { "Content-Type": "application/json" }
          body: { "code": 123 }
        response: { status: 200, body: "token_123" }
    "#,
    )
        .unwrap()
        .flatten();

    index.insert("api".to_string(), vec![rule_auth[0].clone()]);
    let router = MockRouter::new(index);

    // 2. 测试场景 A：完全匹配 (包括 Body)
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    // 构造请求 Body
    let incoming_body = Some(json!({ "code": 123 }));

    let matched = router.match_rule(
        "POST",
        "/api/v1/login",
        &HashMap::new(),
        &headers,
        &incoming_body // 传入新参数
    );
    assert!(matched.is_some());
    assert_eq!(matched.unwrap().id, "auth_v1");

    // 3. 测试场景 B：Body 不匹配
    let wrong_body = Some(json!({ "code": 456 }));
    let matched_fail = router.match_rule(
        "POST",
        "/api/v1/login",
        &HashMap::new(),
        &headers,
        &wrong_body
    );
    assert!(matched_fail.is_none(), "Body 不一致时不应匹配成功");

    // 4. 测试场景 C：智能字符串转换验证 (YAML 是字符串，请求是对象)
    let rule_str_body = serde_yaml::from_str::<MockSource>(
        r#"
        id: "str_match"
        request:
          method: "POST"
          path: "/api/str"
          body: '{"type": "json_in_string"}' # 这里 YAML 解析为 Value::String
        response: { status: 200, body: "ok" }
    "#).unwrap().flatten();

    let mut index2 = HashMap::new();
    index2.insert("api".to_string(), vec![rule_str_body[0].clone()]);
    let router2 = MockRouter::new(index2);

    let incoming_obj = Some(json!({ "type": "json_in_string" })); // 请求是 JSON 对象
    let matched_str = router2.match_rule(
        "POST",
        "/api/str",
        &HashMap::new(),
        &HashMap::new(),
        &incoming_obj
    );
    assert!(matched_str.is_some(), "应该支持将 YAML 字符串 body 转换为对象进行匹配");

    // 5. 测试场景 D：末尾斜杠兼容性测试
    let matched_slash = router.match_rule(
        "POST",
        "/api/v1/login/",
        &HashMap::new(),
        &headers,
        &incoming_body
    );
    assert!(matched_slash.is_some());
}