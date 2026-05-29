pub mod codec;
pub mod ir;
pub mod types;

pub use codec::{
    Authenticator, Codec, CodecError, ConfigAuthenticator, ConfigurableRouter, Converter,
    FixedRouter, NoopConverter, ProviderConfig, RouteInfo, RouteResult, RouteRule, Router,
};
pub use ir::{
    IrMessage, IrRequest, IrResponse, IrResponseFormat, IrStreamEvent, IrThinkingConfig, IrTool,
    IrToolChoice, IrUsage, ProviderExtensions, StreamError, StreamEventType,
};
pub use types::{
    Citation, ContentBlock, ContentType, DocumentContent, ImageContent, Protocol,
    RedactedThinkingContent, RefusalContent, Role, ServerToolUseContent, StopReason, TextContent,
    ThinkingContent, ToolResultContent, ToolUseContent, WebSearchResult,
    WebSearchToolResultContent,
};

// genai 类型重导出 —— 作为统一 IR。
pub use genai::adapter::AdapterKind;
pub use genai::chat::{
    Binary, CacheControl, ChatMessage, ChatOptions, ChatRequest, ChatResponse, ChatResponseFormat,
    ChatRole, ChatStream, ChatStreamEvent, ChatStreamResponse, ContentPart, CustomPart, JsonSpec,
    MessageContent, MessageOptions, ReasoningEffort, ServiceTier, StopReason as GenaiStopReason,
    StreamChunk, StreamEnd, Tool, ToolCall, ToolChoice, ToolName, ToolResponse, Usage, Verbosity,
    WebSearchConfig,
};
pub use genai::resolver::AuthData;
pub use genai::{ModelIden, ModelName, ServiceTarget};
