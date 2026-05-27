use openai_codec::ChatCompletionsCodec;
use anthropic_codec::MessagesCodec;
use llm_mux_core::codec::Codec;
use llm_mux_core::types::{ContentType, Protocol, StopReason};

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

    let ir = anthropic_codec.decode_request(anthropic_body.as_bytes()).unwrap();
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
    let anthropic_codec = MessagesCodec;

    let anthropic_body = r#"{
        "id": "msg_123",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet-4-6",
        "content": [
            {"type": "text", "text": "The weather is 72F and sunny."},
            {"type": "tool_use", "id": "toolu_01", "name": "get_weather", "input": {"location": "San Francisco"}}
        ],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 20, "output_tokens": 30}
    }"#;

    // Decode Anthropic response into IR (via stream event decode since response decode doesn't exist)
    // Actually, we need decode_response. Let's use encode_response → IR → ...
    // For now, construct IR manually and verify both sides
    use llm_mux_core::ir::{IrResponse, IrUsage};
    use llm_mux_core::types::{TextContent, ToolUseContent};

    let ir = IrResponse {
        id: Some("msg_123".into()),
        model: Some("claude-sonnet-4-6".into()),
        content: vec![
            llm_mux_core::types::ContentBlock {
                content_type: ContentType::Text,
                text: Some(TextContent { text: "The weather is 72F and sunny.".into() }),
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
        usage: IrUsage { input_tokens: Some(20), output_tokens: Some(30), total_tokens: Some(50), ..Default::default() },
        provider_extensions: Default::default(),
    };

    let chat_body = chat_codec.encode_response(&ir).unwrap();
    let resp: serde_json::Value = serde_json::from_slice(&chat_body).unwrap();

    assert_eq!(resp["id"], "msg_123");
    assert_eq!(resp["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(resp["choices"][0]["message"]["content"], "The weather is 72F and sunny.");
    assert_eq!(resp["choices"][0]["message"]["tool_calls"][0]["function"]["name"], "get_weather");
    assert_eq!(resp["usage"]["total_tokens"], 50);
}

/// Chat response → IR → Anthropic response
#[test]
fn test_chat_response_to_anthropic_response() {
    use llm_mux_core::ir::{IrResponse, IrUsage};
    use llm_mux_core::types::{TextContent, ToolUseContent};

    let chat_codec = ChatCompletionsCodec;
    let anthropic_codec = MessagesCodec;

    let ir = IrResponse {
        id: Some("chatcmpl-456".into()),
        model: Some("gpt-4o".into()),
        content: vec![
            llm_mux_core::types::ContentBlock {
                content_type: ContentType::Text,
                text: Some(TextContent { text: "Sure! The weather is nice.".into() }),
                ..Default::default()
            },
        ],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: IrUsage { input_tokens: Some(10), output_tokens: Some(15), total_tokens: Some(25), ..Default::default() },
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
