use std::collections::{HashMap, HashSet};

use crate::ir::{IrRequest, IrResponse, IrStreamEvent};
use crate::types::Protocol;

/// Core error type for codec operations.
#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error("failed to decode request: {0}")]
    Decode(String),
    #[error("failed to encode response: {0}")]
    Encode(String),
    #[error("unsupported protocol: {0:?}")]
    UnsupportedProtocol(Protocol),
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("no route matched for model: {0}")]
    RouteNotFound(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// An inbound codec decodes external protocol requests into IR,
/// and encodes IR responses back to the external protocol.
pub trait Codec: Send + Sync {
    /// The protocol this codec handles.
    fn protocol(&self) -> Protocol;

    /// Decode a raw request body into the unified IR.
    fn decode_request(&self, body: &[u8]) -> Result<IrRequest, CodecError>;

    /// Encode an IR response back to the external protocol format.
    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError>;

    /// Write an error response in the protocol's native format.
    fn write_error(&self, status_code: u16, message: &str) -> Vec<u8>;
}

/// Routing information provided to the Router for decision-making.
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub request_id: String,
    pub model: String,
    pub inbound_protocol: Protocol,
    pub stream: bool,
    pub has_tools: bool,
    pub has_media: bool,
    pub api_key: Option<String>,
}

/// The outbound target decided by the Router.
#[derive(Debug, Clone)]
pub struct RouteResult {
    pub protocol: Protocol,
    pub format: genai::adapter::AdapterKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub proxy_url: Option<String>,
    pub headers: HashMap<String, String>,
}

/// Router determines the outbound target for each request.
pub trait Router: Send + Sync {
    /// Decide where to route this request.
    fn route(&self, info: &RouteInfo) -> Result<RouteResult, CodecError>;

    /// Called when a send attempt fails. Return a fallback route or the error.
    fn on_error(
        &self,
        _info: &RouteInfo,
        _target: &RouteResult,
        err: &CodecError,
    ) -> Result<RouteResult, CodecError> {
        Err(CodecError::Decode(format!("routing error: {err}")))
    }

    /// Called after a successful send.
    fn on_success(&self, _info: &RouteInfo, _target: &RouteResult) {}
}

/// A simple router that always routes to the same target.
pub struct FixedRouter {
    pub format: genai::adapter::AdapterKind,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Router for FixedRouter {
    fn route(&self, _info: &RouteInfo) -> Result<RouteResult, CodecError> {
        Ok(RouteResult {
            protocol: match self.format {
                genai::adapter::AdapterKind::Anthropic => Protocol::Anthropic,
                genai::adapter::AdapterKind::OpenAIResp => Protocol::OpenAiResponses,
                _ => Protocol::OpenAiChat,
            },
            format: self.format,
            base_url: self.base_url.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            proxy_url: None,
            headers: HashMap::new(),
        })
    }
}

/// Authenticator validates inbound API keys.
pub trait Authenticator: Send + Sync {
    fn authenticate(&self, api_key: &str) -> Result<(), CodecError>;
}

/// A converter enriches or transforms IR between inbound and outbound protocols.
/// This is where cross-protocol field mapping and vendor-specific adjustments happen.
pub trait Converter: Send + Sync {
    /// Transform the IR request for the target outbound protocol.
    /// Called after routing, before encoding.
    fn convert_request(&self, request: &mut IrRequest, target: &RouteResult);

    /// Transform the IR response from the outbound protocol back to the original inbound protocol.
    fn convert_response(
        &self,
        response: &mut IrResponse,
        source_protocol: Protocol,
        target_protocol: Protocol,
    );

    /// Transform a streaming IR event.
    fn convert_stream_event(
        &self,
        event: &mut IrStreamEvent,
        source_protocol: Protocol,
        target_protocol: Protocol,
    );
}

/// A no-op converter that passes IR through unchanged.
pub struct NoopConverter;

impl Converter for NoopConverter {
    fn convert_request(&self, _request: &mut IrRequest, _target: &RouteResult) {}

    fn convert_response(
        &self,
        _response: &mut IrResponse,
        _source_protocol: Protocol,
        _target_protocol: Protocol,
    ) {
    }

    fn convert_stream_event(
        &self,
        _event: &mut IrStreamEvent,
        _source_protocol: Protocol,
        _target_protocol: Protocol,
    ) {
    }
}

/// API Key based authenticator using a pre-configured key set.
pub struct ConfigAuthenticator {
    keys: HashSet<String>,
}

impl ConfigAuthenticator {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }
}

impl Authenticator for ConfigAuthenticator {
    fn authenticate(&self, api_key: &str) -> Result<(), CodecError> {
        if self.keys.is_empty() || self.keys.contains(api_key) {
            Ok(())
        } else {
            Err(CodecError::Auth("invalid API key".into()))
        }
    }
}

/// Provider configuration with optional model mapping.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub format: genai::adapter::AdapterKind,
    pub base_url: String,
    pub api_key: String,
    pub headers: HashMap<String, String>,
    pub model_mapping: HashMap<String, String>,
}

/// Route rule with multi-condition matching (AND semantics).
#[derive(Debug, Clone)]
pub struct RouteRule {
    pub models: Vec<String>,
    pub protocol: Option<Protocol>,
    pub stream: Option<bool>,
    pub has_tools: Option<bool>,
    pub has_media: Option<bool>,
    pub provider: String,
}

/// A router that matches routes top-to-bottom with first-match-wins.
pub struct ConfigurableRouter {
    rules: Vec<RouteRule>,
    providers: HashMap<String, ProviderConfig>,
}

impl ConfigurableRouter {
    pub fn new(rules: Vec<RouteRule>, providers: HashMap<String, ProviderConfig>) -> Self {
        Self { rules, providers }
    }

    fn wildcard_match(pattern: &str, value: &str) -> bool {
        if pattern == "*" {
            return true;
        }
        if !pattern.contains('*') && !pattern.contains('?') {
            return pattern == value;
        }
        let mut pi = 0;
        let mut vi = 0;
        let pb = pattern.as_bytes();
        let vb = value.as_bytes();
        let mut star = None;
        while vi < vb.len() {
            if pi < pb.len() && (pb[pi] == b'*') {
                star = Some(pi);
                pi += 1;
            } else if pi < pb.len() && (pb[pi] == b'?' || pb[pi] == vb[vi]) {
                pi += 1;
                vi += 1;
            } else if let Some(s) = star {
                pi = s + 1;
                vi += 1;
            } else {
                return false;
            }
        }
        while pi < pb.len() && pb[pi] == b'*' {
            pi += 1;
        }
        pi == pb.len()
    }
}

impl Router for ConfigurableRouter {
    fn route(&self, info: &RouteInfo) -> Result<RouteResult, CodecError> {
        for rule in &self.rules {
            let model_match = rule
                .models
                .iter()
                .any(|p| Self::wildcard_match(p, &info.model));
            if !model_match {
                continue;
            }
            if let Some(ref proto) = rule.protocol {
                if *proto != info.inbound_protocol {
                    continue;
                }
            }
            if let Some(stream) = rule.stream {
                if stream != info.stream {
                    continue;
                }
            }
            if let Some(has_tools) = rule.has_tools {
                if has_tools != info.has_tools {
                    continue;
                }
            }
            if let Some(has_media) = rule.has_media {
                if has_media != info.has_media {
                    continue;
                }
            }

            let provider = self.providers.get(&rule.provider).ok_or_else(|| {
                CodecError::RouteNotFound(format!(
                    "provider '{}' not found for model '{}'",
                    rule.provider, info.model
                ))
            })?;

            let mapped_model = provider
                .model_mapping
                .get(&info.model)
                .or_else(|| {
                    provider
                        .model_mapping
                        .iter()
                        .filter(|(k, _)| Self::wildcard_match(k, &info.model))
                        .max_by_key(|(k, _)| k.len())
                        .map(|(_, v)| v)
                })
                .cloned()
                .unwrap_or_else(|| info.model.clone());

            return Ok(RouteResult {
                protocol: match provider.format {
                    genai::adapter::AdapterKind::Anthropic => Protocol::Anthropic,
                    genai::adapter::AdapterKind::OpenAIResp => Protocol::OpenAiResponses,
                    _ => Protocol::OpenAiChat,
                },
                format: provider.format,
                base_url: provider.base_url.clone(),
                api_key: provider.api_key.clone(),
                model: mapped_model,
                proxy_url: None,
                headers: provider.headers.clone(),
            });
        }

        Err(CodecError::RouteNotFound(format!(
            "no route matched for model '{}'",
            info.model
        )))
    }
}
