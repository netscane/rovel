//! HTTP Error Handling - V2 架构

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// 统一错误响应格式
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub errno: i32,
    pub error: String,
    pub data: Option<()>,
}

impl ErrorResponse {
    pub fn new(errno: i32, error: impl Into<String>) -> Self {
        Self {
            errno,
            error: error.into(),
            data: None,
        }
    }
}

/// 错误码定义
pub mod errno {
    pub const BAD_REQUEST: i32 = 400;
    pub const NOT_FOUND: i32 = 404;
    pub const CONFLICT: i32 = 409;
    pub const INTERNAL_ERROR: i32 = 500;
    pub const SERVICE_UNAVAILABLE: i32 = 503;
}

/// API 错误
#[derive(Debug)]
pub enum ApiError {
    NotFound(String),
    BadRequest(String),
    Internal(String),
    Conflict(String),
    ServiceUnavailable(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, response) = match &self {
            ApiError::NotFound(msg) => {
                tracing::warn!(errno = errno::NOT_FOUND, error = %msg, "Resource not found");
                (
                    StatusCode::OK,
                    ErrorResponse::new(errno::NOT_FOUND, msg.clone()),
                )
            }
            ApiError::BadRequest(msg) => {
                tracing::warn!(errno = errno::BAD_REQUEST, error = %msg, "Bad request");
                (
                    StatusCode::OK,
                    ErrorResponse::new(errno::BAD_REQUEST, msg.clone()),
                )
            }
            ApiError::Internal(msg) => {
                tracing::error!(errno = errno::INTERNAL_ERROR, error = %msg, "Internal server error");
                (
                    StatusCode::OK,
                    ErrorResponse::new(errno::INTERNAL_ERROR, msg.clone()),
                )
            }
            ApiError::Conflict(msg) => {
                tracing::warn!(errno = errno::CONFLICT, error = %msg, "Resource conflict");
                (
                    StatusCode::OK,
                    ErrorResponse::new(errno::CONFLICT, msg.clone()),
                )
            }
            ApiError::ServiceUnavailable(msg) => {
                tracing::error!(errno = errno::SERVICE_UNAVAILABLE, error = %msg, "Service unavailable");
                (
                    StatusCode::OK,
                    ErrorResponse::new(errno::SERVICE_UNAVAILABLE, msg.clone()),
                )
            }
        };

        (status, Json(response)).into_response()
    }
}

impl From<crate::application::RepositoryError> for ApiError {
    fn from(e: crate::application::RepositoryError) -> Self {
        match e {
            crate::application::RepositoryError::NotFound(msg) => ApiError::NotFound(msg),
            crate::application::RepositoryError::Duplicate(msg) => ApiError::Conflict(msg),
            _ => ApiError::Internal(e.to_string()),
        }
    }
}

impl From<crate::application::ApplicationError> for ApiError {
    fn from(e: crate::application::ApplicationError) -> Self {
        match e {
            crate::application::ApplicationError::NotFound { resource_type, id } => {
                ApiError::NotFound(format!("{} not found: {}", resource_type, id))
            }
            crate::application::ApplicationError::ValidationError(msg) => ApiError::BadRequest(msg),
            crate::application::ApplicationError::BusinessRuleViolation(msg) => {
                ApiError::BadRequest(msg)
            }
            crate::application::ApplicationError::InvalidState(msg) => ApiError::BadRequest(msg),
            crate::application::ApplicationError::RepositoryError(msg) => ApiError::Internal(msg),
            crate::application::ApplicationError::ExternalServiceError(msg) => {
                ApiError::ServiceUnavailable(msg)
            }
            crate::application::ApplicationError::StorageError(msg) => ApiError::Internal(msg),
            crate::application::ApplicationError::InternalError(msg) => ApiError::Internal(msg),
        }
    }
}
