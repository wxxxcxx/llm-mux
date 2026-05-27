//! LLM Mux Gateway — 高性能 LLM API 协议互转网关
//!
//! 提供 HTTP 服务、CLI 管理和库嵌入三种交付形态。

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod server;
pub mod sse;

pub use config::Config;
pub use server::Server;

use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::EnvFilter;

pub fn init_tracing(log_level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .json()
        .with_target(false)
        .with_current_span(false)
        .init();
}
