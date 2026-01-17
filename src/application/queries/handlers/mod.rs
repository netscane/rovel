//! Query Handlers 实现
//!
//! 所有 QueryHandler 的具体实现

mod audio_handlers;
mod novel_handlers;
mod voice_handlers;

pub use audio_handlers::*;
pub use novel_handlers::*;
pub use voice_handlers::*;
