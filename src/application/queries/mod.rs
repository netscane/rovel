//! 应用层 - 查询（读操作）
//!
//! CQRS 查询侧：处理所有读操作

mod audio_queries;
mod novel_queries;
mod voice_queries;

pub mod handlers;

pub use audio_queries::*;
pub use novel_queries::*;
pub use voice_queries::*;
