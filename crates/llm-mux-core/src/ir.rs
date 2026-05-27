use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use crate::types::{ContentBlock, Protocol};

/// A single message turn in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrMessage {
    pub role: crate::types::Role,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentBlock>,
}

/// Tool definition available to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub extra_fields: HashMap<String, JsonValue>,
}

/// Controls how the model selects tools.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tool_names: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_parallel_calls: Option<bool>,
}

/// Extended thinking / reasoning configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
}

/// Output format constraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonValue>,
}

/// Provider-specific extension fields, keyed by vendor-namespaced string.
/// e.g. "anthropic/thinking", "openai/reasoning"
pub type ProviderExtensions = HashMap<String, JsonValue>;

/// Unified intermediate representation of an LLM API request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrRequest {
    pub model: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<IrMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system_prompt: Vec<ContentBlock>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<IrTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<IrToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<IrThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<IrResponseFormat>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub provider_extensions: ProviderExtensions,
    /// The protocol this request was originally received as.
    #[serde(skip)]
    pub inbound_protocol: Protocol,
    /// Original model name before any rewriting.
    #[serde(skip)]
    pub original_model: Option<String>,
    /// Protocol-specific opaque fields for same-protocol roundtrips.
    /// Managed internally — do not modify from outside the codec.
    #[serde(skip)]
    pub raw_extra: HashMap<String, JsonValue>,
    /// Extra fields to merge into the outbound request body.
    /// Set by converter before each send attempt.
    #[serde(skip)]
    pub outbound_extra: HashMap<String, JsonValue>,
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IrUsage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_tokens: Option<i64>,
}

/// Unified intermediate representation of an LLM API response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<crate::types::StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    #[serde(default)]
    pub usage: IrUsage,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub provider_extensions: ProviderExtensions,
}

/// Identifies the kind of streaming event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamEventType {
    Start,
    Delta,
    ContentBlockStart,
    ContentBlockStop,
    Stop,
    Error,
}

/// Error details from a streaming error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamError {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

/// A single event in a streaming response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrStreamEvent {
    #[serde(rename = "type")]
    pub event_type: StreamEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<IrResponse>,
    #[serde(default)]
    pub index: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<crate::types::StopReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<IrUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<StreamError>,
}

impl IrRequest {
    pub fn new(model: String, inbound_protocol: Protocol) -> Self {
        Self {
            model,
            inbound_protocol,
            messages: Vec::new(),
            system_prompt: Vec::new(),
            tools: Vec::new(),
            tool_choice: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: Vec::new(),
            stream: None,
            thinking: None,
            response_format: None,
            provider_extensions: HashMap::new(),
            original_model: None,
            raw_extra: HashMap::new(),
            outbound_extra: HashMap::new(),
        }
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub fn inbound_protocol(&self) -> Protocol {
        self.inbound_protocol
    }

    pub fn is_streaming(&self) -> bool {
        self.stream.unwrap_or(false)
    }

    pub fn has_tools(&self) -> bool {
        !self.tools.is_empty()
    }

    pub fn has_media(&self) -> bool {
        fn block_has_media(block: &ContentBlock) -> bool {
            matches!(
                block.content_type,
                crate::types::ContentType::Image | crate::types::ContentType::Document
            )
        }
        fn blocks_have_media(blocks: &[ContentBlock]) -> bool {
            blocks.iter().any(block_has_media)
        }
        blocks_have_media(&self.system_prompt)
            || self.messages.iter().any(|m| blocks_have_media(&m.content))
    }
}
