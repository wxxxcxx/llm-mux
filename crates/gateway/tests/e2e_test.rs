use std::sync::Arc;
use llm_mux_core::codec::{Authenticator, ConfigAuthenticator};
use llm_mux_gateway::config::Config;

/// 验证 AppState 可构造
#[test]
fn test_app_state_creation() {
    let config = Config {
        host: "127.0.0.1".into(),
        port: 8081,
        log_level: "info".into(),
        drain_timeout_secs: 30,
        api_keys: vec!["test-key".into()],
        providers: Default::default(),
        routes: vec![],
    };
    let auth = Arc::new(ConfigAuthenticator::new(config.api_keys.clone()));
    assert!(auth.authenticate("test-key").is_ok());
    assert!(auth.authenticate("wrong-key").is_err());
}

/// 验证 genai Client 可构造
#[test]
fn test_genai_client_creation() {
    let _client = genai::Client::default();
}
