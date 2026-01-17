//! Novel Context - Value Objects

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// 小说唯一标识
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NovelId(Uuid);

impl NovelId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for NovelId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NovelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 小说标题
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Title(String);

impl Title {
    pub fn new(title: impl Into<String>) -> Result<Self, &'static str> {
        let title = title.into();
        if title.is_empty() {
            return Err("标题不能为空");
        }
        if title.len() > 200 {
            return Err("标题长度不能超过200字符");
        }
        Ok(Self(title))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Title {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 原始文本路径
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTextPath(PathBuf);

impl RawTextPath {
    pub fn new(path: PathBuf) -> Self {
        Self(path)
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.0
    }
}

impl From<PathBuf> for RawTextPath {
    fn from(path: PathBuf) -> Self {
        Self(path)
    }
}

impl From<&str> for RawTextPath {
    fn from(path: &str) -> Self {
        Self(PathBuf::from(path))
    }
}
