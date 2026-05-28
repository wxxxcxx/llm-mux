use llm_mux_core::codec::Codec;
use llm_mux_core::types::{ContentType, Protocol, Role, StopReason};
use openai_chat_codec::ChatCompletionsCodec;

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
    assert_eq!(ir.system_prompt[0].text.as_ref().unwrap().text, "You are helpful.");
}

#[test]
fn test_decode_with_tools() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [{"role": "user", "content": "What's the weather?"}],
        "tools": [{"type": "function", "function": {"name": "get_weather", "description": "Get weather", "parameters": {"type": "object", "properties": {"location": {"type": "string"}}}}}],
        "tool_choice": "auto",
        "stream": true
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();
    assert!(ir.is_streaming());
    assert!(ir.has_tools());
    assert_eq!(ir.tools.len(), 1);
    assert_eq!(ir.tools[0].name, "get_weather");
}

#[test]
fn test_decode_assistant_with_tool_calls() {
    let codec = ChatCompletionsCodec;
    let body = r#"{
        "model": "gpt-4o",
        "messages": [
            {"role": "user", "content": "Weather in SF?"},
            {"role": "assistant", "content": null, "tool_calls": [{"id": "call_123", "type": "function", "function": {"name": "get_weather", "arguments": "{\"location\":\"San Francisco\"}"}}]},
            {"role": "tool", "tool_call_id": "call_123", "content": "72F sunny"}
        ]
    }"#;

    let ir = codec.decode_request(body.as_bytes()).unwrap();
    assert_eq!(ir.messages.len(), 3);
    assert_eq!(ir.messages[0].role, Role::User);
    assert_eq!(ir.messages[1].role, Role::Assistant);
    assert_eq!(ir.messages[1].content[0].content_type, ContentType::ToolUse);
    assert_eq!(ir.messages[2].role, Role::Tool);
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
            text: Some(TextContent { text: "Hello!".into() }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn),
        stop_sequence: None,
        usage: IrUsage { input_tokens: Some(10), output_tokens: Some(20), total_tokens: Some(30), ..Default::default() },
        provider_extensions: Default::default(),
    };

    let encoded = codec.encode_response(&resp).unwrap();
    let chat_resp: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(chat_resp["choices"][0]["message"]["content"], "Hello!");
    assert_eq!(chat_resp["choices"][0]["finish_reason"], "stop");
}
