use anthropic_codec::MessagesCodec;
use llm_mux_core::codec::Codec;
use llm_mux_core::ir::{IrStreamEvent, IrUsage, StreamEventType};
use llm_mux_core::types::{ContentBlock, ContentType, StopReason, TextContent, ToolUseContent};
use openai_codec::ChatCompletionsCodec;

/// Full roundtrip: OpenAI Chat request → IR → Anthropic Messages request
#[test]
fn test_chat_request_to_anthropic_request() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let chat_body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "What is the weather in SF?"}
        ],
        "temperature": 0.7,
        "max_tokens": 1000,
        "stream": true
    }"#;

    let ir = chat_codec.decode_request(chat_body.as_bytes()).unwrap();
    let anthropic_body = anthropic_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&anthropic_body).unwrap();

    assert_eq!(req["model"], "gpt-4o");
    assert_eq!(req["system"], "You are helpful.");
    assert_eq!(req["max_tokens"], 1000);
    assert!(req["stream"].as_bool().unwrap());
    assert_eq!(req["temperature"], 0.7);
    assert_eq!(req["messages"][0]["role"], "user");
    assert_eq!(
        req["messages"][0]["content"][0]["text"],
        "What is the weather in SF?"
    );
}

/// Roundtrip: Anthropic Messages request → IR → OpenAI Chat request
#[test]
fn test_anthropic_request_to_chat_request() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let anthropic_body = r#"{
        "model": "claude-sonnet-4-6",
        "system": "You are helpful.",
        "messages": [
            {"role": "user", "content": [{"type": "text", "text": "Hello!"}]}
        ],
        "max_tokens": 4096,
        "stream": false
    }"#;

    let ir = anthropic_codec
        .decode_request(anthropic_body.as_bytes())
        .unwrap();
    let chat_body = chat_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();

    assert_eq!(req["model"], "claude-sonnet-4-6");
    assert_eq!(req["messages"][0]["role"], "system");
    assert_eq!(req["messages"][0]["content"], "You are helpful.");
    assert_eq!(req["messages"][1]["role"], "user");
    assert_eq!(req["messages"][1]["content"], "Hello!");
}

/// Chat request with tools → IR → Anthropic request with tools
#[test]
fn test_chat_tools_to_anthropic_tools() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let chat_body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather for a location",
                    "parameters": {"type": "object", "properties": {"location": {"type": "string"}}, "required": ["location"]}
                }
            }
        ],
        "tool_choice": "auto"
    }"#;

    let ir = chat_codec.decode_request(chat_body.as_bytes()).unwrap();
    let anthropic_body = anthropic_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&anthropic_body).unwrap();

    assert_eq!(req["tools"][0]["name"], "get_weather");
    assert_eq!(req["tools"][0]["input_schema"]["type"], "object");
    assert_eq!(req["tool_choice"], "auto");
}

/// Anthropic response → IR → Chat response
#[test]
fn test_anthropic_response_to_chat_response() {
    let chat_codec = ChatCompletionsCodec;

    // Decode Anthropic response into IR (via stream event decode since response decode doesn't exist)
    // Actually, we need decode_response. Let's use encode_response → IR → ...
    // For now, construct IR manually and verify both sides
    use llm_mux_core::ir::IrResponse;

    let ir = IrResponse {
        id: Some("msg_123".into()),
        model: Some("claude-sonnet-4-6".into()),
        content: vec![
            llm_mux_core::types::ContentBlock {
                content_type: ContentType::Text,
                text: Some(TextContent {
                    text: "The weather is 72F and sunny.".into(),
                }),
                ..Default::default()
            },
            llm_mux_core::types::ContentBlock {
                content_type: ContentType::ToolUse,
                tool_use: Some(ToolUseContent {
                    id: "toolu_01".into(),
                    name: "get_weather".into(),
                    arguments: Some(serde_json::json!({"location": "San Francisco"})),
                }),
                ..Default::default()
            },
        ],
        stop_reason: Some(StopReason::ToolUse),
        stop_sequence: None,
        usage: IrUsage {
            input_tokens: Some(20),
            output_tokens: Some(30),
            total_tokens: Some(50),
            ..Default::default()
        },
        provider_extensions: Default::default(),
    };

    let chat_body = chat_codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();

    assert_eq!(resp["id"], "msg_123");
    assert_eq!(resp["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(
        resp["choices"][0]["message"]["content"],
        "The weather is 72F and sunny."
    );
    assert_eq!(
        resp["choices"][0]["message"]["tool_calls"][0]["function"]["name"],
        "get_weather"
    );
    assert_eq!(resp["usage"]["total_tokens"], 50);
}

/// Chat response → IR → Anthropic response
#[test]
fn test_chat_response_to_anthropic_response() {
    use llm_mux_core::ir::IrResponse;

    let anthropic_codec = MessagesCodec;

    let ir = IrResponse {
        id: Some("chatcmpl-456".into()),
        model: Some("gpt-4o".into()),
        content: vec![llm_mux_core::types::ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: "Sure! The weather is nice.".into(),
            }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: IrUsage {
            input_tokens: Some(10),
            output_tokens: Some(15),
            total_tokens: Some(25),
            ..Default::default()
        },
        provider_extensions: Default::default(),
    };

    let anthropic_body = anthropic_codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&anthropic_body).unwrap();

    assert_eq!(resp["id"], "chatcmpl-456");
    assert_eq!(resp["type"], "message");
    assert_eq!(resp["role"], "assistant");
    assert_eq!(resp["content"][0]["text"], "Sure! The weather is nice.");
    assert_eq!(resp["stop_reason"], "end_turn");
}

// ============================================================================
// US1 Cross-protocol tests: T013–T017
// ============================================================================

/// T013: Chat → Anthropic request roundtrip with system_prompt, thinking, images
#[test]
fn test_chat_to_anthropic_rich_request() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let chat_body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are a helpful assistant."},
            {"role": "user", "content": [
                {"type": "text", "text": "Describe this image:"},
                {"type": "image_url", "image_url": {"url": "https://example.com/photo.jpg", "detail": "high"}}
            ]}
        ],
        "tools": [
            {"type": "function", "function": {"name": "search", "description": "Search the web", "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}}}
        ],
        "temperature": 0.5,
        "max_tokens": 2000,
        "stream": false
    }"#;

    let mut ir = chat_codec.decode_request(chat_body.as_bytes()).unwrap();

    // Inject thinking config
    ir.thinking = Some(llm_mux_core::ir::IrThinkingConfig {
        mode: Some("enabled".into()),
        budget_tokens: Some(1024),
        effort: None,
        include_thoughts: None,
        level: None,
    });

    let anthropic_body = anthropic_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&anthropic_body).unwrap();

    // Model and system
    assert_eq!(req["model"], "gpt-4o");
    assert_eq!(req["system"], "You are a helpful assistant.");

    // Messages structure: user message with text + image blocks
    assert_eq!(req["messages"][0]["role"], "user");
    let blocks = &req["messages"][0]["content"];
    assert_eq!(blocks[0]["type"], "text");
    assert_eq!(blocks[0]["text"], "Describe this image:");
    assert_eq!(blocks[1]["type"], "image");
    assert_eq!(blocks[1]["source"]["type"], "url");
    assert_eq!(blocks[1]["source"]["url"], "https://example.com/photo.jpg");

    // Thinking config
    assert_eq!(req["thinking"]["type"], "enabled");
    assert_eq!(req["thinking"]["budget_tokens"], 1024);

    // Tools
    assert_eq!(req["tools"][0]["name"], "search");
    assert_eq!(req["tools"][0]["input_schema"]["type"], "object");

    // Common params
    assert_eq!(req["temperature"], 0.5);
    assert_eq!(req["max_tokens"], 2000);
}

/// T014: Anthropic → Chat request roundtrip with system blocks, thinking, images, tool_result
#[test]
fn test_anthropic_to_chat_rich_request() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let anthropic_body = r#"{
        "model": "claude-sonnet-4-6",
        "system": [
            {"type": "text", "text": "You are a helpful assistant."},
            {"type": "text", "text": "Always be polite and concise."}
        ],
        "thinking": {"type": "enabled", "budget_tokens": 2048},
        "messages": [
            {
                "role": "user",
                "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image", "source": {"type": "url", "url": "https://example.com/img.png", "media_type": "image/png"}}
                ]
            },
            {
                "role": "assistant",
                "content": [
                    {"type": "tool_use", "id": "toolu_abc", "name": "analyze_image", "input": {"url": "https://example.com/img.png"}}
                ]
            }
        ],
        "tools": [
            {"name": "analyze_image", "description": "Analyze an image", "input_schema": {"type": "object", "properties": {"url": {"type": "string"}}}}
        ],
        "max_tokens": 4096,
        "stream": false
    }"#;

    let ir = anthropic_codec
        .decode_request(anthropic_body.as_bytes())
        .unwrap();

    // Verify IR decoding: thinking config preserved
    assert!(ir.thinking.is_some());
    let thinking = ir.thinking.as_ref().unwrap();
    assert_eq!(thinking.mode, Some("enabled".to_string()));
    assert_eq!(thinking.budget_tokens, Some(2048));

    // Verify system_prompt with multiple blocks
    assert_eq!(ir.system_prompt.len(), 2);
    assert_eq!(
        ir.system_prompt[0].text.as_ref().unwrap().text,
        "You are a helpful assistant."
    );
    assert_eq!(
        ir.system_prompt[1].text.as_ref().unwrap().text,
        "Always be polite and concise."
    );

    // Verify messages: user message with text + image
    assert_eq!(ir.messages[0].role, llm_mux_core::types::Role::User);
    assert_eq!(ir.messages[0].content[0].content_type, ContentType::Text);
    assert_eq!(ir.messages[0].content[1].content_type, ContentType::Image);
    assert_eq!(
        ir.messages[0].content[1].image.as_ref().unwrap().url,
        Some("https://example.com/img.png".into())
    );

    // Verify assistant message with tool_use
    assert_eq!(ir.messages[1].role, llm_mux_core::types::Role::Assistant);
    assert_eq!(ir.messages[1].content[0].content_type, ContentType::ToolUse);
    assert_eq!(
        ir.messages[1].content[0].tool_use.as_ref().unwrap().id,
        "toolu_abc"
    );
    assert_eq!(
        ir.messages[1].content[0].tool_use.as_ref().unwrap().name,
        "analyze_image"
    );

    // Verify tools decoded
    assert_eq!(ir.tools.len(), 1);
    assert_eq!(ir.tools[0].name, "analyze_image");

    // Encode to Chat and verify key fields
    let chat_body = chat_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();

    assert_eq!(req["model"], "claude-sonnet-4-6");

    // System prompt → system messages
    assert_eq!(req["messages"][0]["role"], "system");
    assert_eq!(
        req["messages"][0]["content"],
        "You are a helpful assistant."
    );
    assert_eq!(req["messages"][1]["role"], "system");
    assert_eq!(
        req["messages"][1]["content"],
        "Always be polite and concise."
    );

    // User message with text + image → content parts
    assert_eq!(req["messages"][2]["role"], "user");
    let parts = &req["messages"][2]["content"];
    assert_eq!(parts[0]["type"], "text");
    assert_eq!(parts[0]["text"], "What is in this image?");
    assert_eq!(parts[1]["type"], "image_url");
    assert_eq!(parts[1]["image_url"]["url"], "https://example.com/img.png");

    // Assistant with tool_use → tool_calls in assistant message
    assert_eq!(req["messages"][3]["role"], "assistant");
    assert_eq!(
        req["messages"][3]["tool_calls"][0]["function"]["name"],
        "analyze_image"
    );
    assert_eq!(req["messages"][3]["tool_calls"][0]["id"], "toolu_abc");

    // Tools
    assert_eq!(req["tools"][0]["function"]["name"], "analyze_image");

    // Max tokens
    assert_eq!(req["max_tokens"], 4096);
}

/// T015a: Chat stream events → Anthropic stream encoding
#[test]
fn test_chat_stream_to_anthropic_stream() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    // Text delta
    let text_delta_data = r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
    let ir = chat_codec
        .decode_stream_event(None, text_delta_data)
        .unwrap();
    assert_eq!(ir.event_type, llm_mux_core::ir::StreamEventType::Delta);
    let out = anthropic_codec.encode_stream_event(&ir).unwrap();
    assert!(out.starts_with("data: "));
    assert!(out.contains("text_delta"));
    assert!(out.contains("Hello"));

    // Tool use delta
    let tool_delta_data = r#"{"id":"chatcmpl-2","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"search","arguments":"{\"q\":\"weather\"}"}}]},"finish_reason":null}]}"#;
    let ir = chat_codec
        .decode_stream_event(None, tool_delta_data)
        .unwrap();
    assert_eq!(ir.event_type, llm_mux_core::ir::StreamEventType::Delta);
    let out = anthropic_codec.encode_stream_event(&ir).unwrap();
    assert!(out.contains("input_json_delta"));

    // Stop event
    let stop_data = r#"{"id":"chatcmpl-3","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":10,"completion_tokens":5,"total_tokens":15}}"#;
    let ir = chat_codec.decode_stream_event(None, stop_data).unwrap();
    assert_eq!(ir.event_type, llm_mux_core::ir::StreamEventType::Stop);
    assert_eq!(ir.stop_reason, Some(StopReason::EndTurn));
}

/// T015b: Anthropic stream events → Chat stream encoding
#[test]
fn test_anthropic_stream_to_chat_stream() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    // Text delta
    let text_event = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi there"}}"#;
    let ir = anthropic_codec
        .decode_stream_event(None, text_event)
        .unwrap();
    assert_eq!(ir.event_type, llm_mux_core::ir::StreamEventType::Delta);
    let out = chat_codec.encode_stream_event(&ir).unwrap();
    assert!(out.starts_with("data: "));
    assert!(out.contains("Hi there"));

    // Tool use start
    let tool_start = r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_01","name":"get_weather"}}"#;
    let ir = anthropic_codec
        .decode_stream_event(None, tool_start)
        .unwrap();
    assert_eq!(
        ir.event_type,
        llm_mux_core::ir::StreamEventType::ContentBlockStart
    );

    // Tool use delta
    let tool_delta = r#"{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"city\":\"SF\"}"}}"#;
    let ir = anthropic_codec
        .decode_stream_event(None, tool_delta)
        .unwrap();
    let out = chat_codec.encode_stream_event(&ir).unwrap();
    assert!(out.contains("tool_calls"));

    // Stop event
    let stop_event = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}"#;
    let ir = anthropic_codec
        .decode_stream_event(None, stop_event)
        .unwrap();
    assert_eq!(ir.stop_reason, Some(StopReason::EndTurn));

    // message_stop event
    let msg_stop = r#"{"type":"message_stop"}"#;
    let ir = anthropic_codec.decode_stream_event(None, msg_stop).unwrap();
    assert_eq!(ir.event_type, llm_mux_core::ir::StreamEventType::Stop);
}

/// T016a: Chat write_error maps HTTP status codes correctly
#[test]
fn test_chat_write_error() {
    let codec = ChatCompletionsCodec;

    let err_400 = codec.write_error(400, "Bad request");
    let v400: serde_json::Value = serde_json::from_slice(&err_400).unwrap();
    assert_eq!(v400["error"]["message"], "Bad request");
    assert_eq!(v400["error"]["type"], "invalid_request_error");

    let err_401 = codec.write_error(401, "Unauthorized");
    let v401: serde_json::Value = serde_json::from_slice(&err_401).unwrap();
    assert_eq!(v401["error"]["message"], "Unauthorized");

    let err_429 = codec.write_error(429, "Rate limited");
    let v429: serde_json::Value = serde_json::from_slice(&err_429).unwrap();
    assert_eq!(v429["error"]["message"], "Rate limited");

    let err_500 = codec.write_error(500, "Internal error");
    let v500: serde_json::Value = serde_json::from_slice(&err_500).unwrap();
    assert_eq!(v500["error"]["message"], "Internal error");
}

/// T016b: Anthropic write_error maps HTTP status codes correctly
#[test]
fn test_anthropic_write_error() {
    let codec = MessagesCodec;

    let err_400 = codec.write_error(400, "Bad request");
    let v400: serde_json::Value = serde_json::from_slice(&err_400).unwrap();
    assert_eq!(v400["type"], "error");
    assert_eq!(v400["error"]["type"], "invalid_request_error");
    assert_eq!(v400["error"]["message"], "Bad request");

    let err_401 = codec.write_error(401, "Unauthorized");
    let v401: serde_json::Value = serde_json::from_slice(&err_401).unwrap();
    assert_eq!(v401["error"]["type"], "authentication_error");

    let err_429 = codec.write_error(429, "Rate limited");
    let v429: serde_json::Value = serde_json::from_slice(&err_429).unwrap();
    assert_eq!(v429["error"]["type"], "rate_limit_error");

    let err_500 = codec.write_error(500, "Internal error");
    let v500: serde_json::Value = serde_json::from_slice(&err_500).unwrap();
    assert_eq!(v500["error"]["type"], "api_error");
    assert_eq!(v500["error"]["message"], "Internal error");
}

/// T017: Unknown field passthrough test — decode with extra fields, encode back, verify preserved
#[test]
fn test_unknown_field_passthrough() {
    let chat_codec = ChatCompletionsCodec;

    let chat_body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Hello"}
        ],
        "x-custom-id": "trace-12345",
        "x-region": "us-east"
    }"#;

    let ir = chat_codec.decode_request(chat_body.as_bytes()).unwrap();

    // Unknown fields should end up in provider_extensions via serde(flatten)
    assert!(ir.provider_extensions.contains_key("x-custom-id"));
    assert_eq!(ir.provider_extensions["x-custom-id"], "trace-12345");
    assert!(ir.provider_extensions.contains_key("x-region"));
    assert_eq!(ir.provider_extensions["x-region"], "us-east");

    // Encode back to Chat and verify fields are preserved
    let chat_body_out = chat_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&chat_body_out).unwrap();
    assert_eq!(req["x-custom-id"], "trace-12345");
    assert_eq!(req["x-region"], "us-east");
    assert_eq!(req["model"], "gpt-4o");
    assert_eq!(req["messages"][0]["role"], "user");
}

// ============================================================================
// US4 Streaming tests: T048–T049
// ============================================================================

/// T048: End-to-end SSE streaming test — Chat decode → Anthropic encode → simulate stream → Chat encode
#[test]
fn test_e2e_sse_streaming() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    // Step 1: Decode a Chat streaming request
    let chat_body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Tell me a short joke"}
        ],
        "temperature": 0.7,
        "max_tokens": 200,
        "stream": true
    }"#;

    let ir = chat_codec.decode_request(chat_body.as_bytes()).unwrap();
    assert!(ir.is_streaming());

    // Step 2: Encode as Anthropic request
    let anthropic_body = anthropic_codec.encode_request(&ir).unwrap();
    let req: serde_json::Value = serde_json::from_slice(&anthropic_body).unwrap();
    assert_eq!(req["model"], "gpt-4o");
    assert!(req["stream"].as_bool().unwrap());
    assert_eq!(req["max_tokens"], 200);

    // Step 3: Simulate Anthropic stream events by constructing IrStreamEvents
    // Content block start → text
    let content_start = IrStreamEvent {
        event_type: StreamEventType::ContentBlockStart,
        response: None,
        index: 0,
        delta: Some(ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: String::new(),
            }),
            ..Default::default()
        }),
        stop_reason: None,
        usage: None,
        error: None,
    };

    // Encode the content_block_start with Chat codec
    let out = chat_codec.encode_stream_event(&content_start).unwrap();
    assert!(out.starts_with("data: "));
    let parsed: serde_json::Value =
        serde_json::from_str(out.strip_prefix("data: ").unwrap().trim()).unwrap();
    assert_eq!(parsed["choices"][0]["delta"]["role"], "assistant");

    // Text delta
    let text_delta = IrStreamEvent {
        event_type: StreamEventType::Delta,
        response: None,
        index: 0,
        delta: Some(ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: "Why did".into(),
            }),
            ..Default::default()
        }),
        stop_reason: None,
        usage: None,
        error: None,
    };

    let out = chat_codec.encode_stream_event(&text_delta).unwrap();
    assert!(out.contains("Why did"));

    // Another text delta
    let text_delta2 = IrStreamEvent {
        event_type: StreamEventType::Delta,
        response: None,
        index: 0,
        delta: Some(ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: " the chicken".into(),
            }),
            ..Default::default()
        }),
        stop_reason: None,
        usage: None,
        error: None,
    };

    let out = chat_codec.encode_stream_event(&text_delta2).unwrap();
    assert!(out.contains("the chicken"));

    // Stop event with usage
    let stop = IrStreamEvent {
        event_type: StreamEventType::Stop,
        response: None,
        index: 0,
        delta: None,
        stop_reason: Some(StopReason::EndTurn),
        usage: Some(IrUsage {
            input_tokens: Some(10),
            output_tokens: Some(20),
            total_tokens: Some(30),
            ..Default::default()
        }),
        error: None,
    };

    let out = chat_codec.encode_stream_event(&stop).unwrap();
    assert!(out.contains("[DONE]"));
    assert!(out.contains("stop"));
    assert!(out.contains("30"));
}

/// T049: Stream interruption test — error event encoded properly in client protocol format
#[test]
fn test_stream_interruption() {
    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let error_event = IrStreamEvent {
        event_type: StreamEventType::Error,
        response: None,
        index: 0,
        delta: None,
        stop_reason: None,
        usage: None,
        error: Some(llm_mux_core::ir::StreamError {
            error_type: Some("server_error".into()),
            code: Some("500".into()),
            message: Some("Internal server error occurred".into()),
            param: None,
        }),
    };

    // Encode as Chat stream error
    let chat_out = chat_codec.encode_stream_event(&error_event).unwrap();
    assert!(chat_out.contains("error"));
    assert!(chat_out.contains("Internal server error occurred"));
    assert!(chat_out.contains("[DONE]"));

    // Encode as Anthropic stream error
    let anthropic_out = anthropic_codec.encode_stream_event(&error_event).unwrap();
    assert!(anthropic_out.contains("error"));
    assert!(anthropic_out.contains("Internal server error occurred"));
    assert!(anthropic_out.contains("[DONE]"));
    assert!(anthropic_out.contains("api_error"));
}
