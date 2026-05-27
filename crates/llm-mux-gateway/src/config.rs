//! 配置文件解析与管理。
//!
//! 支持 YAML 配置文件加载、环境变量展开、校验和展示。

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use llm_mux_core::codec::{ConfigurableRouter, ProviderConfig, RouteRule};
use llm_mux_core::types::Protocol;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    #[serde(default = "default_drain_timeout")]
    pub drain_timeout_secs: u64,
    #[serde(default)]
    pub api_keys: Vec<String>,
    pub providers: HashMap<String, ProviderDef>,
    pub routes: Vec<RouteDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDef {
    pub protocol: String,
    pub base_url: String,
    pub api_key: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub model_mapping: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDef {
    pub models: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_tools: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_media: Option<bool>,
    pub provider: String,
}

fn default_host() -> String {
    "127.0.0.1".into()
}
fn default_port() -> u16 {
    8080
}
fn default_log_level() -> String {
    "info".into()
}
fn default_drain_timeout() -> u64 {
    30
}

fn expand_env(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next();
            let mut var = String::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next();
                    break;
                }
                var.push(c);
                chars.next();
            }
            let val = std::env::var(&var).unwrap_or_default();
            result.push_str(&val);
        } else {
            result.push(ch);
        }
    }
    result
}

fn parse_protocol(s: &str) -> Result<Protocol, String> {
    match s {
        "openai-chat" => Ok(Protocol::OpenAiChat),
        "openai-responses" => Ok(Protocol::OpenAiResponses),
        "anthropic" => Ok(Protocol::Anthropic),
        _ => Err(format!("unknown protocol: {s}")),
    }
}

impl Config {
    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let expanded = expand_env(&content);
        let config: Config = serde_yaml::from_str(&expanded)?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.providers.is_empty() {
            return Err("at least one provider is required".into());
        }
        if self.routes.is_empty() {
            return Err("at least one route is required".into());
        }
        for (name, p) in &self.providers {
            parse_protocol(&p.protocol).map_err(|e| format!("provider '{name}': {e}"))?;
        }
        for (i, r) in self.routes.iter().enumerate() {
            if !self.providers.contains_key(&r.provider) {
                return Err(format!(
                    "route[{i}]: provider '{}' not found in providers",
                    r.provider
                ));
            }
        }
        let last = self.routes.last().unwrap();
        if last.models != ["*"] {
            return Err("last route must be models: [\"*\"] as fallback".into());
        }
        if last.protocol.is_some()
            || last.stream.is_some()
            || last.has_tools.is_some()
            || last.has_media.is_some()
        {
            return Err("fallback route must not have extra conditions".into());
        }
        Ok(())
    }

    pub fn to_router(&self) -> Result<ConfigurableRouter, String> {
        let mut providers = HashMap::new();
        for (name, p) in &self.providers {
            let protocol = parse_protocol(&p.protocol)?;
            let mut model_mapping = HashMap::new();
            for (k, v) in &p.model_mapping {
                model_mapping.insert(k.clone(), v.clone());
            }
            providers.insert(
                name.clone(),
                ProviderConfig {
                    protocol,
                    base_url: p.base_url.clone(),
                    api_key: p.api_key.clone(),
                    headers: p.headers.clone(),
                    model_mapping,
                },
            );
        }
        let mut rules = Vec::new();
        for r in &self.routes {
            rules.push(RouteRule {
                models: r.models.clone(),
                protocol: r.protocol.as_deref().map(parse_protocol).transpose()?,
                stream: r.stream,
                has_tools: r.has_tools,
                has_media: r.has_media,
                provider: r.provider.clone(),
            });
        }
        Ok(ConfigurableRouter::new(rules, providers))
    }

    pub fn display(&self) -> String {
        let mut cfg = self.clone();
        for p in cfg.providers.values_mut() {
            if p.api_key.len() > 8 {
                p.api_key = format!("{}***", &p.api_key[..8]);
            }
        }
        for p in cfg.providers.values_mut() {
            for v in p.headers.values_mut() {
                if v.len() > 8 {
                    *v = format!("{}***", &v[..8]);
                }
            }
        }
        serde_yaml::to_string(&cfg).unwrap_or_else(|e| format!("error: {e}"))
    }

    pub fn generate_default() -> String {
        include_str!("../../../config.example.yaml").to_string()
    }
}
