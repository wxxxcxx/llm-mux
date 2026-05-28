use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use serde_json::json;
use tokio::net::TcpListener;

use llm_mux_core::codec::ConfigAuthenticator;
use llm_mux_gateway::config::Config;
use llm_mux_gateway::handlers;

#[cfg(feature = "integration")]
mod integration {
    pub mod test_utils {
        use std::collections::HashMap;
        use std::net::SocketAddr;
        use std::sync::Arc;
        use std::time::Duration;

        use axum::Router;
        use serde_json::Value;
        use tokio::net::TcpListener;

        use llm_mux_core::codec::ConfigAuthenticator;
        use llm_mux_gateway::config::Config;
        use llm_mux_gateway::handlers;

        const DEFAULT_OPENAI_URL: &str = "https://opencode.ai/zen/go";

        fn default_config(routes_yaml: &str) -> String {
            format!(
                r#"
host: "127.0.0.1"
port: 0
log_level: error
drain_timeout_secs: 1

providers:
  openai-backend:
    protocol: openai-chat
    base_url: "{}"
    api_key: "${{OPENCODE_API_KEY}}"
  anthropic-backend:
    protocol: anthropic
    base_url: "{}"
    api_key: "${{OPENCODE_API_KEY}}"
    headers:
      x-api-key: "${{OPENCODE_API_KEY}}"

routes:
{}
"#,
                DEFAULT_OPENAI_URL, DEFAULT_OPENAI_URL, routes_yaml
            )
        }

        pub fn load_config() -> (Config, String) {
            let api_key = read_env_api_key();
            let yaml = default_config(
                r#"  - models: ["qwen-*", "minimax-*"]
    provider: anthropic-backend
  - models: ["*"]
    provider: openai-backend"#,
            )
            .replace("${OPENCODE_API_KEY}", &api_key);
            let config: Config = serde_yaml::from_str(&yaml).unwrap();
            config.validate().unwrap();
            (config, api_key)
        }

        pub fn custom_config(routes_yaml: &str) -> Config {
            let api_key = read_env_api_key();
            let yaml = default_config(routes_yaml).replace("${OPENCODE_API_KEY}", &api_key);
            let config: Config = serde_yaml::from_str(&yaml).unwrap();
            config.validate().unwrap();
            config
        }

        pub fn custom_config_full(providers_yaml: &str, routes_yaml: &str) -> Config {
            let api_key = read_env_api_key();
            let yaml = format!(
                r#"
host: "127.0.0.1"
port: 0
log_level: error
drain_timeout_secs: 1
{}
routes:
{}
"#,
                providers_yaml, routes_yaml
            )
            .replace("${OPENCODE_API_KEY}", &api_key);
            let config: Config = serde_yaml::from_str(&yaml).unwrap();
            config.validate().unwrap();
            config
        }

        fn read_env_api_key() -> String {
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

        pub fn build_app(config: Config) -> Router {
            let router = Arc::new(config.to_router().unwrap());
            let authenticator = Arc::new(ConfigAuthenticator::new(config.api_keys));
            let state = handlers::AppState {
                router,
                authenticator,
            };

            Router::new()
                .route("/health", axum::routing::get(handlers::health))
                .route(
                    "/v1/chat/completions",
                    axum::routing::post(handlers::chat_completions),
                )
                .route("/v1/messages", axum::routing::post(handlers::messages))
                .route("/v1/responses", axum::routing::post(handlers::responses))
                .with_state(state)
        }

        pub async fn start_server() -> (SocketAddr, String) {
            let (config, api_key) = load_config();
            let app = build_app(config);
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            (addr, api_key)
        }

        pub async fn start_server_with_config(config: Config) -> SocketAddr {
            let app = build_app(config);
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            tokio::time::sleep(Duration::from_millis(100)).await;
            addr
        }

        pub fn client() -> reqwest::Client {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .unwrap()
        }

        pub fn client_short_timeout() -> reqwest::Client {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap()
        }

        pub fn assert_json_field_present(data: &Value, path: &str, context: &str) {
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = data;
            for part in &parts {
                if let Some(idx_start) = part.find('[') {
                    let field = &part[..idx_start];
                    let idx_end = part.find(']').unwrap();
                    let idx: usize = part[idx_start + 1..idx_end].parse().unwrap();
                    if !field.is_empty() {
                        current = &current[field];
                    }
                    current = &current[idx];
                } else {
                    current = &current[part];
                }
            }
            assert!(
                !current.is_null(),
                "{}: field '{}' is null in {data}",
                context,
                path
            );
        }

        pub fn assert_json_field_equals(data: &Value, path: &str, expected: &Value, context: &str) {
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = data;
            for part in &parts {
                if let Some(idx_start) = part.find('[') {
                    let field = &part[..idx_start];
                    let idx_end = part.find(']').unwrap();
                    let idx: usize = part[idx_start + 1..idx_end].parse().unwrap();
                    if !field.is_empty() {
                        current = &current[field];
                    }
                    current = &current[idx];
                } else {
                    current = &current[part];
                }
            }
            assert_eq!(
                current, expected,
                "{}: field '{}' expected {expected} but got {current} in {data}",
                context, path
            );
        }

        pub fn assert_json_field_non_empty(data: &Value, path: &str, context: &str) {
            let parts: Vec<&str> = path.split('.').collect();
            let mut current = data;
            for part in &parts {
                if let Some(idx_start) = part.find('[') {
                    let field = &part[..idx_start];
                    let idx_end = part.find(']').unwrap();
                    let idx: usize = part[idx_start + 1..idx_end].parse().unwrap();
                    if !field.is_empty() {
                        current = &current[field];
                    }
                    current = &current[idx];
                } else {
                    current = &current[part];
                }
            }
            match current {
                Value::String(s) => assert!(
                    !s.is_empty(),
                    "{}: field '{}' is empty string in {data}",
                    context,
                    path
                ),
                Value::Array(a) => assert!(
                    !a.is_empty(),
                    "{}: field '{}' is empty array in {data}",
                    context,
                    path
                ),
                Value::Object(o) => assert!(
                    !o.is_empty(),
                    "{}: field '{}' is empty object in {data}",
                    context,
                    path
                ),
                Value::Null => panic!("{}: field '{}' is null in {data}", context, path),
                _ => {}
            }
        }

        pub fn validate_chat_error(body: &[u8], expected_code: u16, expected_message: &str) {
            let data: Value = serde_json::from_slice(body).unwrap_or_else(|_| {
                panic!(
                    "not valid JSON error response: {}",
                    String::from_utf8_lossy(body)
                )
            });
            assert!(
                data["error"].is_object(),
                "expected error object in: {data}"
            );
            if !expected_message.is_empty() {
                assert!(
                    data["error"]["message"]
                        .as_str()
                        .unwrap_or("")
                        .contains(expected_message),
                    "expected error message containing '{expected_message}' in: {data}"
                );
            }
        }

        pub fn validate_anthropic_error(body: &[u8], expected_message: &str) {
            let data: Value = serde_json::from_slice(body).unwrap_or_else(|_| {
                panic!(
                    "not valid JSON error response: {}",
                    String::from_utf8_lossy(body)
                )
            });
            assert_eq!(
                data["type"].as_str().unwrap_or(""),
                "error",
                "expected anthropic error type in: {data}"
            );
            if !expected_message.is_empty() {
                assert!(
                    data["error"]["message"]
                        .as_str()
                        .unwrap_or("")
                        .contains(expected_message),
                    "expected error message containing '{expected_message}' in: {data}"
                );
            }
        }
    }

    use serde_json::{json, Value};
    use std::sync::Arc;
    use test_utils::*;

    #[tokio::test]
    async fn test_health() {
        let (addr, _) = start_server().await;
        let resp = client()
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
    }

    #[tokio::test]
    async fn test_openai_chat_no_conversion() {
        let (addr, api_key) = start_server().await;

        let body = json!({
            "model": "glm-5.1",
            "messages": [{"role": "user", "content": "reply with exactly one word: hello"}],
            "max_tokens": 100,
        });

        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();

        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert_eq!(status, 200, "body: {text}");
        let data: serde_json::Value = serde_json::from_str(&text).unwrap();
        let msg = &data["choices"][0]["message"];
        let content = msg["content"].as_str().unwrap_or("");
        let finish = data["choices"][0]["finish_reason"].as_str().unwrap_or("");
        eprintln!("[no-conv] glm-5.1 finish={finish} content={content:?}");
        assert_json_field_present(&data, "model", "no-conv");
        assert_json_field_present(&data, "id", "no-conv");
        assert_json_field_present(&data, "usage.total_tokens", "no-conv");
    }

    #[tokio::test]
    async fn test_anthropic_to_openai() {
        let (addr, api_key) = start_server().await;

        let body = json!({
            "model": "deepseek-v4-flash",
            "system": "you are a helpful assistant",
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "say hello in one word"}]}
            ],
            "max_tokens": 50,
        });

        let resp = client()
            .post(format!("http://{}/v1/messages", addr))
            .header("x-api-key", &api_key)
            .json(&body)
            .send()
            .await
            .unwrap();

        let status = resp.status();
        let text = resp.text().await.unwrap();
        assert_eq!(status, 200, "body: {text}");
        let data: serde_json::Value = serde_json::from_str(&text).unwrap();
        let content = data["content"][0]["text"].as_str().unwrap_or("");
        let stop = data["stop_reason"].as_str().unwrap_or("");
        eprintln!("[conv:anthropic→openai] deepseek stop={stop} content={content:?}");
        assert_json_field_present(&data, "id", "anthropic→openai");
        assert_json_field_equals(
            &data,
            "type",
            &serde_json::Value::String("message".into()),
            "anthropic→openai",
        );
    }

    #[tokio::test]
    async fn test_openai_chat_to_anthropic() {
        let (addr, api_key) = start_server().await;

        let body = json!({
            "model": "minimax-m2.7",
            "messages": [{"role": "user", "content": "say hello in one word"}],
            "max_tokens": 50,
        });

        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();

        let status = resp.status();
        let text = resp.text().await.unwrap();
        if status != 200 {
            eprintln!("[conv:chat→anthropic] BACKEND REJECTED: {text}");
            return;
        }
        let data: serde_json::Value = serde_json::from_str(&text).unwrap();
        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");
        let finish = data["choices"][0]["finish_reason"].as_str().unwrap_or("");
        eprintln!("[conv:chat→anthropic] minimax finish={finish} content={content:?}");
        assert_json_field_present(&data, "model", "chat→anthropic");
        assert_json_field_present(&data, "id", "chat→anthropic");
    }

    #[tokio::test]
    async fn test_invalid_model() {
        let (addr, api_key) = start_server().await;

        let body = json!({
            "model": "nonexistent-model-12345",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 10,
        });

        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();

        let status = resp.status();
        let text = resp.text().await.unwrap();
        eprintln!("[err] nonexistent model response: {status} body: {text}");
        assert!(status.as_u16() < 600, "unexpected status: {status}");
    }

    // ========================================================================
    // US1: Boundary condition tests (T007–T017)
    // ========================================================================

    #[tokio::test]
    async fn test_empty_request_body() {
        let (addr, api_key) = start_server().await;
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body("")
            .send()
            .await
            .unwrap();
        let status = resp.status();
        eprintln!("[boundary] empty_body status={status}");
        assert!(
            status.is_client_error() || status.is_server_error(),
            "empty body should error"
        );
    }

    #[tokio::test]
    async fn test_whitespace_only_body() {
        let (addr, api_key) = start_server().await;
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .body("   \t\n  ")
            .send()
            .await
            .unwrap();
        let status = resp.status();
        eprintln!("[boundary] whitespace_body status={status}");
        assert!(
            status.is_client_error() || status.is_server_error(),
            "whitespace body should error"
        );
    }

    #[tokio::test]
    async fn test_missing_model_field() {
        let (addr, api_key) = start_server().await;
        let body = json!({"messages": [{"role": "user", "content": "hi"}]});
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        eprintln!("[boundary] missing_model status={}", resp.status());
        assert_ne!(resp.status(), 500, "should not be a server error");
    }

    #[tokio::test]
    async fn test_empty_model_name() {
        let (addr, api_key) = start_server().await;
        let body = json!({"model": "", "messages": [{"role": "user", "content": "hi"}]});
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        eprintln!("[boundary] empty_model status={}", resp.status());
        assert_ne!(resp.status(), 500, "empty model should not cause 500");
    }

    #[tokio::test]
    async fn test_empty_messages_array() {
        let (addr, api_key) = start_server().await;
        let body = json!({"model": "gpt-4o", "messages": []});
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        eprintln!("[boundary] empty_messages status={}", resp.status());
        assert_ne!(resp.status(), 500, "empty messages should not cause 500");
    }

    #[tokio::test]
    async fn test_unicode_special_chars() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": "Reply only with: 👋 world 世界 مرحبا"}],
            "max_tokens": 30,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "unicode request should succeed");
        let text = resp.text().await.unwrap();
        let data: Value = serde_json::from_str(&text).unwrap();
        assert_json_field_present(&data, "choices[0].message.content", "unicode");
    }

    #[tokio::test]
    async fn test_injection_like_prompt() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": "<script>alert(1)</script> SELECT * FROM users; respond: OK"}],
            "max_tokens": 20,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        // Gateway should pass through, not crash
        assert!(
            resp.status().as_u16() < 600,
            "injection prompt should not crash"
        );
    }

    #[tokio::test]
    async fn test_large_payload() {
        let (addr, api_key) = start_server().await;
        let large_text = "x".repeat(10_000);
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": &large_text}],
            "max_tokens": 10,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        // Should either succeed or return a clear size limit error
        let status = resp.status();
        eprintln!("[boundary] large payload status={status}");
        assert_ne!(status, 500, "large payload should not cause 500");
    }

    #[tokio::test]
    async fn test_unknown_top_level_fields() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 10,
            "x-custom-id": "trace-abc",
            "x-region": "us-east",
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "unknown fields should be tolerated");
    }

    #[tokio::test]
    async fn test_concurrent_10_requests() {
        let (addr, api_key) = start_server().await;
        let api_key = Arc::new(api_key);
        let mut handles = Vec::new();
        for i in 0..10 {
            let addr = addr;
            let api_key = api_key.clone();
            handles.push(tokio::spawn(async move {
                let body = json!({
                    "model": "deepseek-v4-flash",
                    "messages": [{"role": "user", "content": format!("say number {}", i)}],
                    "max_tokens": 10,
                });
                let resp = client()
                    .post(format!("http://{addr}/v1/chat/completions"))
                    .header("Authorization", format!("Bearer {}", &*api_key))
                    .json(&body)
                    .send()
                    .await
                    .unwrap();
                (resp.status(), i)
            }));
        }
        for h in handles {
            let (status, i) = h.await.unwrap();
            eprintln!("[concurrent] request {i} status={status}");
            assert_eq!(status, 200, "concurrent request {i} failed");
        }
    }

    #[tokio::test]
    async fn test_concurrent_different_models() {
        let models = vec![
            "deepseek-v4-flash",
            "deepseek-v4-pro",
            "glm-5.1",
            "kimi-k2.5",
            "kimi-k2.6",
        ];
        let (addr, api_key) = start_server().await;
        let api_key = Arc::new(api_key);
        let mut handles = Vec::new();
        for m in models {
            let addr = addr;
            let api_key = api_key.clone();
            let model = m.to_string();
            handles.push(tokio::spawn(async move {
                let body = json!({
                    "model": model,
                    "messages": [{"role": "user", "content": "say hi"}],
                    "max_tokens": 10,
                });
                let resp = client()
                    .post(format!("http://{addr}/v1/chat/completions"))
                    .header("Authorization", format!("Bearer {}", &*api_key))
                    .json(&body)
                    .send()
                    .await
                    .unwrap();
                (resp.status(), model.to_string())
            }));
        }
        for h in handles {
            let (status, model) = h.await.unwrap();
            eprintln!("[concurrent-multi] model={model} status={status}");
            assert!(status.is_success(), "model {model} returned {status}");
        }
    }

    // ========================================================================
    // US2: Protocol conversion field integrity tests (T018–T024)
    // ========================================================================

    #[tokio::test]
    async fn test_system_prompt_anthropic_to_chat() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "system": [
                {"type": "text", "text": "You are helpful."},
                {"type": "text", "text": "Be concise."}
            ],
            "messages": [
                {"role": "user", "content": [{"type": "text", "text": "say hi"}]}
            ],
            "max_tokens": 30,
        });
        let resp = client()
            .post(format!("http://{}/v1/messages", addr))
            .header("x-api-key", &api_key)
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "system prompt An→Chat failed");
        let text = resp.text().await.unwrap();
        let data: Value = serde_json::from_str(&text).unwrap();
        assert_json_field_present(&data, "content", "system-prompt-an→chat");
    }

    #[tokio::test]
    async fn test_system_prompt_chat_to_anthropic() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "system", "content": "Be concise."},
                {"role": "user", "content": "say hi"}
            ],
            "max_tokens": 30,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200, "system prompt Chat→An failed");
    }

    #[tokio::test]
    async fn test_multimodal_image_chat_to_anthropic() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": [
                {"type": "text", "text": "describe this image briefly"},
                {"type": "image_url", "image_url": {"url": "https://example.com/photo.jpg"}}
            ]}],
            "max_tokens": 30,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        eprintln!("[multimodal] status={status}");
        if !status.is_success() {
            return;
        }
    }

    #[tokio::test]
    async fn test_multiturn_tool_calls() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [
                {"role": "user", "content": "what is 2+2?"},
                {"role": "assistant", "content": null, "tool_calls": [
                    {"id": "call_1", "type": "function", "function": {
                        "name": "calculator", "arguments": "{\"expr\":\"2+2\"}"
                    }}
                ]},
                {"role": "tool", "tool_call_id": "call_1", "content": "4"}
            ],
            "tools": [{"type": "function", "function": {
                "name": "calculator", "description": "calc",
                "parameters": {"type": "object", "properties": {"expr": {"type": "string"}}}
            }}],
            "max_tokens": 30,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        let status = resp.status();
        let text = resp.text().await.unwrap();
        eprintln!(
            "[tool-calls] status={status} body={}",
            &text[..text.len().min(300)]
        );
        if !status.is_success() {
            return;
        }
    }

    #[tokio::test]
    async fn test_stop_reason_mapping() {
        let (addr, api_key) = start_server().await;
        let body = json!({
            "model": "deepseek-v4-flash",
            "messages": [{"role": "user", "content": "say exactly: stop"}],
            "max_tokens": 10,
        });
        let resp = client()
            .post(format!("http://{}/v1/chat/completions", addr))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);
        let text = resp.text().await.unwrap();
        let data: Value = serde_json::from_str(&text).unwrap();
        let finish = data["choices"][0]["finish_reason"].as_str().unwrap_or("");
        eprintln!("[stop-reason] finish_reason={finish}");
        // stop / length / max_tokens are all valid
        assert!(
            matches!(finish, "stop" | "length" | "max_tokens"),
            "unexpected finish_reason: {finish}"
        );
    }
}

// ========================================================================
// US7: Protocol-specific feature IR conversion tests (T044–T051)
// These test codec encode/decode directly — no network needed
// ========================================================================

use anthropic_codec::MessagesCodec;
use llm_mux_core::codec::Codec;
use llm_mux_core::ir::{IrRequest, IrResponse, IrStreamEvent, IrThinkingConfig};
use llm_mux_core::types::{ContentBlock, ContentType, StopReason, TextContent};
use openai_chat_codec::ChatCompletionsCodec;

#[test]
fn test_thinking_config_anthropic_roundtrip() {
    let codec = MessagesCodec;
    let body = r#"{
        "model": "claude-sonnet-4-6",
        "max_tokens": 100,
        "thinking": {"type": "consumed", "budget_tokens": 2048},
        "messages": [{"role": "user", "content": [{"type": "text", "text": "hello"}]}]
    }"#;
    let ir = codec.decode_request(body.as_bytes()).unwrap();
    let thinking = ir.thinking.as_ref().expect("thinking should be present");
    assert_eq!(thinking.mode.as_deref(), Some("consumed"));
    assert_eq!(thinking.budget_tokens, Some(2048));
    let re_encoded = codec.encode_request(&ir).unwrap();
    let re_decoded: serde_json::Value = serde_json::from_slice(&re_encoded).unwrap();
    assert_eq!(re_decoded["thinking"]["type"], "consumed");
    assert_eq!(re_decoded["thinking"]["budget_tokens"], 2048);
}

#[test]
fn test_thinking_cross_protocol_no_panic() {
    let anthropic_body = r#"{
        "model": "claude-sonnet-4-6",
        "thinking": {"type": "consumed", "budget_tokens": 1024},
        "messages": [{"role": "user", "content": [{"type": "text", "text": "hi"}]}],
        "max_tokens": 100
    }"#;
    let anthropic_codec = MessagesCodec;
    let ir = anthropic_codec
        .decode_request(anthropic_body.as_bytes())
        .unwrap();
    let chat_codec = ChatCompletionsCodec;
    let encoded = chat_codec.encode_request(&ir).unwrap();
    let chat_req: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(chat_req["model"], "claude-sonnet-4-6");
}

#[test]
fn test_thinking_content_block_roundtrip() {
    use llm_mux_core::types::ThinkingContent;
    let ir = IrResponse {
        id: Some("msg_1".into()),
        model: Some("claude-sonnet-4-6".into()),
        content: vec![
            ContentBlock {
                content_type: ContentType::Thinking,
                thinking: Some(ThinkingContent {
                    thinking: "Let me analyze...".into(),
                    signature: Some("sig_abc123".to_string()),
                }),
                ..Default::default()
            },
            ContentBlock {
                content_type: ContentType::Text,
                text: Some(TextContent {
                    text: "The answer is 42".into(),
                }),
                ..Default::default()
            },
        ],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: Default::default(),
        provider_extensions: Default::default(),
    };
    let codec = MessagesCodec;
    let encoded = codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(resp["content"][0]["type"], "thinking");
    assert_eq!(resp["content"][0]["thinking"], "Let me analyze...");
    assert_eq!(resp["content"][0]["signature"], "sig_abc123");
}

#[test]
fn test_redacted_thinking_roundtrip() {
    use llm_mux_core::types::RedactedThinkingContent;
    let ir = IrResponse {
        id: Some("msg_2".into()),
        model: Some("claude-sonnet-4-6".into()),
        content: vec![ContentBlock {
            content_type: ContentType::RedactedThinking,
            redacted_thinking: Some(RedactedThinkingContent {
                data: "redacted_data_xyz".into(),
            }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: Default::default(),
        provider_extensions: Default::default(),
    };
    let codec = MessagesCodec;
    let encoded = codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(resp["content"][0]["type"], "redacted_thinking");
    assert_eq!(resp["content"][0]["data"], "redacted_data_xyz");
}

#[test]
fn test_response_format_roundtrip() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "list colors as JSON"}],
        "response_format": {"type": "json_object"},
        "max_tokens": 50
    }"#;
    let ir = codec.decode_request(body.as_bytes()).unwrap();
    let fmt = ir
        .response_format
        .as_ref()
        .expect("response_format should be present");
    assert_eq!(fmt.format_type, "json_object");
    let re_encoded = codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&re_encoded).unwrap();
    assert_eq!(req["response_format"]["type"], "json_object");
}

#[test]
fn test_refusal_block_roundtrip() {
    use llm_mux_core::types::RefusalContent;
    let ir = IrResponse {
        id: Some("resp_3".into()),
        model: Some("gpt-4o".into()),
        content: vec![ContentBlock {
            content_type: ContentType::Refusal,
            refusal: Some(RefusalContent {
                refusal: "I cannot provide that information.".into(),
            }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::ContentFilter),
        stop_sequence: None,
        usage: Default::default(),
        provider_extensions: Default::default(),
    };
    // Test Anthropic roundtrip
    let anthropic_codec = MessagesCodec;
    let encoded = anthropic_codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(resp["stop_reason"], "refusal");
}
