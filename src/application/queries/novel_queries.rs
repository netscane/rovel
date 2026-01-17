//! Novel Queries - V2 架构

use uuid::Uuid;

/// 获取小说详情查询
#[derive(Debug, Clone)]
pub struct GetNovel {
    pub novel_id: Uuid,
}

/// 列出所有小说查询
#[derive(Debug, Clone)]
pub struct ListNovels;

/// 获取小说片段查询
#[derive(Debug, Clone)]
pub struct GetNovelSegments {
    pub novel_id: Uuid,
    pub start_index: Option<usize>,
    pub limit: Option<usize>,
}
