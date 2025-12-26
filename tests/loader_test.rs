use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;
// 假设你的项目名在 Cargo.toml 中叫 mock_server
use mock_server::config::MockSource;
use mock_server::loader::MockLoader;

#[test]
fn test_config_deserialization() {
    // 测试 1：验证单接口 YAML 解析
    let single_yaml = r#"
        id: "auth_v1"
        request:
          method: "POST"
          path: "/api/v1/login"
        response:
          status: 200
          body: "inline_content"
    "#;
    let res: MockSource = serde_yaml::from_str(single_yaml).expect("应该成功解析单接口");
    assert_eq!(res.flatten().len(), 1);
    // assert_eq!(res.flatten()[0].id, "auth_v1");

    // 测试 2：验证多接口 YAML 数组解析
    let multi_yaml = r#"
        - id: "api_1"
          request: { method: "GET", path: "/1" }
          response: { status: 200, body: "b1" }
        - id: "api_2"
          request: { method: "GET", path: "/2" }
          response: { status: 200, body: "b2" }
    "#;
    let res_multi: MockSource = serde_yaml::from_str(multi_yaml).expect("应该成功解析接口数组");
    assert_eq!(res_multi.flatten().len(), 2);
}

#[test]
fn test_recursive_loading_logic() {
    // 创建临时 Mock 目录结构
    let root_dir = tempdir().expect("创建临时目录失败");
    let root_path = root_dir.path();

    // 构造物理层级：mocks/v1/user/
    let user_dir = root_path.join("v1/user");
    fs::create_dir_all(&user_dir).unwrap();

    // 在深层目录创建单接口文件
    let mut file1 = File::create(user_dir.join("get_profile.yaml")).unwrap();
    writeln!(
        file1,
        r#"
        id: "user_profile"
        request:
          method: "GET"
          path: "/api/v1/user/profile"
        response:
          status: 200
          body: "profile_data"
    "#
    )
    .unwrap();

    // 在根目录创建多接口文件
    let mut file2 = File::create(root_path.join("system.yaml")).unwrap();
    writeln!(
        file2,
        r#"
        - id: "sys_health"
          request: {{ method: "GET", path: "/health" }}
          response: {{ status: 200, body: "ok" }}
        - id: "sys_version"
          request: {{ method: "GET", path: "/version" }}
          response: {{ status: 200, body: "1.0.0" }}
    "#
    )
    .unwrap();
//     writeln!(
//         file2,
//         r#"
// - id: "sys_health"
//   request:
//     method: "GET"
//     path: "/health"
//   response:
//     status: 200
//     body: "ok"
// - id: "sys_version"
//   request:
//     method: "GET"
//     path: "/version"
//   response:
//     status: 200
//     body: "1.0.0"
// "#
//     )
//     .unwrap();

    // 执行加载
    let index = MockLoader::load_all_from_dir(root_path);

    // 断言结果：
    // 1. 检查 Key 是否根据路径首段正确提取（/api/v1/... -> api, /health -> health）
    assert!(index.contains_key("api"), "索引应包含 'api' 键");
    assert!(index.contains_key("health"), "索引应包含 'health' 键");
    assert!(index.contains_key("version"), "索引应包含 'version' 键");

    // 2. 检查规则总数
    let total_rules: usize = index.values().map(|v| v.len()).sum();
    assert_eq!(total_rules, 3, "总规则数应为 3");

    // 3. 检查深层文件是否被正确读取
    let api_rules = &index["api"];
    assert_eq!(api_rules[0].id, "user_profile");
}
