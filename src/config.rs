use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 顶层包装：支持单对象或数组，自动打平
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MockSource {
    /// 对应“一接口一文件”模式
    Single(MockRule),
    /// 对应“一文件多接口”模式
    Multiple(Vec<MockRule>),
}

impl MockSource {
    /// 将不同的解析模式统一转化为列表，供 Loader 构建索引
    pub fn flatten(self) -> Vec<MockRule> {
        match self {
            Self::Single(rule) => vec![rule],
            Self::Multiple(rules) => rules,
        }
    }
}

/// 核心 Mock 规则定义
#[derive(Debug, Deserialize,Serialize, Clone,PartialEq)]
pub struct MockRule {
    pub id: String,
    pub request: RequestMatcher,
    pub response: MockResponse,
    pub settings: Option<MockSettings>,
}

/// 请求匹配条件
#[derive(Debug, Deserialize,Serialize, Clone,PartialEq)]
pub struct RequestMatcher {
    pub method: String,
    pub path: String,
    /// 选填：只有请求包含这些参数时才匹配
    pub query_params: Option<HashMap<String, String>>,
    /// 选填：只有请求包含这些 Header 时才匹配
    pub headers: Option<HashMap<String, String>>,
}

/// 响应内容定义
#[derive(Debug, Deserialize,Serialize, Clone,PartialEq)]
pub struct MockResponse {
    pub status: u16,
    pub headers: Option<HashMap<String, String>>,
    /// 统一字段：支持 Inline 文本或 file:// 前缀的路径
    pub body: String,
}

/// 模拟器行为设置
#[derive(Debug, Deserialize,Serialize, Clone,PartialEq)]
pub struct MockSettings {
    /// 模拟网络延迟（毫秒）
    pub delay_ms: Option<u64>,
}

impl MockResponse {
    /// 辅助方法：判断是否为文件协议
    pub fn is_file_protocol(&self) -> bool {
        self.body.starts_with("file://")
    }

    /// 获取去掉协议前缀后的纯物理路径
    pub fn get_file_path(&self) -> Option<&str> {
        if self.is_file_protocol() {
            Some(&self.body[7..]) // 截取 "file://" 之后的内容
        } else {
            None
        }
    }
}
