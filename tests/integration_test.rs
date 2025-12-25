use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
// 替换为你的项目实际名称
use mock_server::config::{MockSource, MockRule};
use mock_server::loader::MockLoader;

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
    assert!(index.contains_key("api"), "必须通过 /api/v1/login 提取出 'api' 键");
    assert!(index.contains_key("health"), "必须通过 /health 提取出 'health' 键");

    // 验证扁平化后的总数
    let total: usize = index.values().map(|v| v.len()).sum();
    assert_eq!(total, 2);
}