use llm_mux_core::codec::Codec;
use llm_mux_core::ir::StreamEventType;
use llm_mux_core::types::{ContentType, Protocol, Role, StopReason};
use openai_codec::ChatCompletionsCodec;

#[test]
fn test_decode_basic_chat_request() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "system", "content": "You are helpful."},
            {"role": "user", "content": "Hello!"}
        ]
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert_eq!(ir.model, "gpt-4o");
    assert_eq!(ir.inbound_protocol(), Protocol::OpenAiChat);
    assert_eq!(ir.messages.len(), 1);
    assert_eq!(ir.messages[0].role, Role::User);
    assert_eq!(ir.system_prompt.len(), 1);
    assert_eq!(
        ir.system_prompt[0].text.as_ref().unwrap().text,
        "You are helpful."
    );
}

#[test]
fn test_decode_with_tools() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "get_weather",
                    "description": "Get weather",
                    "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}
                }
            }
        ],
        "tool_choice": "auto",
        "stream": true
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert!(ir.is_streaming());
    assert!(ir.has_tools());
    assert_eq!(ir.tools.len(), 1);
    assert_eq!(ir.tools[0].name, "get_weather");
    assert!(ir.tool_choice.is_some());
    assert_eq!(ir.tool_choice.as_ref().unwrap().choice_type, "auto");
}

#[test]
fn test_decode_assistant_with_tool_calls() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Weather in SF?"},
            {
                "role": "assistant",
                "content": null,
                "tool_calls": [
                    {
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"location\":\"San Francisco\"}"
                        }
                    }
                ]
            },
            {
                "role": "tool",
                "tool_call_id": "call_123",
                "content": "72F sunny"
            }
        ]
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert_eq!(ir.messages.len(), 3);
    // user
    assert_eq!(ir.messages[0].role, Role::User);
    // assistant with tool_use
    assert_eq!(ir.messages[1].role, Role::Assistant);
    assert_eq!(ir.messages[1].content.len(), 1);
    assert_eq!(ir.messages[1].content[0].content_type, ContentType::ToolUse);
    let tu = ir.messages[1].content[0].tool_use.as_ref().unwrap();
    assert_eq!(tu.name, "get_weather");
    // tool result
    assert_eq!(ir.messages[2].role, Role::Tool);
    assert_eq!(
        ir.messages[2].content[0].content_type,
        ContentType::ToolResult
    );
}

#[test]
fn test_encode_basic_response() {
    use llm_mux_core::ir::{IrResponse, IrUsage};
    use llm_mux_core::types::TextContent;

    let codec = ChatCompletionsCodec;
    let resp = IrResponse {
        id: Some("chatcmpl-123".into()),
        model: Some("gpt-4o".into()),
        content: vec![llm_mux_core::types::ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent {
                text: "Hello!".into(),
            }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: IrUsage {
            input_tokens: Some(10),
            output_tokens: Some(20),
            total_tokens: Some(30),
            ..Default::default()
        },
        provider_extensions: Default::default(),
    };

    let encoded = codec.encode_response(&resp).unwrap();
    let chat_resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(chat_resp["id"], "chatcmpl-123");
    assert_eq!(chat_resp["choices"][0]["message"]["content"], "Hello!");
    assert_eq!(chat_resp["choices"][0]["finish_reason"], "stop");
    assert_eq!(chat_resp["usage"]["total_tokens"], 30);
}

#[test]
fn test_decode_stream_chunk_text() {
    let codec = ChatCompletionsCodec;
    let data = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;

    let event = codec.decode_stream_event(None, data).unwrap();
    assert_eq!(event.event_type, StreamEventType::Delta);
    assert_eq!(
        event.delta.as_ref().unwrap().text.as_ref().unwrap().text,
        "Hello"
    );
}

#[test]
fn test_decode_stream_chunk_finish() {
    let codec = ChatCompletionsCodec;
    let data = r#"{"id":"chatcmpl-123","object":"chat.completion.chunk","created":0,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#;

    let event = codec.decode_stream_event(None, data).unwrap();
    assert_eq!(event.event_type, StreamEventType::Stop);
    assert_eq!(event.stop_reason, Some(StopReason::EndTurn));
}

#[test]
fn test_encode_stream_delta() {
    use llm_mux_core::ir::IrStreamEvent;
    use llm_mux_core::types::ContentBlock;
    use llm_mux_core::types::TextContent;

    let codec = ChatCompletionsCodec;
    let event = IrStreamEvent {
        event_type: StreamEventType::Delta,
        response: None,
        index: 0,
        delta: Some(ContentBlock {
            content_type: ContentType::Text,
            text: Some(TextContent { text: "Hi".into() }),
            ..Default::default()
        }),
        stop_reason: None,
        usage: None,
        error: None,
    };

    let sse = codec.encode_stream_event(&event).unwrap();
    assert!(sse.starts_with("data: "));
    assert!(sse.contains("\"content\":\"Hi\""));
}

#[test]
fn test_encode_stream_stop() {
    use llm_mux_core::ir::IrStreamEvent;

    let codec = ChatCompletionsCodec;
    let event = IrStreamEvent {
        event_type: StreamEventType::Stop,
        response: None,
        index: 0,
        delta: None,
        stop_reason: Some(StopReason::EndTurn),
        usage: None,
        error: None,
    };

    let sse = codec.encode_stream_event(&event).unwrap();
    assert!(sse.contains("[DONE]"));
    assert!(sse.contains("\"stop\""));
}
