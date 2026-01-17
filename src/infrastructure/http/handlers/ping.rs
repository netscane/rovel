//! Ping Handler
//!
//! Health check endpoint similar to OpenSubsonic ping

use axum::Json;
use serde::Serialize;

/// Ping 响应
#[derive(Serialize)]
pub struct PingResponse {
    pub status: &'static str,
    pub version: &'static str,
}

/// Ping endpoint - 健康检查
pub async fn ping() -> Json<PingResponse> {
    Json(PingResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
