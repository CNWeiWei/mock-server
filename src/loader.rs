use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir; // 需在 Cargo.toml 添加 walkdir 依赖

use crate::config::{MockRule, MockSource}; // 假设 config.rs 中定义了这两个类型

pub struct MockLoader;

impl MockLoader {
    /// 递归扫描指定目录并构建索引表
    pub fn load_all_from_dir(dir: &Path) -> HashMap<String, Vec<MockRule>> {
        let mut index: HashMap<String, Vec<MockRule>> = HashMap::new();

        // 1. 使用 walkdir 递归遍历目录，不限层级
        for entry in WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "yaml" || ext == "yml"))
        {
            if let Some(rules) = Self::parse_yaml_file(entry.path()) {
                for rule in rules {
                    // 2. 提取路径首段作为索引 Key
                    let key = Self::extract_first_segment(&rule.request.path);

                    // 3. 将规则插入到对应的索引桶中
                    index.entry(key).or_insert_with(Vec::new).push(rule);
                }
            }
        }

        println!("Successfully loaded {} segments from {:?}", index.len(), dir);
        index
    }

    /// 解析单个 YAML 文件，支持单接口和多接口模式
    fn parse_yaml_file(path: &Path) -> Option<Vec<MockRule>> {
        let content = fs::read_to_string(path).ok()?;

        // 利用 serde_yaml 的反序列化能力处理 MockSource 枚举
        match serde_yaml::from_str::<MockSource>(&content) {
            Ok(source) => Some(source.flatten()), // 统一打平为 Vec<MockRule>
            Err(e) => {
                eprintln!("Failed to parse YAML at {:?}: {}", path, e);
                None
            }
        }
    }

    /// 提取路径的第一级作为 Key（例如 "/api/v1/login" -> "api"）
    fn extract_first_segment(path: &str) -> String {
        path.trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("root") // 如果是根路径 "/"，则归类到 "root"
            .to_string()
    }
}