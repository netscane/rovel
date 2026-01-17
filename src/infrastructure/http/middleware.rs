//! HTTP Middleware
//!
//! HTTP 状态码错误日志中间件

use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};

/// HTTP 状态码错误日志中间件
///
/// 拦截 HTTP 响应，当状态码为 4xx 或 5xx 时记录日志
/// 注意：业务错误（errno != 0）在 AppError::into_response() 中记录
pub async fn error_logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;
    let status = response.status();

    if status.is_server_error() {
        tracing::error!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            "HTTP server error"
        );
    } else if status.is_client_error() {
        tracing::warn!(
            method = %method,
            uri = %uri,
            status = %status.as_u16(),
            "HTTP client error"
        );
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
        Router,
    };
    use tower::util::ServiceExt;

    async fn ok_handler() -> &'static str {
        "OK"
    }

    async fn not_found_handler() -> StatusCode {
        StatusCode::NOT_FOUND
    }

    async fn error_handler() -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    fn create_test_router() -> Router {
        Router::new()
            .route("/ok", get(ok_handler))
            .route("/not-found", get(not_found_handler))
            .route("/error", get(error_handler))
            .layer(axum::middleware::from_fn(error_logging_middleware))
    }

    #[tokio::test]
    async fn test_ok_response_no_log() {
        let app = create_test_router();
        let request = HttpRequest::builder()
            .uri("/ok")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_client_error_logs_warning() {
        let app = create_test_router();
        let request = HttpRequest::builder()
            .uri("/not-found")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_server_error_logs_error() {
        let app = create_test_router();
        let request = HttpRequest::builder()
            .uri("/error")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
