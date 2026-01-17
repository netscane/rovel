//! Novel Context - Errors

use thiserror::Error;

use super::NovelId;

#[derive(Debug, Error)]
pub enum NovelError {
    #[error("小说不存在: {0}")]
    NotFound(NovelId),

    #[error("小说已存在: {0}")]
    AlreadyExists(NovelId),

    #[error("无效的标题: {0}")]
    InvalidTitle(String),

    #[error("无效的文本内容: {0}")]
    InvalidContent(String),

    #[error("文件读取错误: {0}")]
    FileReadError(String),

    #[error("存储错误: {0}")]
    StorageError(String),

    #[error("分段错误: {0}")]
    SegmentationError(String),
}
