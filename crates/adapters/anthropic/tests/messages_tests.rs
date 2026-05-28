use anthropic_codec::MessagesCodec;
use llm_mux_core::codec::Codec;
use llm_mux_core::types::{ContentType, Protocol, Role, StopReason};

#[test]
fn test_decode_basic_messages_request() {
    let codec = MessagesCodec;
    let body = r#"{
        "model": "claude-sonnet-4-6",
        "system": "You are helpful.",
        "messages": [
            {"role": "user", "content": "Hello!"}
        ],
        "max_tokens": 4096
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert_eq!(ir.model, "claude-sonnet-4-6");
    assert_eq!(ir.inbound_protocol(), Protocol::Anthropic);
    assert_eq!(ir.messages.len(), 1);
    assert_eq!(ir.messages[0].role, Role::User);
    assert_eq!(ir.system_prompt.len(), 1);
    assert_eq!(
        ir.system_prompt[0].text.as_ref().unwrap().text,
        "You are helpful."
    );
    assert_eq!(ir.max_tokens, Some(4096));
}

#[test]
fn test_decode_with_tools() {
    let codec = MessagesCodec;
    let body = r#"{
        "model": "claude-sonnet-4-6",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": "What's the weather?"}
        ],
        "tools": [
            {
                "name": "get_weather",
                "description": "Get weather",
                "input_schema": {"type": "object", "properties": {"location": {"type": "string"}}}
            }
        ],
        "tool_choice": "auto"
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert!(ir.has_tools());
    assert_eq!(ir.tools.len(), 1);
    assert_eq!(ir.tools[0].name, "get_weather");
    assert_eq!(ir.tools[0].parameters.as_ref().unwrap()["type"], "object");
    assert!(ir.tool_choice.is_some());
    assert_eq!(ir.tool_choice.as_ref().unwrap().choice_type, "auto");
}

#[test]
fn test_decode_assistant_with_tool_use() {
    let codec = MessagesCodec;
    let body = r#"{
        "model": "claude-sonnet-4-6",
        "max_tokens": 1024,
        "messages": [
            {"role": "user", "content": "Weather in SF?"},
            {
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "Let me check."},
                    {
                        "type": "tool_use",
                        "id": "toolu_01",
                        "name": "get_weather",
                        "input": {"location": "San Francisco"}
                    }
                ]
            },
            {
                "role": "user",
                "content": [
                    {
                        "type": "tool_result",
                        "tool_use_id": "toolu_01",
                        "content": "72F sunny"
                    }
                ]
            }
        ]
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    assert_eq!(ir.messages.len(), 3);
    // user
    assert_eq!(ir.messages[0].role, Role::User);
    // assistant with text + tool_use
    assert_eq!(ir.messages[1].role, Role::Assistant);
    assert_eq!(ir.messages[1].content.len(), 2);
    assert_eq!(
        ir.messages[1].content[0].text.as_ref().unwrap().text,
        "Let me check."
    );
    assert_eq!(ir.messages[1].content[1].content_type, ContentType::ToolUse);
    let tu = ir.messages[1].content[1].tool_use.as_ref().unwrap();
    assert_eq!(tu.name, "get_weather");
    // tool result
    assert_eq!(ir.messages[2].role, Role::User);
    assert_eq!(
        ir.messages[2].content[0].content_type,
        ContentType::ToolResult
    );
}

#[test]
fn test_decode_with_thinking() {
    let codec = MessagesCodec;
    let body = r#"{
        "model": "claude-sonnet-4-6",
        "max_tokens": 4096,
        "messages": [
            {"role": "user", "content": "Think deeply."}
        ],
        "thinking": {
            "type": "enabled",
            "budget_tokens": 1024
        }
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();

    let think = ir.thinking.as_ref().unwrap();
    assert_eq!(think.mode.as_deref(), Some("enabled"));
    assert_eq!(think.budget_tokens, Some(1024));
}

#[test]
fn test_encode_basic_response() {
    use llm_mux_core::ir::{IrResponse, IrUsage};
    use llm_mux_core::types::TextContent;

    let codec = MessagesCodec;
    let resp = IrResponse {
        id: Some("msg_123".into()),
        model: Some("claude-sonnet-4-6".into()),
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
    let msg_resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(msg_resp["id"], "msg_123");
    assert_eq!(msg_resp["type"], "message");
    assert_eq!(msg_resp["role"], "assistant");
    assert_eq!(msg_resp["content"][0]["text"], "Hello!");
    assert_eq!(msg_resp["stop_reason"], "end_turn");
}

#[test]
fn test_encode_response_with_tool_use() {
    use llm_mux_core::ir::{IrResponse, IrUsage};
    use llm_mux_core::types::ToolUseContent;

    let codec = MessagesCodec;
    let resp = IrResponse {
        id: Some("msg_456".into()),
        model: Some("claude-sonnet-4-6".into()),
        content: vec![llm_mux_core::types::ContentBlock {
            content_type: ContentType::ToolUse,
            tool_use: Some(ToolUseContent {
                id: "toolu_01".into(),
                name: "get_weather".into(),
                arguments: Some(serde_json::json!({"location": "SF"})),
            }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::ToolUse),
        stop_sequence: None,
        usage: IrUsage::default(),
        provider_extensions: Default::default(),
    };

    let encoded = codec.encode_response(&resp).unwrap();
    let msg_resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(msg_resp["content"][0]["type"], "tool_use");
    assert_eq!(msg_resp["content"][0]["name"], "get_weather");
    assert_eq!(msg_resp["stop_reason"], "tool_use");
}
