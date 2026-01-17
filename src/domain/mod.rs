//! Domain Layer - 领域层
//!
//! 包含两个限界上下文:
//! - Novel Context: 小说管理
//! - Voice Context: 音色管理

pub mod novel;
pub mod voice;

// 共享的文本分割器
mod text_segmenter;

pub use text_segmenter::{segment_text, SegmentConfig};
