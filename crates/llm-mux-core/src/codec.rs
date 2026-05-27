use std::collections::HashMap;

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
    #[error("stream event error: {0}")]
    Stream(String),
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

    /// Encode an IR request into the external protocol format.
    fn encode_request(&self, request: &IrRequest) -> Result<Vec<u8>, CodecError>;

    /// Encode an IR response back to the external protocol format.
    fn encode_response(&self, response: &IrResponse) -> Result<Vec<u8>, CodecError>;

    /// Decode a raw SSE event into an IR stream event.
    fn decode_stream_event(
        &self,
        event_type: Option<&str>,
        data: &str,
    ) -> Result<IrStreamEvent, CodecError>;

    /// Encode an IR stream event into an SSE data line.
    fn encode_stream_event(&self, event: &IrStreamEvent) -> Result<String, CodecError>;

    /// Known top-level fields for this protocol (used for merging unknown fields).
    fn known_fields(&self) -> &HashMap<String, bool>;

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
    pub protocol: Protocol,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
}

impl Router for FixedRouter {
    fn route(&self, _info: &RouteInfo) -> Result<RouteResult, CodecError> {
        Ok(RouteResult {
            protocol: self.protocol,
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
    fn convert_request(
        &self,
        request: &mut IrRequest,
        target: &RouteResult,
    );

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
