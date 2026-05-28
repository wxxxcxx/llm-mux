use anthropic_codec::MessagesCodec;
use llm_mux_core::codec::Codec;
use llm_mux_core::types::{ContentType, StopReason};
use openai_chat_codec::ChatCompletionsCodec;

/// 解码测试: OpenAI Chat → IR
#[test]
fn test_chat_decode_basic() {
    let codec = ChatCompletionsCodec;
    let body = r#"{"model":"gpt-4o","messages":[{"role":"user","content":"Hello!"}]}"#;
    let ir = codec.decode_request(body.as_bytes()).unwrap();
    assert_eq!(ir.model, "gpt-4o");
    assert_eq!(ir.messages.len(), 1);
}

/// 编码测试: IR → OpenAI Chat
#[test]
fn test_chat_encode_response() {
    let codec = ChatCompletionsCodec;
    let resp = llm_mux_core::ir::IrResponse {
        id: Some("id".into()), model: Some("gpt-4o".into()),
        content: vec![llm_mux_core::types::ContentBlock {
            content_type: ContentType::Text,
            text: Some(llm_mux_core::types::TextContent { text: "Hi".into() }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn), stop_sequence: None,
        usage: Default::default(), provider_extensions: Default::default(),
    };
    let encoded = codec.encode_response(&resp).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(v["choices"][0]["message"]["content"], "Hi");
}

/// 解码测试: Anthropic → IR
#[test]
fn test_anthropic_decode_basic() {
    let codec = MessagesCodec;
    let body = r#"{"model":"claude","system":"You are helpful.","messages":[{"role":"user","content":"Hello!"}],"max_tokens":1024}"#;
    let ir = codec.decode_request(body.as_bytes()).unwrap();
    assert_eq!(ir.model, "claude");
    assert_eq!(ir.messages.len(), 1);
    assert_eq!(ir.system_prompt.len(), 1);
    assert_eq!(ir.max_tokens, Some(1024));
}

/// 编码测试: IR → Anthropic
#[test]
fn test_anthropic_encode_response() {
    let codec = MessagesCodec;
    let resp = llm_mux_core::ir::IrResponse {
        id: Some("id".into()), model: Some("claude".into()),
        content: vec![llm_mux_core::types::ContentBlock {
            content_type: ContentType::Text,
            text: Some(llm_mux_core::types::TextContent { text: "Hey".into() }),
            ..Default::default()
        }],
        stop_reason: Some(StopReason::EndTurn), stop_sequence: None,
        usage: Default::default(), provider_extensions: Default::default(),
    };
    let encoded = codec.encode_response(&resp).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert!(!v["content"][0]["text"].as_str().unwrap().is_empty());
}

/// 端到端: genai Client + opencode go 真实调用
#[test]
#[cfg(feature = "integration")]
fn test_genai_opencode_go_real_call() {
    let client = genai::Client::default();
    let req = genai::chat::ChatRequest::new(vec![
        genai::chat::ChatMessage::user("say hi")
    ]);
    let rt = tokio::runtime::Runtime::new().unwrap();
    let result = rt.block_on(client.exec_chat("opencode_go::minimax-m2.5", req, None));
    assert!(result.is_ok());
    let text = result.unwrap().first_text().unwrap_or("").to_string();
    assert!(!text.is_empty());
}
