//! 认证中间件

use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// API Key 认证中间件
pub async fn auth_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // 获取 API Key
    let api_key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

    // TODO: 验证 API Key
    // 目前跳过验证，仅作示例
    if let Some(_key) = api_key {
        // 验证逻辑
    }

    Ok(next.run(request).await)
}
