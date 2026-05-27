use serde::{Deserialize, Serialize};

/// Protocol identifier for routing and codec selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Protocol {
    #[serde(rename = "openai_chat")]
    #[default]
    OpenAiChat,
    #[serde(rename = "openai_responses")]
    OpenAiResponses,
    #[serde(rename = "anthropic")]
    Anthropic,
}

/// Role of a message sender in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// Reason the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    ToolUse,
    StopSequence,
    ContentFilter,
    PauseTurn,
    #[serde(untagged)]
    Other(String),
}

/// The type of a content block within a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    #[default]
    Text,
    Image,
    ToolUse,
    ToolResult,
    ServerToolUse,
    WebSearchToolResult,
    Document,
    Thinking,
    RedactedThinking,
    Refusal,
}

/// A citation attached to a content part.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
}

/// Plain text content.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TextContent {
    pub text: String,
}

/// Image content reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Document content reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// A tool invocation made by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseContent {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// The result of a tool invocation returned to the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultContent {
    pub tool_use_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// A server-side tool invocation (e.g. Anthropic web_search).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUseContent {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

/// A single search result from a web search server tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Result of a web search server tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchToolResultContent {
    pub tool_use_id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub content: Vec<WebSearchResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

/// Extended thinking output from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingContent {
    pub thinking: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Redacted thinking data that must round-trip exactly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactedThinkingContent {
    pub data: String,
}

/// Refusal content from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefusalContent {
    pub refusal: String,
}

/// A single content block within a message — discriminator union.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContentBlock {
    #[serde(rename = "type")]
    pub content_type: ContentType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<TextContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use: Option<ToolUseContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUseContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub web_search_tool_result: Option<WebSearchToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<DocumentContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redacted_thinking: Option<RedactedThinkingContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<RefusalContent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<Citation>,
}
