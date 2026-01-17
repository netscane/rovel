//! HTTP Handlers
//!
//! V2 架构 - 基于 ARCHITECTURE.md 设计

mod audio;
mod infer;
mod novel;
mod ping;
mod session;
mod voice;
mod websocket;

pub use audio::*;
pub use infer::*;
pub use novel::*;
pub use ping::*;
pub use session::*;
pub use voice::*;
pub use websocket::*;
