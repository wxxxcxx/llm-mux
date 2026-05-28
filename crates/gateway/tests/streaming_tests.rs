use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

use llm_mux_core::codec::ConfigAuthenticator;
use llm_mux_gateway::config::Config;
use llm_mux_gateway::handlers;

#[cfg(feature = "integration")]
mod streaming {
    use std::collections::HashMap;

    use super::*;

    const DEFAULT_OPENAI_URL: &str = "https://opencode.ai/zen/go";

    fn read_api_key() -> String {
        let env_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join(".env");
        let env_content = std::fs::read_to_string(&env_path)
            .unwrap_or_else(|_| panic!("missing .env at: {}", env_path.display()));
        for line in env_content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                std::env::set_var(k.trim(), v.trim());
            }
        }
        std::env::var("OPENCODE_API_KEY").expect("OPENCODE_API_KEY must be set in .env")
    }

    fn default_config(routes_yaml: &str) -> Config {
        let api_key = read_api_key();
        let yaml = format!(
            r#"
host: "127.0.0.1"
port: 0
log_level: error
drain_timeout_secs: 1

providers:
  openai-backend:
    protocol: openai-chat
    base_url: "{}"
    api_key: "${{API_KEY}}"
  anthropic-backend:
    protocol: anthropic
    base_url: "{}"
    api_key: "${{API_KEY}}"
    headers:
      x-api-key: "${{API_KEY}}"

routes:
{}
"#,
            DEFAULT_OPENAI_URL, DEFAULT_OPENAI_URL, routes_yaml
        )
        .replace("${{API_KEY}}", &api_key);
        let config: Config = serde_yaml::from_str(&yaml).unwrap();
        config.validate().unwrap();
        config
    }

    fn client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .unwrap()
    }

    async fn start_server_with_config(config: Config) -> (SocketAddr, String) {
        let api_key = read_api_key();
        let router = Arc::new(config.to_router().unwrap());
        let authenticator = Arc::new(ConfigAuthenticator::new(config.api_keys));
        let state = handlers::AppState {
            router,
            authenticator,
        };

        let app = Router::new()
            .route("/health", axum::routing::get(handlers::health))
            .route(
                "/v1/chat/completions",
                axum::routing::post(handlers::chat_completions),
            )
            .route("/v1/messages", axum::routing::post(handlers::messages))
            .route("/v1/responses", axum::routing::post(handlers::responses))
            .with_state(state);

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        tokio::time::sleep(Duration::from_millis(100)).await;
        (addr, api_key)
    }

    fn default_routes() -> &'static str {
        r#"  - models: ["qwen-*", "minimax-*"]
    provider: anthropic-backend
  - models: ["*"]
    provider: openai-backend"#
    }

    /// SSE validation: check that a stream response conforms to SSE spec
    async fn validate_sse_basics(resp: reqwest::Response) -> (Vec<Value>, bool) {
        let mut saw_done = false;
        let mut events: Vec<Value> = Vec::new();
        let mut buf = String::new();

        let bytes = resp.bytes().await.unwrap();
        buf = String::from_utf8_lossy(&bytes).to_string();

        for chunk in buf.split("\n\n") {
            let chunk = chunk.trim();
            if chunk.is_empty() {
                continue;
            }
            for line in chunk.lines() {
                let line = line.trim();
                if line == "data: [DONE]" {
                    saw_done = true;
                    continue;
                }
                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(val) = serde_json::from_str::<Value>(data) {
                        events.push(val);
                    }
                }
            }
        }
        (events, saw_done)
    }

    /// Verify an event sequence against expected event types
    fn assert_event_sequence(events: &[Value], expected_types: &[&str], label: &str) {
        assert!(
            events.len() >= expected_types.len(),
            "[{label}] expected at least {} events, got {}: {events:?}",
            expected_types.len(),
            events.len()
        );
        for (i, expected) in expected_types.iter().enumerate() {
            let event = &events[i];
            match *expected {
                "delta" => {
                    assert!(
                        event["choices"][0]["delta"].is_object(),
                        "[{label}] event[{i}] expected delta: {event}"
                    );
                }
                "content_block_start" => {
                    let obj = &event;
                    assert!(
                        obj["type"] == "content_block_start"
                            || obj["event"]["type"] == "content_block_start",
                        "[{label}] event[{i}] expected content_block_start: {event}"
                    );
                }
                "message_start" => {
                    assert!(
                        event["type"] == "message_start",
                        "[{label}] event[{i}] expected message_start: {event}"
                    );
                }
                "message_stop" => {
                    assert!(
                        event["type"] == "message_stop",
                        "[{label}] event[{i}] expected message_stop: {event}"
                    );
                }
                _ => {
                    eprintln!("[{label}] unvalidated event[{i}] type={expected}: {event}");
                }
            }
        }
    }

    // ========================================================================
    // US5: Stream event sequence tests — to be implemented in Phase 6
    // ========================================================================

    // ========================================================================
    // US6: SSE format compliance tests — to be implemented in Phase 7
    // ========================================================================
}
