use std::collections::HashMap;
use crate::config::MockRule;

pub struct MockRouter {
    // 索引表：Key 是路径首段（如 "api"），Value 是该段下的所有 Mock 规则
    pub index: HashMap<String, Vec<MockRule>>,
}

impl MockRouter {
    pub fn new(index: HashMap<String, Vec<MockRule>>) -> Self {
        Self { index }
    }

    /// 核心匹配函数：根据请求信息寻找匹配的 Mock 规则
    pub fn match_rule(
        &self,
        method: &str,
        path: &str,
        queries: &HashMap<String, String>,
        headers: &HashMap<String, String>,
    ) -> Option<&MockRule> {
        // 1. 提取请求路径的首段作为索引 Key
        let key = self.extract_first_segment(path);
        println!("DEBUG: Request Key: '{}', Available Keys: {:?}", key, self.index.keys());
        // 2. 从 HashMap 中快速定位候选规则列表 (O(k) 复杂度)
        if let Some(rules) = self.index.get(&key) {
            // 3. 在候选集中进行线性深度匹配
            for rule in rules {
                if self.is_match(rule, method, path, queries, headers) {
                    return Some(rule);
                }
            }
        }

        None
    }

    /// 细粒度匹配逻辑
    fn is_match(
        &self,
        rule: &MockRule,
        method: &str,
        path: &str,
        queries: &HashMap<String, String>,
        headers: &HashMap<String, String>,
    ) -> bool {
        // A. 基础校验：Method 和 Path 必须完全一致 (忽略末尾斜杠)
        if rule.request.method.to_uppercase() != method.to_uppercase() {
            println!("DEBUG: [ID:{}] Method Mismatch: YAML={}, Req={}", rule.id, rule.request.method, method);
            return false;
        }
        if rule.request.path.trim_end_matches('/') != path.trim_end_matches('/') {
            println!("DEBUG: [ID:{}] Path Mismatch: YAML='{}', Req='{}'", rule.id, rule.request.path, path);
            return false;
        }
        println!("DEBUG: [ID:{}] Method and Path matched! Checking headers...", rule.id);
        // B. Query 参数校验 (子集匹配原则)
        if let Some(ref required_queries) = rule.request.query_params {
            for (key, val) in required_queries {
                if queries.get(key) != Some(val) {
                    return false;
                }
            }
        }
        
        // C. Header 校验 (优化版)
        if let Some(ref required_headers) = rule.request.headers {
            for (key, val) in required_headers {
                println!("{}:{}",key.clone(), val.clone());
                // 方案：将 Key 统一转为小写比较，并检查请求头是否“包含”期望的值
                let matched = headers.iter().any(|(k, v)| {
                    // k.to_lowercase() == key.to_lowercase() && v.contains(val)
                    k.to_lowercase() == key.to_lowercase() && v==val
                });

                if !matched {
                    return false;
                }
            }
        }

        true
    }

    /// 与 Loader 保持一致的 Key 提取算法
    fn extract_first_segment(&self, path: &str) -> String {
        path.trim_start_matches('/')
            .split('/')
            .next()
            .unwrap_or("root")
            .to_string()
    }
}