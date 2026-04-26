use axum::{Json, Extension};
use serde::Serialize;
use crate::engine::state::SharedEngineState;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub engine: String,
}

/// 健康检查
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        engine: "QuantEngine Futures".to_string(),
    })
}

/// 引擎状态
pub async fn engine_status(Extension(state): Extension<SharedEngineState>) -> Json<serde_json::Value> {
    let st = state.read().await;
    Json(serde_json::to_value(&*st).unwrap_or(serde_json::json!({})))
}
