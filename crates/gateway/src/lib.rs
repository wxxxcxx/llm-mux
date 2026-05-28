//! LLM Mux Gateway — 高性能 LLM API 协议互转网关
//!
//! 提供 HTTP 服务、CLI 管理和库嵌入三种交付形态。

pub mod config;
pub mod handlers;
pub mod middleware;
pub mod server;

pub use config::Config;
pub use server::Server;

use std::io::IsTerminal;

use tracing_subscriber::EnvFilter;

pub fn init_tracing(log_level: &str) {
    let filter = EnvFilter::new(log_level)
        .add_directive("llm_mux_gateway::handlers=debug".parse().unwrap());

    let is_tty = std::io::stdout().is_terminal();
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false);

    if is_tty {
        subscriber.compact().init();
    } else {
        subscriber.json().init();
    }
}
