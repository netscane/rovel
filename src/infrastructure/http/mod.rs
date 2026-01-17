//! HTTP Layer - RESTful API + WebSocket
//!
//! V2 架构 - 基于 ARCHITECTURE.md 设计

pub mod dto;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod routes;
pub mod server;
pub mod state;

pub use error::ApiError;
pub use routes::create_routes;
pub use server::{HttpServer, ServerConfig};
pub use state::AppState;
