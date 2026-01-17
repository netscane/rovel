//! Novel Context - 小说限界上下文
//!
//! 职责:
//! - 小说聚合管理
//! - 文本片段实体
//! - 章节和段落管理

mod aggregate;
mod entities;
mod errors;
mod value_objects;

pub use aggregate::Novel;
pub use entities::{Chapter, TextSegment};
pub use errors::NovelError;
pub use value_objects::{NovelId, RawTextPath, Title};
