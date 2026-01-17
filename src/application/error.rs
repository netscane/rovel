//! 应用层错误定义
//!
//! 统一的命令/查询错误类型

use thiserror::Error;
use uuid::Uuid;

/// 应用层错误
#[derive(Debug, Error)]
pub enum ApplicationError {
    /// 资源未找到
    #[error("{resource_type} not found: {id}")]
    NotFound {
        resource_type: &'static str,
        id: Uuid,
    },

    /// 验证错误
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// 业务规则违反
    #[error("Business rule violation: {0}")]
    BusinessRuleViolation(String),

    /// 状态无效
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// 仓储错误
    #[error("Repository error: {0}")]
    RepositoryError(String),

    /// 外部服务错误
    #[error("External service error: {0}")]
    ExternalServiceError(String),

    /// 存储错误
    #[error("Storage error: {0}")]
    StorageError(String),

    /// 内部错误
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl ApplicationError {
    /// 创建 NotFound 错误
    pub fn not_found(resource_type: &'static str, id: Uuid) -> Self {
        Self::NotFound { resource_type, id }
    }

    /// 创建 NotFound 错误（使用字符串 ID）
    pub fn not_found_str(resource_type: &'static str, id: &str) -> Self {
        Self::ValidationError(format!("{} not found: {}", resource_type, id))
    }

    /// 创建验证错误
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ValidationError(message.into())
    }

    /// 创建业务规则违反错误
    pub fn business_rule(message: impl Into<String>) -> Self {
        Self::BusinessRuleViolation(message.into())
    }

    /// 创建状态无效错误
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::InvalidState(message.into())
    }

    /// 创建内部错误
    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalError(message.into())
    }
}

impl From<crate::application::ports::RepositoryError> for ApplicationError {
    fn from(err: crate::application::ports::RepositoryError) -> Self {
        Self::RepositoryError(err.to_string())
    }
}
