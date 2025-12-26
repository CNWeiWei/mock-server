use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
// 替换为你的项目实际名称
use mock_server::config::{MockRule, MockSource};
use mock_server::loader::MockLoader;
use mock_server::router::MockRouter;
use std::collections::HashMap;

/// 模块一：验证 Config 反序列化逻辑
#[test]
fn test_config_parsing_scenarios() {
    // 场景 A: 验证单接口配置 (Inline 模式)
    let yaml_single = r#"
        id: "auth_v1"
        request: { method: "POST", path: "/api/v1/login" }
        response: { status: 200, body: "welcome" }
    "#;
    let source_s: MockSource = serde_yaml::from_str(yaml_single).expect("解析单接口失败");
    let rules = source_s.flatten();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].request.path, "/api/v1/login");

    // 场景 B: 验证多接口配置 (Collection 模式)
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

    // 场景 C: 验证 Smart Body 的 file:// 协议字符串解析
    let yaml_file = r#"
        id: "export_api"
        request: { method: "GET", path: "/download" }
        response: { status: 200, body: "file://./storage/data.zip" }
    "#;
    let source_f: MockSource = serde_yaml::from_str(yaml_file).unwrap();
    let rule = &source_f.flatten()[0];
    assert!(rule.response.body.starts_with("file://"));
}

/// 模块二：验证 Loader 递归扫描与索引构建
#[test]
fn test_loader_recursive_indexing() {
    let temp_root = tempdir().expect("无法创建临时目录");
    let root_path = temp_root.path();

    // 创建多级目录结构
    let auth_path = root_path.join("v1/auth");
    fs::create_dir_all(&auth_path).unwrap();

    // 1. 在深层目录写入一个单接口文件
    let mut f1 = File::create(auth_path.join("login.yaml")).unwrap();
    writeln!(f1, "id: 'l1'\nrequest: {{ method: 'POST', path: '/api/v1/login' }}\nresponse: {{ status: 200, body: 'ok' }}").unwrap();

    // 2. 在根目录写入一个多接口文件
    let mut f2 = File::create(root_path.join("sys.yaml")).unwrap();
    writeln!(f2, "- id: 's1'\n  request: {{ method: 'GET', path: '/health' }}\n  response: {{ status: 200, body: 'up' }}").unwrap();

    // 执行加载
    let index = MockLoader::load_all_from_dir(root_path);

    // 验证逻辑：
    // 即使物理路径很深，索引 Key 必须是逻辑路径的首段
    assert!(
        index.contains_key("api"),
        "必须通过 /api/v1/login 提取出 'api' 键"
    );
    assert!(
        index.contains_key("health"),
        "必须通过 /health 提取出 'health' 键"
    );

    // 验证扁平化后的总数
    let total: usize = index.values().map(|v| v.len()).sum();
    assert_eq!(total, 2);
}

#[test]
fn test_router_matching_logic() {
    // 1. 准备模拟数据（模拟 Loader 的输出）
    let mut index = HashMap::new();

    let rule_auth = serde_yaml::from_str::<MockSource>(
        r#"
        id: "auth_v1"
        request:
          method: "POST"
          path: "/api/v1/login"
          headers: { "Content-Type": "application/json" }
        response: { status: 200, body: "token_123" }
    "#,
    )
    .unwrap()
    .flatten();

    let rule_user = serde_yaml::from_str::<MockSource>(
        r#"
        id: "get_user"
        request:
          method: "GET"
          path: "/api/user/info"
          query_params: { "id": "100" }
        response: { status: 200, body: "user_info" }
    "#,
    )
    .unwrap()
    .flatten();

    // 构建 HashMap 索引（模仿 Loader 的行为）
    index.insert(
        "api".to_string(),
        vec![rule_auth[0].clone(), rule_user[0].clone()],
    );

    let router = MockRouter::new(index);

    // 2. 测试场景 A：完全匹配
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());

    let matched = router.match_rule("POST", "/api/v1/login", &HashMap::new(), &headers);
    assert!(matched.is_some());
    assert_eq!(matched.unwrap().id, "auth_v1");

    // 3. 测试场景 B：路径末尾斜杠归一化 (Trailing Slash)
    let matched_slash = router.match_rule("POST", "/api/v1/login/", &HashMap::new(), &headers);
    assert!(matched_slash.is_some(), "应该忽略路径末尾的斜杠");

    // 4. 测试场景 C：Query 参数子集匹配
    let mut queries = HashMap::new();
    queries.insert("id".to_string(), "100".to_string());
    queries.insert("extra".to_string(), "unused".to_string()); // 额外的参数不应影响匹配

    let matched_query = router.match_rule("GET", "/api/user/info", &queries, &HashMap::new());
    assert!(matched_query.is_some());
    assert_eq!(matched_query.unwrap().id, "get_user");

    // 5. 测试场景 D：匹配失败（Method 错误）
    let fail_method = router.match_rule("GET", "/api/v1/login", &HashMap::new(), &headers);
    assert!(fail_method.is_none());
}
