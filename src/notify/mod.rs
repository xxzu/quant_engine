//! 通知系统

use anyhow::Result;
use async_trait::async_trait;

/// 通知消息
#[derive(Debug, Clone)]
pub struct NotifyMessage {
    pub title: String,
    pub body: String,
    pub level: NotifyLevel,
}

#[derive(Debug, Clone)]
pub enum NotifyLevel {
    Info,
    Warning,
    Error,
}

/// 通知器 Trait
#[async_trait]
pub trait Notifier: Send + Sync {
    async fn send(&self, msg: &NotifyMessage) -> Result<()>;
}

/// 日志通知器（默认，输出到日志）
pub struct LogNotifier;

#[async_trait]
impl Notifier for LogNotifier {
    async fn send(&self, msg: &NotifyMessage) -> Result<()> {
        match msg.level {
            NotifyLevel::Info => tracing::info!("[通知] {}: {}", msg.title, msg.body),
            NotifyLevel::Warning => tracing::warn!("[通知] {}: {}", msg.title, msg.body),
            NotifyLevel::Error => tracing::error!("[通知] {}: {}", msg.title, msg.body),
        }
        Ok(())
    }
}
