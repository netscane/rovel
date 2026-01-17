//! Novel Commands - V2 架构

use std::path::PathBuf;
use uuid::Uuid;

/// 创建小说命令
#[derive(Debug, Clone)]
pub struct CreateNovel {
    pub title: String,
    pub text_path: PathBuf,
}

/// 从文本创建小说命令（第一步：创建 processing 状态记录）
#[derive(Debug, Clone)]
pub struct CreateNovelFromText {
    pub title: String,
    pub text: String,
}

/// 处理小说分段命令（第二步：异步分段处理）
#[derive(Debug, Clone)]
pub struct ProcessNovelSegments {
    pub novel_id: Uuid,
    pub text: String,
}

/// 删除小说命令
#[derive(Debug, Clone)]
pub struct DeleteNovel {
    pub novel_id: Uuid,
}
